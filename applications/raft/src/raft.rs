use std::collections::HashMap;
use log::debug;
use log::warn;

use yulong_network::identity::{Peer, Me};

use yulong::utils::AsBytes;
use yulong::utils::CasualTimer;

use yulong_bdn::overlay::BDN;
use yulong_bdn::message::OverlayMessage;
use yulong_bdn::msg_header::{MsgHeader, MsgTypeKind, RelayMethodKind};
use yulong_bdn::route::AppLayerRouteUser;
use yulong_network::transport::Transport;
use yulong_bdn::route_inner::RelayCtl;

use crate::log_store::LogEntry;
use crate::log_store::LogService;
use crate::log_store::ReplicatedLog;

use crate::message::RaftClientReply;
use crate::message::RaftClientRequest;
use crate::message::{
    RaftAppendEntries,
    RaftAppendEntriesReply,
    RaftRequestVote,
    RaftRequestVoteReply
};

use crate::quorum::QuorumCollector;
use crate::quorum::VoteBox;

use crate::message::{RaftMessage, RaftMessageKind};
use crate::quorum::VoteResult;
use crate::raft_timer::RaftTimer;


#[derive(Debug, PartialEq)]
enum NodeState {
    Follower,
    Leader,
    Candidate,
}


struct PersistentState {
    term: u64,
    voted_for: Option<Peer>,
    log: ReplicatedLog,
}


struct VolatileState {
    commit_idx: u64,
    last_applied: u64,
}


struct VolatileStateServer {
    next_idx: HashMap::<Peer, u64>,
    match_idx: HashMap::<Peer, u64>,
}


pub struct RaftContext<T: Transport, R: RelayCtl> {

    state: NodeState,

    ps: PersistentState,
    vs: VolatileState,
    vss: VolatileStateServer,

    peers: Vec<Peer>,
    leader: Option<Peer>,

    network_handle: BDN<T, R>,
    local_id: Me,
    seq: u32,

    timer: RaftTimer,
    election: VoteBox,
}


impl<T: Transport, R: RelayCtl> RaftContext<T, R> {

    fn raft_msg_dispatch(&self, raft_msg: RaftMessage) {

        match raft_msg.msg() {
            
            RaftMessageKind::RequestVote(msg) => {
                self.request_vote_cb(msg, raft_msg.seq(), raft_msg.sender());
            }

            RaftMessageKind::RequestVoteReply(_) => todo!(),
            RaftMessageKind::AppendEntries(_) => todo!(),
            RaftMessageKind::AppendEntriesReply(_) => todo!(),
            RaftMessageKind::ClientRequest(_) => todo!(),
            RaftMessageKind::ClientReply(_) => todo!(),
        }

    }


    fn request_cb(&mut self, sender: &Peer, msg: &RaftClientRequest, seq: u32) {
        if self.state == NodeState::Leader {
            // serve it
            
            // append entry
            let new_entry = LogEntry::new(self.ps.term, msg.command().to_owned());
            self.ps.log.client_new_entry(new_entry.clone());
        
            // send append_entry to all followers
            // todo 
            self.broadcast_append_entry(vec![new_entry]);

        }
        else {
            // info leader address
            if let Some(leader) = &self.leader {
                let refuse_msg = RaftClientReply::new(leader.to_owned());
                let raft_msg = RaftMessage::new(
                    RaftMessageKind::ClientReply(refuse_msg),
                    self.seq(),
                    self.local_id.peer()
                );

                async_std::task::block_on(
                    self.send_to_direct(raft_msg, sender)
                );
            }
        }
    }


    fn apply_append_entry() {
        todo!()
    }
 

    fn send_append_entry() {
        todo!()
    }
    

    // empty append entry
    fn send_heartbeat(&mut self) {

        let (last_idx, last_log) = self.ps.log.last();

        let heartbeat_msg = RaftMessage::new(
            RaftMessageKind::AppendEntries(RaftAppendEntries::new(
                self.ps.term,
                self.local_id.peer().to_owned(),    // only leader send heartbeat
                last_idx,
                last_log.term(),
                vec![],
                self.vs.commit_idx
            )),
            self.seq(),
            self.local_id.peer()
        );

        async_std::task::block_on(
            self.broadcast(heartbeat_msg)
        );
    }
    

    // receive empty append entry, term is already checked
    fn heartbeat_cb(&mut self, msg: RaftAppendEntries, seq: u32) {
        let reply_msg = RaftMessage::new(
            RaftMessageKind::AppendEntriesReply(RaftAppendEntriesReply::new(
                seq,
                self.ps.term,
                true
            )),
            self.seq(),
            self.local_id.peer()
        );
    }
    

