use std::{convert::TryInto, mem::size_of};

use yulong_network::{
    identity::crypto::AsBytes,
    error::DeserializeError,
    error::BadMessageError,
    error::DumbError,
    transport::Transport,
};

use crate::bdn_message::BdnMessage;
use async_std::io::BufReader;
use futures::{AsyncReadExt};

pub const MSG_MAXLEN: usize = 2048; //bytes

// message type: bdn control msg or payloads

#[derive(Debug)]
pub struct Message {
    payload: Vec<u8>,
}

impl Message {
 
    pub fn new(raw: &[u8]) -> Result<Message, BadMessageError> {
        if raw.len() > MSG_MAXLEN - 2 {
            Err(BadMessageError::new(
                "Length is larger than MSG_MAXLEN", 
                DumbError))
        }
        else {
            Ok(Self{payload: raw.to_vec(),})
        }
    }

    pub fn get_payload(&self) -> Vec<u8> {
        self.payload.clone()
    }

}

impl AsBytes for Message {

    fn into_bytes(&self) -> Vec<u8> {
        let mut ret = Vec::new();
        let len_bytes = self.payload.len().to_be_bytes();

        // Layout of Message is:
        // 2 bytes length (Big Endian) + payload, no more than MSG_MAXLEN in total.

        // The length of payload has been checked at the time it is created
        // and payload is private, therefore we do not need to check it again here.
        ret.extend(len_bytes[size_of::<usize>()-2..size_of::<usize>()].iter());
        ret.extend(self.payload.iter());
        ret
    }

    /// Do not call this, use MessageReader instead
    fn from_bytes(buf: &[u8]) -> Result<Self, DeserializeError> {
        let len = u16::from_be_bytes(buf[0..2].try_into().unwrap()) as usize;
        
        if len > MSG_MAXLEN - 2 {
            Err(DeserializeError::new(
                "Length is larger than MSG_MAXLEN",
                DumbError))
        }
        else {
            Ok(Self{payload: buf[2..len + 2].to_vec()})
        }
    }
}


pub struct MessageReader<T: Transport>  {
    inner: BufReader<<T as Transport>::Stream>,
}


impl<T: Transport> MessageReader<T> {
    pub fn new(inner_reader: BufReader<<T as Transport>::Stream>) -> Self {
        Self {inner: inner_reader}
    }

    pub async fn read_message(&mut self) -> Result<Option<Message>, DeserializeError> {

        // first read the length 
        let mut len_buf = [0_u8; 2];
        let read_result = self.inner.read_exact(&mut len_buf).await;

        if read_result.is_err() {
            // reach EOF, no more message
            return Ok(None);
        }

        let len = u16::from_be_bytes(len_buf) as usize;
        let mut payload_buf = vec![0_u8; len];
        let read_result = self.inner.read_exact(&mut payload_buf).await;

        if read_result.is_err() {
            return Err(DeserializeError::new(
                "read payload failed", read_result.unwrap_err()
            ));
        }
        Ok(Some(Message::new(&payload_buf).unwrap()))
    }
}


#[cfg(test)]
mod test {
    
    use super::*;

    #[test]
    fn message_serde() {
        let payload = [42_u8; 258];
        let msg = Message::new(&payload).unwrap();
        let raw = msg.into_bytes();
        // println!("raw message: {:?}", &raw);

        let rec_msg = Message::from_bytes(&raw).unwrap();
        assert_eq!(msg.payload, rec_msg.payload);
    }

}