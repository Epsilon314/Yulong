use prost_types;
use crate::bdn_message::MlbtMessage;

use yulong::utils::AsBytes;
use yulong::error::{DumbError, SerializeError, DeserializeError};

use num_traits::{FromPrimitive, ToPrimitive};

#[derive(FromPrimitive, ToPrimitive)]
pub enum RelayMsgKind {
    JOIN = 0,
    LEAVE = 1,
    ACCEPT = 2,
    REJECT = 3,
}

pub struct RelayCtlMessage {
    msg_type: u32,
    msg_id: u64,
    payload: Vec<u8>,
}

impl RelayCtlMessage {
    pub fn new(msg_type: u32, msg_id: u64, payload: Vec<u8>) -> Self {
        Self {msg_type, msg_id, payload}
    }
}


impl AsBytes for RelayCtlMessage {
    fn into_bytes(&self) -> Result<Vec<u8>, SerializeError> {
        todo!()
    }

    fn from_bytes(buf: &[u8]) -> Result<Self, DeserializeError> {
        todo!()
    }
}

