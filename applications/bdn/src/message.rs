use std::fmt::Display;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::{convert::TryInto, mem::size_of};

use log::warn;

use yulong::utils::AsBytes;

use yulong::error::{
    DeserializeError,
    SerializeError,
    DumbError,
};

use crate::error::BadFieldError;

use yulong_network::{
    identity::Peer,
    transport::Transport,
};

use prost::Message;
use crate::bdn_message::BdnMessage;
use async_std::io::BufReader;
use futures::{AsyncReadExt};

use crate::configs::MSG_MAXLEN;
use crate::msg_header;

// message type: bdn control msg or payloads
#[derive(Debug, Clone)]
pub struct OverlayMessage {

    header: u32,
    timestamp: u64,

    src_id: Peer,
    from_id: Peer,
    dst_id: Peer,

    payload: Vec<u8>,
}

impl OverlayMessage {

    pub fn new(
        header: u32,
        src_id: &Peer, 
        from_id: &Peer,
        dst_id: &Peer,
        payload: &[u8]
    ) -> Self {
        Self {
            header,
            // timestamp is filled just be send
            timestamp: 0,
            src_id: src_id.to_owned(),
            from_id: from_id.to_owned(),
            dst_id: dst_id.to_owned(),
            payload: payload.to_vec(),
        }
    }

    // getters and setters

    pub fn payload(&self) -> Vec<u8> {
        self.payload.clone()
    }


    pub fn timestamp(&self) -> Duration {
        Duration::from_nanos(self.timestamp)
    }


    pub fn set_timestamp_now(&mut self) -> Option<()> {
        match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(duration) => {
                self.timestamp = duration.as_nanos() as u64;
                Some(())
            }
            Err(err) => {
                warn!("OverlayMessage::set_timestamp_now Cannot get system clock {}", err);
                None
            }
        }
    }


    pub fn src(&self) -> Peer {
        self.src_id.clone()
    }


    pub fn set_src(&mut self, src: &Peer) {
        self.src_id = src.to_owned()
    }


    pub fn from(&self) -> Peer {
        self.from_id.clone()
    }


    pub fn set_from(&mut self, from: &Peer) {
        self.from_id = from.to_owned()
    }


    pub fn dst(&self) -> Peer {
        self.dst_id.clone()
    }


    pub fn set_dst(&mut self, dst: &Peer) {
        self.dst_id = dst.to_owned()
    }

    
    // deal with message type bitmap

    pub fn get_type(&self) -> Result<msg_header::MsgTypeKind, BadFieldError> {
        match msg_header::MsgType::get_msg_type(self.header) {
            Some(kind) => Ok(kind),
            None => Err(BadFieldError::new(
                format!("UnKnown msg_type field in {}", &self),
                DumbError
            )),
        }
    }

    
    pub fn set_type(&mut self, msg_type: msg_header::MsgTypeKind) {
        
        // Since msg_type is of MsgTypeKind, set_msg_type should always work
        // unless there exists some coding bug. Therefore we do not throw an error
        // but let it panic.
        msg_header::MsgType::set_msg_type(&mut self.header, msg_type).unwrap();
    }


    pub fn get_relay_method(&self) -> Result<msg_header::RelayMethodKind, BadFieldError> {
        match msg_header::RelayMethod::get_relay_method(self.header) {
            Some(kind) => Ok(kind),
            None => Err(BadFieldError::new(
                format!("UnKnown relay_method field in {}", &self),
                DumbError
            )),
        }
    }


    pub fn set_relay_method(&mut self, relay_method: msg_header::RelayMethodKind) {

        // Since relay_method is of RelayMethodKind, set_relay_method should always work
        // unless there exists some coding bug. Therefore we do not throw an error
        // but let it panic.
        msg_header::RelayMethod::set_relay_method(
            &mut self.header, relay_method).unwrap();
    }

    
    pub fn is_relay(&self) -> bool {
        msg_header::RelayFlag::get_relay_flag(self.header)
    }

    
    pub fn set_relay(&mut self, flag: bool) {
        msg_header::RelayFlag::set_relay_flag(&mut self.header, flag);
    }

    
    pub fn get_fanout(&self) -> u32 {
        msg_header::FanOut::get_fan_out(self.header)
    }


    pub fn set_fanout(&mut self, fanout: u32) -> Result<(), BadFieldError>{
        if msg_header::FanOut::set_fan_out(&mut self.header, fanout).is_none() {
            warn!("OverlayMessage::set_fanout fan out value overflow: {}", fanout);
            return Err(BadFieldError::new(
                "OverlayMessage::set_fanout parameter fanout is too large.",
                DumbError));
        }
        Ok(())
    }


    pub fn get_ttl(&self) -> u32 {
        msg_header::TTL::get_ttl(self.header)
    }


    pub fn set_ttl(&mut self, ttl: u32) -> Result<(), BadFieldError> {
        if msg_header::TTL::set_ttl(&mut self.header, ttl).is_none() {
            warn!("OverlayMessage::set_ttl ttl value overflow: {}", ttl);
            return Err(BadFieldError::new(
                "OverlayMessage::set_ttl parameter ttl is too large.",
                DumbError));
        }
        Ok(())
    }

    
    /// derserialize 
    fn der_protobf_payload(buf: &[u8]) -> Result<Self, DeserializeError> {
        match BdnMessage::decode(buf) {
                
            Ok(m) => {
                let src_peer = Peer::try_from_id(&m.src_id);
                let from_peer = Peer::try_from_id(&m.from_id);
                let dst_peer = Peer::try_from_id(&m.dst_id);

                if src_peer.is_err() {
                    return Err(DeserializeError::new(
                        "Decode src id error", src_peer.unwrap_err()));
                }

                if from_peer.is_err() {
                    return Err(DeserializeError::new(
                        "Decode from id error", from_peer.unwrap_err()));
                }
                
                if dst_peer.is_err() {
                    return Err(DeserializeError::new(
                        "Decode dst id error", dst_peer.unwrap_err()));
                }
                
                Ok(Self {
                    header: m.header,
                    timestamp: m.timestamp,
                    src_id: src_peer.unwrap(),
                    from_id: from_peer.unwrap(),
                    dst_id: dst_peer.unwrap(),
                    payload: m.payload,
                })
            }

            Err(error) => {
                Err(DeserializeError::new("Decode error", error))
            }
        }
    }
}

