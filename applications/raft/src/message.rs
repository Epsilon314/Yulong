use yulong::utils::AsBytes;
use yulong::error::{SerializeError, DeserializeError};

pub struct RaftMessage {
    msg: RaftMessageKind,
}


pub enum RaftMessageKind {
    RequestVote(RaftRequestVote),
    RequestVoteReply(RaftRequestVoteReply),
}


pub struct RaftRequestVote {

}


impl RaftRequestVote {}


pub struct RaftRequestVoteReply {

}


impl RaftRequestVoteReply {}


impl AsBytes for RaftMessage {


    fn into_bytes(&self) -> Result<Vec<u8>, SerializeError> {
        todo!()
    }


    fn from_bytes(buf: &[u8]) -> Result<Self, DeserializeError> {
        todo!()
    }

}


impl RaftMessage {

    fn new(msg: RaftMessageKind) -> Self {
        Self {
            msg
        }
    }

}