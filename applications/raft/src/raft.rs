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

use crate::log_store::LogService;
use crate::log_store::ReplicatedLog;

use crate::message::{
    RaftAppendEntries,
    RaftAppendEntriesReply,
    RaftRequestVote,
    RaftRequestVoteReply
};

use crate::quorum::QuorumCollector;
use crate::quorum::VoteBox;

use crate::message::{RaftMessage, RaftMessageKind};
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

    network_handle: BDN<T, R>,
    local_id: Me,
    seq: u32,

    timer: RaftTimer,
    election: VoteBox,
}


impl<T: Transport, R: RelayCtl> RaftContext<T, R> {

    fn raft_msg_dispatch(&self, msg: RaftMessage) {
        todo!()
    }


    fn request_cb(&mut self) {
        
    }


    fn apply_append_entry() {
        todo!()
    }
 

    fn send_append_entry() {
        todo!()
    }
    

    // empty append entry
    fn send_heartbeat() {
        todo!()
    }
    

    // receive empty append entry
    fn heartbeat_cb(&mut self, msg: RaftAppendEntries) {
        if self.state == NodeState::Follower {
            // refresh timer
            self.timer.start_heartbeat();
        }
    }
    

    // follower heartbeat timeout, or election split vote timeout, start an election
    fn election_timeout_cb(&mut self) {

        self.ps.term += 1;
        self.state = NodeState::Candidate;
        
        // begin a new election
        self.timer.start_election_timer();

        // vote for self
        self.vote(&self.local_id.peer().to_owned());
        
        let (last_idx, last_log) = self.ps.log.last();

        let request_vote_msg = RaftMessage::new(
            RaftMessageKind::RequestVote(RaftRequestVote::new(
                self.ps.term,
                self.local_id.peer().to_owned(),
                last_idx,
                last_log.term()
            )),
            self.seq()
        );

        // todo: broadcast in parallel
        async_std::task::block_on(
            self.broadcast(request_vote_msg)
        );
    }


    fn send_request_vote(&mut self) {
        todo!()
    }


    fn append_entry_cb(&mut self, msg: RaftAppendEntries) {

        if msg.is_empty() {
            // empty append_entry is a heartbeat message
            return self.heartbeat_cb(msg);
        }
        
        match self.state {
            
            NodeState::Follower => {

            },

            NodeState::Leader => {

            },

            NodeState::Candidate => {

            },
        }
    }


    fn append_entry_reply_cb(&mut self, msg: RaftAppendEntriesReply) {
        todo!()
    }


    fn request_vote_cb(&mut self, msg: RaftRequestVote, seq: u32, from: &Peer) {
        
        let vote_msg: RaftMessage;

        if msg.term() < self.ps.term {
            // downvote immediately
            vote_msg = RaftMessage::new(
                RaftMessageKind::RequestVoteReply(RaftRequestVoteReply::new(
                    seq,
                    self.ps.term,
                    false
                )),
                self.seq()
            );
        }
        else {
            // check log freshness
            let (last_idx, last_log) = self.ps.log.last();
            
            if msg.last_log_term() > last_log.term() {
                // remote is more recent, vote
                self.ps.voted_for = Some(from.to_owned());

                vote_msg = RaftMessage::new(
                    RaftMessageKind::RequestVoteReply(RaftRequestVoteReply::new(
                        seq,
                        self.ps.term,
                        true
                    )),
                    self.seq()
                );

            }
            else if msg.last_log_term() == last_log.term() {
                if msg.last_log_index() >= last_idx {
                    // at least as recent as local, vote
                    self.ps.voted_for = Some(from.to_owned());

                    vote_msg = RaftMessage::new(
                        RaftMessageKind::RequestVoteReply(RaftRequestVoteReply::new(
                            seq,
                            self.ps.term,
                            true
                        )),
                        self.seq()
                    );
                }
                else {
                    // local is more recent, downvote
                    vote_msg = RaftMessage::new(
                        RaftMessageKind::RequestVoteReply(RaftRequestVoteReply::new(
                            seq,
                            self.ps.term,
                            false
                        )),
                        self.seq()
                    );
                }
            }
            else {
                // local is more recent, downvote
                vote_msg = RaftMessage::new(
                    RaftMessageKind::RequestVoteReply(RaftRequestVoteReply::new(
                        seq,
                        self.ps.term,
                        false
                    )),
                    self.seq()
                );

            }
        }
        
        async_std::task::block_on(
            self.send_to_direct(vote_msg, from)
        );

    }


    fn request_vote_reply_cb(&mut self, msg: RaftRequestVoteReply) {
        todo!()
    }


    fn vote(&mut self, voter: &Peer) {
        self.election.vote(voter, true);
        self.ps.voted_for = Some(voter.to_owned());
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
    fn update_term(&mut self, msg: &RaftMessage) {
        if let Some(term) = msg.term() {
            if term > self.ps.term {
                self.ps.term = term;
                self.state = NodeState::Follower;
            }
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