    // follower heartbeat timeout, or election split vote timeout, start an election
    fn election_timeout_cb(&mut self) {

        self.ps.term += 1;
        self.state = NodeState::Candidate;
        
        // begin a new election
        self.timer.start_election_timer();

        // vote for self
        self.vote(&self.local_id.peer().to_owned());
        self.ps.voted_for = Some(self.local_id.peer().to_owned());

        self.send_request_vote();
    }


    fn start_new_election(&mut self) {
        
        // increase term for a new round of election
        self.ps.term += 1;
        self.timer.start_election_timer();

        // clear previous election and vote for self
        self.clear_election();
        self.vote(&self.local_id.peer().to_owned());
        // vote_for is already set in previous round of election

        self.send_request_vote();
    }


    fn send_request_vote(&mut self) {

        let (last_idx, last_log) = self.ps.log.last();

        let request_vote_msg = RaftMessage::new(
            RaftMessageKind::RequestVote(RaftRequestVote::new(
                self.ps.term,
                self.local_id.peer().to_owned(),
                last_idx,
                last_log.term()
            )),
            self.seq(),
            self.local_id.peer()
        );

        // todo: broadcast in parallel
        async_std::task::block_on(
            self.broadcast(request_vote_msg)
        );
    }


    fn append_entry_cb(&mut self, msg: RaftAppendEntries, seq: u32) {

        if msg.term() < self.ps.term {
            // if message's term is smaller than current term, ignore it
            return;
        }

        // refresh timer
        self.timer.start_heartbeat();

        // follow leader commit
        self.vs.commit_idx = msg.leader_commit();

        // if remote term is higher, update term and clear vote_for
        if msg.term() != self.ps.term {
            self.update_term(msg.term(), None);
        }

        // update leader
        self.leader = Some(msg.leader_id().to_owned());

        // become follower
        self.state = NodeState::Follower;

        if msg.is_empty() {
            // empty append_entry is a heartbeat message
            self.heartbeat_cb(msg, seq);
            return;
        }

        // todo: check log and reply
        
    }


    fn broadcast_append_entry(&mut self, entries: Vec<LogEntry>) {

        let (last_idx, last_log) = self.ps.log.last();

        let append_entry_msg = RaftAppendEntries::new(
            self.ps.term,
            self.local_id.peer().to_owned(),
            last_idx,
            last_log.term(),
            entries,
            self.vs.commit_idx
        );

        let raft_msg = RaftMessage::new(
            RaftMessageKind::AppendEntries(append_entry_msg),
            self.seq(),
            self.local_id.peer()
        );

        async_std::task::block_on(
            self.broadcast(raft_msg)
        );
    }


    fn append_entry_reply_cb(&mut self, msg: RaftAppendEntriesReply) {
        todo!()
    }


    fn request_vote_cb(&mut self, msg: &RaftRequestVote, seq: u32, from: &Peer) {
        
        let vote_msg: RaftMessage;

        if msg.term() < self.ps.term {
            // if candidate's term is lower, reject immediately
            vote_msg = RaftMessage::new(
                RaftMessageKind::RequestVoteReply(RaftRequestVoteReply::new(
                    seq,
                    self.ps.term,
                    false
                )),
                self.seq(),
                self.local_id.peer()
            );

            async_std::task::block_on(
                self.send_to_direct(vote_msg, from)
            );

            return;
        }

        // if we receive a request_vote with higher term, update term
        // and become follower
        if msg.term() > self.ps.term {
            self.update_term(msg.last_log_term(), None);
            self.state = NodeState::Follower;
            // reset election timeout
        }
        
        // check log freshness
        let (last_idx, last_log) = self.ps.log.last();
        
        if msg.last_log_term() >= last_log.term() &&
            msg.last_log_index() >= last_idx
        {
            // candidate's log is at least as recent as local, vote

            match &self.ps.voted_for {
                
                Some(cand) => {
                    if cand == from {
                        // already voted this candidate, vote again
                        vote_msg = RaftMessage::new(
                            RaftMessageKind::RequestVoteReply(RaftRequestVoteReply::new(
                                seq,
                                self.ps.term,
                                true
                            )),
                            self.seq(),
                            self.local_id.peer()
                        );
                    }
                    else {
                        // already voted for another candidate, cannot vote
                        vote_msg = RaftMessage::new(
                            RaftMessageKind::RequestVoteReply(RaftRequestVoteReply::new(
                                seq,
                                self.ps.term,
                                false
                            )),
                            self.seq(),
                            self.local_id.peer()
                        );
                    }
                },

                None => {
                    // have not vote yet
                    vote_msg = RaftMessage::new(
                        RaftMessageKind::RequestVoteReply(RaftRequestVoteReply::new(
                            seq,
                            self.ps.term,
                            true
                        )),
                        self.seq(),
                        self.local_id.peer()
                    );

                    self.ps.voted_for = Some(from.to_owned());
                    self.state = NodeState::Follower;
                    // todo: reset election timeout

                },
            }
        }
        else {
            // candidate's log is not fresher, reject
            vote_msg = RaftMessage::new(
                RaftMessageKind::RequestVoteReply(RaftRequestVoteReply::new(
                    seq,
                    self.ps.term,
                    false
                )),
                self.seq(),
                self.local_id.peer()
            );
        }
        
        async_std::task::block_on(
            self.send_to_direct(vote_msg, from)
        );

    }


