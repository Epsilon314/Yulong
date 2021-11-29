use std::convert::TryInto;

use log::warn;
use prost_types;
use prost::Message;
use crate::bdn_message::{
    MlbtMessage,
    MlbtMerge,
    MlbtMergeCheck,
    MlbtGrant,
    MlbtGrantInfo,
    MlbtRetract,
};

use yulong::utils::AsBytes;
use yulong::error::{DumbError, SerializeError, DeserializeError};
use yulong_network::identity::Peer;

use num_traits::{FromPrimitive, ToPrimitive};

#[allow(non_camel_case_types)]
#[derive(FromPrimitive, ToPrimitive, Clone, Copy, Debug)]
pub enum RelayMsgKind {
    JOIN = 0,
    LEAVE = 1,
    
    ACCEPT = 2,
    REJECT = 3,
    
    MERGE = 4,
    MERGE_CHECK = 5,

    GRANT = 6,
    GRANT_INFO = 7,

    RETRACT = 8,
    RETRACT_INFO = 9,
    RETRACT_REPLY = 10,
}


#[derive(Debug)]
pub struct RelayCtlMessage {
    msg_type: RelayMsgKind,
    msg_id: u64,
    payload: Vec<u8>,
}

impl AsBytes for RelayCtlMessage {

    fn into_bytes(&self) -> Result<Vec<u8>, SerializeError> {

        let protobuf_msg = MlbtMessage {
            message_type: ToPrimitive::to_u32(&self.msg_type()).unwrap(),
            message_id: self.msg_id,
            payload: self.payload.clone(),
        };

        let protobuf_bytes_len = protobuf_msg.encoded_len();
        let mut protobuf_buf: Vec<u8> = Vec::with_capacity(protobuf_bytes_len);
        match protobuf_msg.encode(&mut protobuf_buf) {
            Ok(_) => {
                Ok(protobuf_buf)
            }

            Err(error) => {
                Err(SerializeError::new(
                    "RelayCtlMessage::into_bytes",
                    error)
                )
            }
        }
    }


    fn from_bytes(buf: &[u8]) -> Result<Self, DeserializeError> {
        
        match MlbtMessage::decode(buf) {
            Ok(msg) => {

                let mtype: Option<RelayMsgKind> = FromPrimitive::from_u32(msg.message_type);
                if mtype.is_none() {
                    warn!("RelayCtlMessage::from_bytes decode msg type error");
                    return Err(DeserializeError::new("decode msg type error", DumbError))
                }
                let mtype = mtype.unwrap();

                Ok(Self{
                    msg_type: mtype,
                    msg_id: msg.message_id,
                    payload: msg.payload,
                })
            }
            Err(error) => {
                warn!("RelayCtlMessage::from_bytes decode error {}", error);
                Err(DeserializeError::new("decode error", error))
            }
        }
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


    pub fn accept(&self, seq: u64) -> Self {
        Self::new(
            RelayMsgKind::ACCEPT,
            seq,
            RelayMsgAccept::new(self)
        )
    }


    pub fn reject(&self, seq: u64) -> Self {
        Self::new(
            RelayMsgKind::REJECT,
            seq,
            RelayMsgReject::new(self)
        )
    }

}

// define payload structures for each msg_type


// join message only need to specify the broadcast tree to join by 
// including its src id
#[derive(Debug, Clone)]
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

    pub fn new(src: Peer) -> Self {
        Self {
            src: src,
        }
    }