impl AsBytes for OverlayMessage {

    fn into_bytes(&self) -> Result<Vec<u8>, SerializeError> {

        // Layout of OverlayMessage is:
        // 2 bytes length (Big Endian) + payload, no more than MSG_MAXLEN in total.
        // payload is de/serialized using protocol-buf functionality

        // If the payload is too long, a SerializeError is thrown.
        
        let mut ret = Vec::new();

        // protocol-buf message struct generated by prost
        let protobuf_msg = BdnMessage {
            header: self.header,
            timestamp: self.timestamp,
            src_id: self.src_id.get_id().to_vec(),
            from_id: self.from_id.get_id().to_vec(),
            dst_id: self.dst_id.get_id().to_vec(),
            payload: self.payload.clone(),
        };

        // check payload length
        let protobuf_bytes_len = protobuf_msg.encoded_len();

        // length in bytes
        let len_bytes = protobuf_bytes_len.to_be_bytes();

        if protobuf_bytes_len > MSG_MAXLEN - 2 {
            Err(SerializeError::new(
                "Length is larger than MSG_MAXLEN", 
                DumbError))
        }
        else {

            // serialized payload using protocol-buf
            let mut protobuf_bytes = Vec::with_capacity(protobuf_bytes_len);
            protobuf_msg.encode(&mut protobuf_bytes).unwrap();

            // set first 2 bytes to the lowest two bytes of length bytes
            // therefore MSG_MAXLEN is at most 65536 bytes
            ret.extend(len_bytes[size_of::<usize>()-2..size_of::<usize>()].iter());
            ret.extend(protobuf_bytes.iter());
            Ok(ret)
        }
    }