    fn request_vote_reply_cb(&mut self, msg: RaftRequestVoteReply, from: &Peer) {

        // if remote has higher term, back to follower
        if msg.term() > self.ps.term {

            self.update_term(msg.term(), None);
            
            // some leader has higher term, but we have not received append
            // entries from him yet
            self.leader = None;

            self.state = NodeState::Follower;
            return;
        }
        
        if msg.vote_granted() {
            self.vote(from);
            if self.election.result() == VoteResult::PASS {
                // enough vote, become leader
                self.state = NodeState::Leader;
                
                debug!("Peer {} gathered enough votes and is elected the leader of term {}",
                    self.local_id.peer(), self.ps.term);
            
            }
            // not enough vote, wait for more
        }
        
        // if do not grant vote, wait for other votes or timeout
    }


    fn vote(&mut self, voter: &Peer) {
        self.election.vote(voter, true);
    }


    fn clear_election(&mut self) {
        self.election.reset();
    }

}


// all server behavior
impl<T: Transport, R: RelayCtl> RaftContext<T, R> {
    
    /// If commitIndex > lastApplied: increment lastApplied, apply
    /// log[lastApplied] to state machine
    fn apply(&mut self) {
        if self.vs.commit_idx > self.vs.last_applied {
            self.vs.last_applied += 1;
            // todo: apply log to state machine
            debug!("Apply log {}", self.vs.last_applied);
        }
    }


    /// If RPC request or response contains term T > currentTerm:
    /// set currentTerm = T, convert to follower
    fn update_term(&mut self, new_term: u64, voted_for: Option<Peer>) {
        if new_term > self.ps.term {
            self.ps.term = new_term;
            self.state = NodeState::Follower;
            self.ps.voted_for = voted_for;
        }
    }

}


impl<T: Transport, R: RelayCtl> Iterator for RaftContext<T, R> {
    
    type Item = usize;

    fn next(&mut self) -> std::option::Option<<Self as Iterator>::Item> {
        
        let msg = self.network_handle.next();
        if let Some(msg) = msg {
            let raw_payload = msg.payload();
            // todo

            match RaftMessage::from_bytes(&raw_payload) {
                Ok(pbft_msg) => {
                    self.raft_msg_dispatch(pbft_msg)
                }

                Err(error) => {
                    warn!("RaftContext::next Decode msg error: {}", error);
                }
            }
        }
        todo!()
    }
}


// wrapper for BDN functionality
impl<T: Transport, R: RelayCtl> RaftContext<T, R>{

    async fn broadcast(&mut self, msg: RaftMessage) {
        if let Ok(msg_buf) = msg.into_bytes() {
            let header = MsgHeader::build(
                MsgTypeKind::PAYLOAD_MSG,
                true,
                RelayMethodKind::LOOKUP_TABLE_1,
                1,
                20
            ).unwrap();
    
            let mut overlay_msg = OverlayMessage::new(
                header,
                self.local_id.peer(),
                self.local_id.peer(),
                &Peer::BROADCAST_ID,
                &msg_buf);
    
            self.network_handle.broadcast(&mut overlay_msg).await
        }
        else {
            warn!("RaftContext::broadcast ill-formed msg: {:?}", msg);
        }
        
    }


    async fn send_to_direct(&mut self, msg: RaftMessage, target: &Peer) {
        if let Ok(msg_buf) = msg.into_bytes() {
            let header = MsgHeader::build(
                MsgTypeKind::PAYLOAD_MSG,
                false,
                RelayMethodKind::ALL,
                1,
                20
            ).unwrap();
    
            let mut overlay_msg = OverlayMessage::new(
                header,
                self.local_id.peer(),
                self.local_id.peer(),
                target,
                &msg_buf);
    
            self.network_handle.send_to(target, &mut overlay_msg).await;
        }
        else {
            warn!("RaftMessage::send_to_direct ill-formed msg: {:?}", msg);
        }
    }


    fn seq(&mut self) -> u32 {
        self.seq += 1;
        self.seq
    }

}