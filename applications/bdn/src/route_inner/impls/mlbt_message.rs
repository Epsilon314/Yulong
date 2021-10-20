use prost_types;
use crate::bdn_message::MlbtMessage;

use yulong::utils::AsBytes;
use yulong::error::{DumbError, SerializeError, DeserializeError};
use yulong_network::identity::Peer;

use num_traits::{FromPrimitive, ToPrimitive};

#[derive(FromPrimitive, ToPrimitive)]
pub enum RelayMsgKind {
    JOIN = 0,
    LEAVE = 1,
    ACCEPT = 2,
    REJECT = 3,
}

pub struct RelayCtlMessage {
    msg_type: RelayMsgKind,
    msg_id: u64,
    payload: Vec<u8>,
}

impl AsBytes for RelayCtlMessage {
    fn into_bytes(&self) -> Result<Vec<u8>, SerializeError> {
        todo!()
    }

    fn from_bytes(buf: &[u8]) -> Result<Self, DeserializeError> {
        todo!()
    }
}

impl RelayCtlMessage {
    
    pub fn new<T: AsBytes>(msg_type: RelayMsgKind, msg_id: u64, payload: T) -> Self {
        Self {
            msg_type,
            msg_id,
            payload: payload.into_bytes().unwrap()
        }
    }
}

// define payload structures for each msg_type


// join message only need to specify the broadcast tree to join by 
// including its src id
struct RelayMsgJoin {
    src: Peer
}

impl AsBytes for RelayMsgJoin {
    fn into_bytes(&self) -> Result<Vec<u8>, SerializeError> {
        Ok(self.src.get_id().to_vec())
    }

    fn from_bytes(buf: &[u8]) -> Result<Self, DeserializeError> {
        Peer::try_from_id(buf)
            .map(|peer| Self{src: peer})
            .map_err(|error| DeserializeError::new(
                "RelayMsgJoin::from_bytes failed to parse src peer id", error))
    }
}


struct RelayMsgLeave {
    src: Peer
}


impl AsBytes for RelayMsgLeave {
    fn into_bytes(&self) -> Result<Vec<u8>, SerializeError> {
        Ok(self.src.get_id().to_vec())
    }

    fn from_bytes(buf: &[u8]) -> Result<Self, DeserializeError> {
        Peer::try_from_id(buf)
            .map(|peer| Self{src: peer})
            .map_err(|error| DeserializeError::new(
                "RelayMsgJoin::from_bytes failed to parse src peer id", error
            ))
    }
}


// accept is an empty message
struct RelayMsgAccept {}

impl AsBytes for RelayMsgAccept {
    fn into_bytes(&self) -> Result<Vec<u8>, SerializeError> {
        Ok(vec![])
    }

    fn from_bytes(_: &[u8]) -> Result<Self, DeserializeError> {
        Ok(Self{})
    }
}


// reject is an empty message
struct RelayMsgReject {}

impl AsBytes for RelayMsgReject {
    fn into_bytes(&self) -> Result<Vec<u8>, SerializeError> {
        Ok(vec![])
    }

    fn from_bytes(_: &[u8]) -> Result<Self, DeserializeError> {
        Ok(Self{})
    }
}