    pub fn src(&self) -> &Peer {
        &self.src
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

    pub fn new(src: &Peer) -> Self {
        Self {
            src: src.to_owned()
        }
    }

    pub fn src(&self) -> &Peer {
        &self.src
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


    pub fn from_id(id: u64) -> Self {
        Self{ack: id}
    }


    pub fn ack(&self) -> u64 {
        self.ack
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

    pub fn ack(&self) -> u64 {self.ack}
}


#[derive(Debug)]
pub struct RelayMsgMerge {
    weight: u64,
    merge_thrd: u64,
    src: Peer,
}


impl AsBytes for RelayMsgMerge {

    fn into_bytes(&self) -> Result<Vec<u8>, SerializeError> {
        let protobuf_msg = MlbtMerge {
            weight: self.weight,
            thrd: self.merge_thrd,
            src_id: self.src.get_id().to_vec(),
        };

        let protobuf_bytes_len = protobuf_msg.encoded_len();
        let mut protobuf_buf: Vec<u8> = Vec::with_capacity(protobuf_bytes_len);
        match protobuf_msg.encode(&mut protobuf_buf) {
            Ok(_) => {
                Ok(protobuf_buf)
            }

            Err(error) => {
                Err(SerializeError::new(
                    "RelayMsgMerge::into_bytes",
                    error
                ))
            }
        }

    }


    fn from_bytes(buf: &[u8]) -> Result<Self, DeserializeError> {
        match MlbtMerge::decode(buf) {
            Ok(msg) => {
                let peer = Peer::try_from_id(&msg.src_id);
                if peer.is_err() {
                    return Err(DeserializeError::new(
                        "RelayMsgMerge::from_bytes", 
                        peer.unwrap_err()));
                }

                Ok(Self{
                    weight: msg.weight,
                    merge_thrd: msg.thrd,
                    src: peer.unwrap()
                })

            }

            Err(error) => {
                Err(DeserializeError::new("RelayMsgMerge::from_bytes", error))
            }
        }
    }

}


impl RelayMsgMerge {

    pub fn new(weight: u64, merge_thrd: u64, src: Peer) -> Self {
        Self {
            weight,
            merge_thrd,
            src,
        }
    }

    
    pub fn weight(&self) -> u64 {
        self.weight
    }


    pub fn merge_thrd(&self) -> u64 {
        self.merge_thrd
    }


    pub fn src(&self) -> &Peer {
        &self.src
    }

}


#[derive(Debug)]
pub struct RelayMsgMergeCheck {
    weight: u64,
}


impl AsBytes for RelayMsgMergeCheck {

    // todo: write some protobuf helper to shorten this repeated pattern
    fn into_bytes(&self) -> Result<Vec<u8>, SerializeError> {
        let protobuf_msg = MlbtMergeCheck {
            weight: self.weight
        };

        let protobuf_bytes_len = protobuf_msg.encoded_len();
        let mut protobuf_buf: Vec<u8> = Vec::with_capacity(protobuf_bytes_len);
        match protobuf_msg.encode(&mut protobuf_buf) {
            Ok(_) => {
                Ok(protobuf_buf)
            }

            Err(error) => {
                Err(SerializeError::new(
                    "RelayMsgMergeCheck::into_bytes",
                    error
                ))
            }
        }
    }


    fn from_bytes(buf: &[u8]) -> Result<Self, DeserializeError> {
        match MlbtMergeCheck::decode(buf) {
            Ok(msg) => {
                Ok(Self{
                    weight: msg.weight,
                })
            }

            Err(error) => {
                Err(DeserializeError::new("RelayMsgMergeCheck::from_bytes", error))
            }
        }
    }
}


impl RelayMsgMergeCheck {
    pub fn new(weight: u64) -> Self {
        Self {
            weight
        }
    }

    pub fn weight(&self) -> u64 {
        self.weight
    }
}


#[derive(Debug)]
pub struct RelayMsgGrant {
    target_id: Peer,
    src_inv: u64,
    src_id: Peer,
}


impl AsBytes for RelayMsgGrant {

    fn into_bytes(&self) -> Result<Vec<u8>, SerializeError> {
        let protobuf_msg = MlbtGrant {
            target_id: self.target_id.get_id().to_vec(),
            src_inv: self.src_inv,
            src_id: self.src_id.get_id().to_vec(),
        };

        let protobuf_bytes_len = protobuf_msg.encoded_len();
        let mut protobuf_buf: Vec<u8> = Vec::with_capacity(protobuf_bytes_len);
        match protobuf_msg.encode(&mut protobuf_buf) {
            Ok(_) => {
                Ok(protobuf_buf)
            }

            Err(error) => {
                Err(SerializeError::new(
                    "RelayMsgGrant::into_bytes",
                    error
                ))
            }
        }
    }


    fn from_bytes(buf: &[u8]) -> Result<Self, DeserializeError> {
        match MlbtGrant::decode(buf) {

            Ok(msg) => {

                let target = Peer::try_from_id(&msg.target_id);
                if target.is_err() {
                    return Err(DeserializeError::new(
                        "RelayMsgGrant::from_bytes", 
                        target.unwrap_err()));
                }
    
                let src = Peer::try_from_id(&msg.src_id);
                if src.is_err() {
                    return Err(DeserializeError::new(
                        "RelayMsgGrant::from_bytes", 
                        src.unwrap_err()));
                }

                Ok(Self {
                    target_id: target.unwrap(),
                    src_inv: msg.src_inv,
                    src_id: src.unwrap()
                })
            }

            Err(error) => {
                Err(DeserializeError::new("RelayMsgMergeCheck::from_bytes", error))
            }
        }
    }
}


impl RelayMsgGrant {
    pub fn new(target_id: Peer, src_inv: u64, src_id: Peer) -> Self {
        Self { 
            target_id,
            src_inv,
            src_id
        }
    }

    /// Get a reference to the relay msg grant's grant id.
    pub fn target_id(&self) -> &Peer {
        &self.target_id
    }

    /// Get a reference to the relay msg grant's src inv.
    pub fn src_inv(&self) -> u64 {
        self.src_inv
    }

    /// Get a reference to the relay msg grant's src id.
    pub fn src_id(&self) -> &Peer {
        &self.src_id
    }
}


#[derive(Debug)]
pub struct RelayMsgGrantInfo {
    src_id: Peer,
    // todo: add more metric to decide whether to shift delegate
}


impl AsBytes for RelayMsgGrantInfo {

    fn into_bytes(&self) -> Result<Vec<u8>, SerializeError> {
        let protobuf_msg = MlbtGrantInfo {
            src_id: self.src_id.get_id().to_vec(),
        };

        let protobuf_bytes_len = protobuf_msg.encoded_len();
        let mut protobuf_buf: Vec<u8> = Vec::with_capacity(protobuf_bytes_len);
        match protobuf_msg.encode(&mut protobuf_buf) {
            Ok(_) => {
                Ok(protobuf_buf)
            }

            Err(error) => {
                Err(SerializeError::new(
                    "RelayMsgGrantInfo::into_bytes",
                    error
                ))
            }
        }
    }


    fn from_bytes(buf: &[u8]) -> Result<Self, DeserializeError> {
        match MlbtGrantInfo::decode(buf) {

            Ok(msg) => {

                let src = Peer::try_from_id(&msg.src_id);
                if src.is_err() {
                    return Err(DeserializeError::new(
                        "RelayMsgGrantInfo::from_bytes", 
                        src.unwrap_err()));
                }

                Ok(Self {
                    src_id: src.unwrap()
                })
            }

            Err(error) => {
                Err(DeserializeError::new("RelayMsgGrantInfo::from_bytes", error))
            }
        }
    }

}


impl RelayMsgGrantInfo {
    pub fn new(src_id: Peer) -> Self { Self { src_id } }



    /// Get a reference to the relay msg grant info's src id.
    pub fn src_id(&self) -> &Peer {
        &self.src_id
    }
}


#[derive(Debug)]
pub struct RelayMsgRetract {
    src_id: Peer,
    src_inv: u64,
}


impl AsBytes for RelayMsgRetract {
    fn into_bytes(&self) -> Result<Vec<u8>, SerializeError> {
        todo!()
    }

    fn from_bytes(buf: &[u8]) -> Result<Self, DeserializeError> {
        todo!()
    }
}


impl RelayMsgRetract {
    pub fn new(src_id: Peer, relay_inv: u64) -> Self {
        Self {
            src_id,
            src_inv: relay_inv
        }
    }



    /// Get a reference to the relay msg retract's src id.
    pub fn src_id(&self) -> &Peer {
        &self.src_id
    }

    /// Get a reference to the relay msg retract's relay inv.
    pub fn src_inv(&self) -> u64 {
        self.src_inv
    }
}


#[derive(Debug)]
pub struct RelayMsgRetractReply {
    src_id: Peer,
    target_id: Peer,
    src_inv: u64,
}

impl AsBytes for RelayMsgRetractReply {
    fn into_bytes(&self) -> Result<Vec<u8>, SerializeError> {
        todo!()
    }

    fn from_bytes(buf: &[u8]) -> Result<Self, DeserializeError> {
        todo!()
    }
}

impl RelayMsgRetractReply {
    pub fn new(src_id: Peer, target_id: Peer, src_inv: u64) -> Self { Self { src_id, target_id, src_inv } }


    /// Get a reference to the relay msg retract reply's target id.
    pub fn target_id(&self) -> &Peer {
        &self.target_id
    }

    /// Get a reference to the relay msg retract reply's src id.
    pub fn src_id(&self) -> &Peer {
        &self.src_id
    }

    /// Get a reference to the relay msg retract reply's src inv.
    pub fn src_inv(&self) -> u64 {
        self.src_inv
    }
}



#[derive(Debug)]
pub struct RelayMsgRetractInfo {
    src_id: Peer,
    src_inv: u64,
}


impl AsBytes for RelayMsgRetractInfo {
    fn into_bytes(&self) -> Result<Vec<u8>, SerializeError> {
        todo!()
    }

    fn from_bytes(buf: &[u8]) -> Result<Self, DeserializeError> {
        todo!()
    }
}


impl RelayMsgRetractInfo {
    pub fn new(src_id: Peer, src_inv: u64) -> Self { Self { src_id, src_inv } }

    

    /// Get a reference to the relay msg retract info's src id.
    pub fn src_id(&self) -> &Peer {
        &self.src_id
    }

    /// Get a reference to the relay msg retract info's src inv.
    pub fn src_inv(&self) -> u64 {
        self.src_inv
    }
}





#[cfg(test)]
mod test {

    use log::debug;

    use super::*;


    #[test]
    fn ctl_msg_serde() {

        let peer = Peer::from_random();

        let payload = RelayMsgJoin::new(peer);

        let msg = RelayCtlMessage::new(
            RelayMsgKind::JOIN,
            15514,
            payload.clone()
        );

        let buf = msg.into_bytes().unwrap();
        debug!("Serialized Message: {:?}", buf);

        let de_msg = RelayCtlMessage::from_bytes(&buf).unwrap();

        assert!(matches!(de_msg.msg_type(), RelayMsgKind::JOIN));
        assert_eq!(de_msg.msg_id(), 15514);
        assert_eq!(de_msg.payload(), payload.into_bytes().unwrap());
    }

}