    // Do not use this, only for test & fulfil trait bound, use MessageReader instead
    fn from_bytes(buf: &[u8]) -> Result<Self, DeserializeError> {

        // first 2 bytes is payload length
        let len = u16::from_be_bytes(buf[0..2].try_into().unwrap()) as usize;
        
        if len > MSG_MAXLEN - 2 {
            Err(DeserializeError::new(
                "Length is larger than MSG_MAXLEN",
                DumbError))
        }
        else {
            OverlayMessage::der_protobf_payload(&buf[2..len + 2])
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

    pub async fn read_message(&mut self) -> Result<Option<OverlayMessage>, DeserializeError> {

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

        OverlayMessage::der_protobf_payload(&payload_buf).map(|m| Some(m))
    }
}


impl Display for OverlayMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Overlay Message\n    header: {:032b}\n    src {}\
        \n    from {}\n    dst {}\n    payload: {:02x?}\n", self.header,
        self.src_id, self.from_id, self.dst_id, self.payload)
    }
}


#[cfg(test)]
mod test {
    
    use super::*;
    use crate::msg_header::{MsgTypeKind, RelayMethodKind};
    use log::debug;
    use yulong::log::setup_logger;

    #[test]
    fn message_serde() {
        setup_logger("message").unwrap();
        let payload = [42_u8; 258];

        let peer1 = Peer::from_bytes(&[1]);
        let peer2 = Peer::from_bytes(&[1]);
        let peer3 = Peer::from_bytes(&[1]);

        let msg = OverlayMessage::new(
            1,
            &peer1, 
            &peer2, 
            &peer3, 
            &payload
        );

        let raw = msg.into_bytes().unwrap();
        debug!("raw message: {:?}", &raw);

        let rec_msg = OverlayMessage::from_bytes(&raw).unwrap();
        assert_eq!(msg.payload, rec_msg.payload);
    }

    #[test]
    fn msg_header1() {
        setup_logger("message").unwrap();
        let payload = [42_u8; 10];
        let peer = Peer::from_bytes(&[1]);

        let mut msg = OverlayMessage::new(
            0b00100000000000000000000000000000,
            &peer,
            &peer,
            &peer,
            &payload
        );

        msg.set_timestamp_now();

        msg.set_relay(false);
        msg.set_relay_method(RelayMethodKind::LOOKUP_TABLE_1);

        assert!(matches!(msg.get_type().unwrap(), MsgTypeKind::NET_MEASURE_MSG));
        assert!(matches!(msg.get_relay_method().unwrap(), RelayMethodKind::LOOKUP_TABLE_1));

        msg.set_type(MsgTypeKind::PAYLOAD_MSG);

        assert!(matches!(msg.get_type().unwrap(), MsgTypeKind::PAYLOAD_MSG));
        assert_eq!(msg.is_relay(), false);
        msg.set_relay_method(RelayMethodKind::KAD);
        msg.set_relay(true);
        

        assert_eq!(msg.is_relay(), true);
        assert!(matches!(msg.get_type().unwrap(), MsgTypeKind::PAYLOAD_MSG));
        assert!(matches!(msg.get_relay_method().unwrap(), RelayMethodKind::KAD));
    }


    #[test]
    fn msg_header2() {
        setup_logger("message").unwrap();
        let payload = [1_u8; 2];
        let peer = Peer::from_bytes(&[1]);
        

        let mut msg = OverlayMessage::new(
            0b00100000000000000000000000000000,
            &peer,
            &peer,
            &peer,
            &payload
        );

        msg.set_timestamp_now();

        msg.set_fanout(10).unwrap();
        msg.set_ttl(15).unwrap();

        assert_eq!(msg.get_fanout(), 10);
        assert_eq!(msg.get_ttl(), 15);

        msg.set_relay(true);
        msg.set_ttl(10).unwrap();

        assert_eq!(msg.get_fanout(), 10);
        assert_eq!(msg.get_ttl(), 10);
    }
}