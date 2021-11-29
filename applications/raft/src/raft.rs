use std::collections::HashMap;
use log::warn;

use yulong_network::identity::{Peer, Me};

use yulong::utils::AsBytes;
use yulong::utils::CasualTimer;

use yulong_bdn::overlay::BDN;
use yulong_bdn::message::OverlayMessage;
use yulong_bdn::msg_header::{MsgHeader, MsgTypeKind, RelayMethodKind};
use yulong_bdn::route_inner::impls::mlbt::MlbtRelayCtlContext;
use yulong_bdn::route::AppLayerRouteUser;

use yulong_tcp::TcpContext;

use crate::log_store::ReplicatedLog;
use crate::message::RaftMessage;
use crate::raft_timer::RaftTimer;


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


pub struct RaftContext {
    ps: PersistentState,
    vs: VolatileState,
    vss: VolatileStateServer,

    network_handle: BDN<TcpContext, MlbtRelayCtlContext>,

    local_id: Me,

    seq: u32,
}


impl RaftContext {

    fn raft_msg_dispatch(&self, msg: RaftMessage) {
        todo!()
    }

    fn request_cb(&mut self, msg: RaftMessage) {

    }

    fn apply_append_entry() {}
 
    fn send_append_entry() {}
    
    // empty append entry
    fn send_heartbeat() {}
    
    // receive empty append entry
    fn heartbeat_cb() {}
    
    // follower heartbeat timeout, start an election
    fn election_timeout_cb() {}

    fn send_request_vote() {}

    fn request_vote_cb() {}

}


impl Iterator for RaftContext {
    
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