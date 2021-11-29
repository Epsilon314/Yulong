use yulong_network::identity::Peer;
use crate::log_store::ReplicatedLog;
use std::collections::HashMap;

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
}


impl RaftContext {
 
    fn send_append_entry() {}
    
    // empty append entry
    fn send_heartbeat() {}
    
    fn heartbeat_cb() {}
    
    // follower heartbeat timeout, start an election
    fn election_timeout_cb() {}

    fn send_request_vote() {}

    fn request_vote_cb() {}


}