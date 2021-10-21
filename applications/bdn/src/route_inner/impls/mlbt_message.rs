use std::convert::TryInto;

use prost_types;
use crate::bdn_message::MlbtMessage;

use yulong::utils::AsBytes;
use yulong::error::{DumbError, SerializeError, DeserializeError};
use yulong_network::identity::Peer;

use num_traits::{FromPrimitive, ToPrimitive};

#[derive(FromPrimitive, ToPrimitive, Clone, Copy)]
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


    pub fn msg_type(&self) -> RelayMsgKind {
        self.msg_type
    }


    pub fn msg_id(&self) -> u64 {
        self.msg_id
    }


    pub fn payload(&self) -> Vec<u8> {
        self.payload.clone()
    }

}

// define payload structures for each msg_type


// join message only need to specify the broadcast tree to join by 
// including its src id
pub struct RelayMsgJoin {
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


impl RelayMsgJoin {
    pub fn src(&self) -> Peer {
        self.src.clone()
    }
}


pub struct RelayMsgLeave {
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


impl RelayMsgLeave {
    pub fn src(&self) -> Peer {
        self.src.clone()
    }
}



pub struct RelayMsgAccept {
    ack: u64,
}


impl AsBytes for RelayMsgAccept {
    fn into_bytes(&self) -> Result<Vec<u8>, SerializeError> {
        Ok(self.ack.to_be_bytes().to_vec())
    }

    fn from_bytes(buf: &[u8]) -> Result<Self, DeserializeError> {
        if buf.len() == 8 {
            Ok(Self {ack: u64::from_be_bytes(buf[0..8].try_into().unwrap())})
        }
        else {
            return Err(DeserializeError::new("RelayMsgAccept::from_bytes wrong size", DumbError));
        }
    }
}


impl RelayMsgAccept {
    
    // reply with msg_id to indicate accept which message
    pub fn new(income: &RelayCtlMessage) -> Self {
        Self{ack: income.msg_id}
    }
}



pub struct RelayMsgReject {
    ack: u64,
}

impl AsBytes for RelayMsgReject {
    fn into_bytes(&self) -> Result<Vec<u8>, SerializeError> {
        Ok(self.ack.to_be_bytes().to_vec())
    }

    fn from_bytes(buf: &[u8]) -> Result<Self, DeserializeError> {
        if buf.len() == 8 {
            Ok(Self {ack: u64::from_be_bytes(buf[0..8].try_into().unwrap())})
        }
        else {
            return Err(DeserializeError::new("RelayMsgReject::from_bytes wrong size", DumbError));
        }
    }
}


impl RelayMsgReject {

    // reply with msg_id to indicate reject which message
    pub fn new(income: &RelayCtlMessage) -> Self {
        Self{ack: income.msg_id}
    }
}