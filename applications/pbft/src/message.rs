use std::fmt::Display;

use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::{FromPrimitive, ToPrimitive};
use prost::Message;

use yulong::error::{DeserializeError, SerializeError};
use yulong::utils::AsBytes;
use yulong_network::identity::crypto::{GenericSigner, PublicKey, PrivateKey};

use yulong_network::identity::Peer;

use crate::pbft_message::ProtoPbftMessage;


#[derive(FromPrimitive, ToPrimitive, PartialEq, Debug, Clone, Copy)]
pub enum PbftMsgKind {
    REQUEST = 0,
    PRE_PREPARE = 1,
    PREPARE = 2,
    COMMIT = 3,
    REPLY = 4,

    BLAME = 5,
    NEW_EPOCH = 6,
    CONFIRM_EPOCH = 7,
}


#[derive(Debug)]
pub struct PbftMessage {
    round: u32,
    
    msg_no: u32,
    msg_type: PbftMsgKind,

    signer_id: Peer,

    proof: Vec<u8>,
    payload: Vec<u8>,
}

impl AsBytes for PbftMessage {

    fn into_bytes(&self) -> Result<Vec<u8>, SerializeError> {

        let proto_message = ProtoPbftMessage {
            round: self.round,
            msg_no: self.msg_no,
            msg_type: ToPrimitive::to_u32(&self.msg_type).unwrap(),
            signer_id: self.signer_id.get_id().to_vec(),
            proof: self.proof.clone(),
            payload: self.payload.clone(),
        };

        let mut buf = Vec::with_capacity(proto_message.encoded_len());
        proto_message.encode(&mut buf).unwrap();
        Ok(buf)
    }

    fn from_bytes(buf: &[u8]) -> Result<Self, DeserializeError> {
        ProtoPbftMessage::decode(buf)
            .map_err(|e| DeserializeError::new("Deserialize PbftMessage", e))
            .map(|m| Self {
                round: m.round,
                msg_no: m.msg_no,
                msg_type: FromPrimitive::from_u32(m.msg_type).unwrap(),
 
                signer_id: Peer::try_from_id(&m.signer_id).expect("Wrong ID size."),

                proof: m.proof,
                payload: m.payload,
            })
    }
}


impl PbftMessage {


    // do not sign
    pub fn new(round: u32, msg_no: u32, msg_type: PbftMsgKind, signer_id: Peer,
        payload: Vec<u8>) -> Self 
    { 
        Self {
            round,
            msg_no,
            msg_type,
            signer_id,
            proof: vec![],
            payload
        }
    }


    pub fn sign<S: GenericSigner>(&mut self,s: &S, sk: &PrivateKey, pk: &PublicKey) {
        let sig = GenericSigner::sign(s, &self.into_bytes().unwrap(), sk, pk);
        self.proof = sig.into_bytes().unwrap();
    }


    pub fn verify(&self) -> bool {
        //todo: check if proof is valid
        true
    }

    /// Get a reference to the pbft message's round.
    pub fn round(&self) -> u32 {
        self.round
    }

    /// Set the pbft message's round.
    pub fn set_round(&mut self, round: u32) {
        self.round = round;
    }

    /// Get a reference to the pbft message's msg no.
    pub fn msg_no(&self) -> u32 {
        self.msg_no
    }

    /// Set the pbft message's msg no.
    pub fn set_msg_no(&mut self, msg_no: u32) {
        self.msg_no = msg_no;
    }

    /// Get a reference to the pbft message's msg type.
    pub fn msg_type(&self) -> PbftMsgKind {
        self.msg_type
    }

    /// Set the pbft message's msg type.
    pub fn set_msg_type(&mut self, msg_type: PbftMsgKind) {
        self.msg_type = msg_type;
    }

    /// Get a reference to the pbft message's signer id.
    pub fn signer_id(&self) -> &Peer {
        &self.signer_id
    }

    /// Set the pbft message's signer id.
    pub fn set_signer_id(&mut self, signer_id: Peer) {
        self.signer_id = signer_id;
    }

    /// Get a reference to the pbft message's proof.
    pub fn proof(&self) -> &[u8] {
        self.proof.as_ref()
    }

    /// Set the pbft message's proof.
    pub fn set_proof(&mut self, proof: Vec<u8>) {
        self.proof = proof;
    }

    /// Get a reference to the pbft message's payload.
    pub fn payload(&self) -> &[u8] {
        self.payload.as_ref()
    }

    /// Set the pbft message's payload.
    pub fn set_payload(&mut self, payload: Vec<u8>) {
        self.payload = payload;
    }
}

#[cfg(test)]
mod test {

    use super::*;

    use yulong::utils::AsBytes;

    use yulong_network::identity::{
        crypto::{
            Signer,
            sm_signer::SmSigner,
            PublicKey,
        }
    };

    use crate::*;

    #[test]
    fn pbft_message_serialize_deserialize() {

        let signer = SmSigner::new();
        let (pk, sk) = signer.keygen();

        let msg = message::PbftMessage {
            round: 10,
    
            msg_no: 11278,
            msg_type: PbftMsgKind::PREPARE,
        
            signer_id: message::Peer::from_bytes(&[1,2,3,4,5,6]),
        
            proof: Signer::sign(&signer, &vec![3,3,3,3,3,3,3,3,3], &sk, &pk).into_bytes().unwrap(),
            payload: vec![3,3,3,3,3,3,3,3,3],
        };

        let msg_bytes = msg.into_bytes().unwrap();

        // println!("Serialized msg: {:?}", msg_bytes);

        let dse_msg = message::PbftMessage::from_bytes(&msg_bytes).ok().unwrap();
        assert_eq!(msg.round, dse_msg.round);
        assert_eq!(msg.msg_no, dse_msg.msg_no);
        assert_eq!(msg.msg_type, dse_msg.msg_type);
        assert_eq!(msg.proof, dse_msg.proof);
        assert_eq!(msg.payload, dse_msg.payload);

        assert_eq!(msg.signer_id.get_id(), dse_msg.signer_id.get_id());
    }

}