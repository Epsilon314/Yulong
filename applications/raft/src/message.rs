use yulong::utils::AsBytes;
use yulong::error::{SerializeError, DeserializeError};
use yulong_network::identity::Peer;

use crate::log_store::LogEntry;

#[derive(Debug)]
pub struct RaftMessage {
    seq: u32,
    sender: Peer,
    msg: RaftMessageKind,
}


#[derive(Debug)]
pub enum RaftMessageKind {
    RequestVote(RaftRequestVote),
    RequestVoteReply(RaftRequestVoteReply),
    AppendEntries(RaftAppendEntries),
    AppendEntriesReply(RaftAppendEntriesReply),
    ClientRequest(RaftClientRequest),
    ClientReply(RaftClientReply),
}


#[derive(Debug)]
pub struct RaftRequestVote {
    term: u64,
    candidate_id: Peer,
    last_log_index: u64,
    last_log_term: u64,
}


impl RaftRequestVote {

    pub fn new(term: u64, candidate: Peer,
        last_log_index: u64, last_log_term: u64) -> Self 
    {
        Self {
            term,
            candidate_id: candidate.to_owned(),
            last_log_index,
            last_log_term
        }
    }


    /// Get a reference to the raft request vote's term.
    pub fn term(&self) -> u64 {
        self.term
    }

    /// Get a reference to the raft request vote's candidate id.
    pub fn candidate_id(&self) -> &Peer {
        &self.candidate_id
    }

    /// Get a reference to the raft request vote's last log index.
    pub fn last_log_index(&self) -> u64 {
        self.last_log_index
    }

    /// Get a reference to the raft request vote's last log term.
    pub fn last_log_term(&self) -> u64 {
        self.last_log_term
    }
}


#[derive(Debug)]
pub struct RaftRequestVoteReply {
    ack: u32,
    term: u64,
    vote_granted: bool,
}


impl RaftRequestVoteReply {

    pub fn new(ack: u32, term: u64, vote_granted: bool) -> Self {
        Self {
            ack,
            term,
            vote_granted
        }
    }


    /// Get a reference to the raft request vote reply's ack.
    pub fn ack(&self) -> u32 {
        self.ack
    }

    /// Get a reference to the raft request vote reply's term.
    pub fn term(&self) -> u64 {
        self.term
    }

    /// Get a reference to the raft request vote reply's vote granted.
    pub fn vote_granted(&self) -> bool {
        self.vote_granted
    }
}


#[derive(Debug)]
pub struct RaftAppendEntries {
    term: u64,
    leader_id: Peer,

    prev_log_idx: u64,
    prev_log_term: u64,

    prev_log_term_entries: Vec<LogEntry>,

    leader_commit: u64
}

impl RaftAppendEntries {
    
    pub fn new(term: u64, leader_id: Peer, prev_log_idx: u64,
        prev_log_term: u64, prev_log_term_entries: Vec<LogEntry>,
        leader_commit: u64) -> Self 
    { 
        Self {
            term, leader_id, prev_log_idx, prev_log_term,
            prev_log_term_entries, leader_commit
        } 
    }



    /// Get a reference to the raft append entries's term.
    pub fn term(&self) -> u64 {
        self.term
    }

    /// Get a reference to the raft append entries's leader id.
    pub fn leader_id(&self) -> &Peer {
        &self.leader_id
    }

    /// Get a reference to the raft append entries's prev log idx.
    pub fn prev_log_idx(&self) -> u64 {
        self.prev_log_idx
    }



    /// Get a reference to the raft append entries's leader commit.
    pub fn leader_commit(&self) -> u64 {
        self.leader_commit
    }

    pub fn is_empty(&self) -> bool {
        self.prev_log_term_entries.is_empty()
    }


    /// Get a reference to the raft append entries's prev log term.
    pub fn prev_log_term(&self) -> u64 {
        self.prev_log_term
    }

    /// Get a reference to the raft append entries's prev log term entries.
    pub fn prev_log_term_entries(&self) -> &[LogEntry] {
        self.prev_log_term_entries.as_ref()
    }
}


#[derive(Debug)]
pub struct RaftAppendEntriesReply {
    ack: u32,
    term: u64,
    success: bool,
}


impl RaftAppendEntriesReply {
    pub fn new(ack: u32, term: u64, success: bool) -> Self { Self { ack, term, success } }



    /// Get a reference to the raft append entries reply's ack.
    pub fn ack(&self) -> u32 {
        self.ack
    }

    /// Get a reference to the raft append entries reply's term.
    pub fn term(&self) -> u64 {
        self.term
    }

    /// Get a reference to the raft append entries reply's success.
    pub fn success(&self) -> bool {
        self.success
    }
}


#[derive(Debug)]
pub struct RaftClientRequest {
    command: Vec<u8>,
}


impl AsBytes for RaftClientRequest {
    fn into_bytes(&self) -> Result<Vec<u8>, SerializeError> {
        todo!()
    }

    fn from_bytes(buf: &[u8]) -> Result<Self, DeserializeError> {
        todo!()
    }
}


impl RaftClientRequest {
    pub fn new(command: Vec<u8>) -> Self { Self { command } }



    /// Get a reference to the client request's command.
    pub fn command(&self) -> &[u8] {
        self.command.as_ref()
    }
}


#[derive(Debug)]
pub struct RaftClientReply {
    leader_id: Peer,
}


impl AsBytes for RaftClientReply {
    fn into_bytes(&self) -> Result<Vec<u8>, SerializeError> {
        todo!()
    }

    fn from_bytes(buf: &[u8]) -> Result<Self, DeserializeError> {
        todo!()
    }
}


impl RaftClientReply {
    pub fn new(leader_id: Peer) -> Self { Self { leader_id } }



    /// Get a reference to the raft client reply's leader id.
    pub fn leader_id(&self) -> &Peer {
        &self.leader_id
    }
}

impl AsBytes for RaftMessage {


    fn into_bytes(&self) -> Result<Vec<u8>, SerializeError> {
        todo!()
    }


    fn from_bytes(buf: &[u8]) -> Result<Self, DeserializeError> {
        todo!()
    }

}


impl RaftMessage {

    pub fn new(msg: RaftMessageKind, seq: u32, sender: &Peer) -> Self {
        Self {
            seq,
            msg,
            sender: sender.to_owned()
        }
    }


    /// Get a reference to the raft message's seq.
    pub fn seq(&self) -> u32 {
        self.seq
    }

    /// Get a reference to the raft message's msg.
    pub fn msg(&self) -> &RaftMessageKind {
        &self.msg
    }


    // for now this option is meaningless, for future compatibility only
    pub fn term(&self) -> Option<u64> {
        match &self.msg {
            RaftMessageKind::RequestVote(m) => Some(m.term()),
            RaftMessageKind::RequestVoteReply(m) => Some(m.term()),
            RaftMessageKind::AppendEntries(m) => Some(m.term()),
            RaftMessageKind::AppendEntriesReply(m) => Some(m.term()),
            RaftMessageKind::ClientRequest(_) => todo!(),
            RaftMessageKind::ClientReply(_) => todo!(),
        }
    }

    /// Get a reference to the raft message's sender.
    pub fn sender(&self) -> &Peer {
        &self.sender
    }
}