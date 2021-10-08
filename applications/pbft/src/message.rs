use prost::Message;
use yulong_network::{
    error::DeserializeError,
    identity::{Peer,
        crypto::{
            AsBytes,
        },
    }
};
use crate::pbft_message::ProtoPbftMessage;

pub struct PbftMessage {
    pub round: u32,
    
    pub msg_no: u32,
    pub msg_type: u32,

    pub dst_id: Peer,
    pub src_id: Peer,
    pub signer_id: Peer,

    pub proof: Vec<u8>,
    pub payload: Vec<u8>,
}

impl AsBytes for PbftMessage {

    fn into_bytes(&self) -> Vec<u8> {

        let proto_message = ProtoPbftMessage {
            round: self.round,
            msg_no: self.msg_no,
            msg_type: self.msg_type,
            dst_id: self.dst_id.get_id().to_vec(),
            src_id: self.src_id.get_id().to_vec(),
            signer_id: self.signer_id.get_id().to_vec(),
            proof: self.proof.clone(),
            payload: self.payload.clone(),
        };

        let mut buf = Vec::with_capacity(proto_message.encoded_len());
        proto_message.encode(&mut buf).unwrap();
        buf
    }

    fn from_bytes(buf: &[u8]) -> Result<Self, DeserializeError> {
        ProtoPbftMessage::decode(buf)
            .map_err(|e| DeserializeError::new("Deserialize PbftMessage", e))
            .map(|m| Self {
                round: m.round,
                msg_no: m.msg_no,
                msg_type: m.msg_type,
 
                dst_id: Peer::try_from_id(&m.dst_id).expect("Wrong ID size."),
                src_id: Peer::try_from_id(&m.src_id).expect("Wrong ID size."),
                signer_id: Peer::try_from_id(&m.signer_id).expect("Wrong ID size."),

                proof: m.proof,
                payload: m.payload,
            })
    }
}


impl PbftMessage {
}

#[cfg(test)]
mod test {

    use yulong_network::identity::{
        crypto::{
            AsBytes,
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
            msg_type: 0,
        
            dst_id: message::Peer::from_random(),
            src_id: message::Peer::from_public_key(&PublicKey::SM2(pk.clone())),
            signer_id: message::Peer::from_bytes(&[1,2,3,4,5,6]),
        
            proof: signer.sign(&vec![3,3,3,3,3,3,3,3,3], &sk, &pk).into_bytes(),
            payload: vec![3,3,3,3,3,3,3,3,3],
        };

        let msg_bytes = msg.into_bytes();

        // println!("Serialized msg: {:?}", msg_bytes);

        let dse_msg = message::PbftMessage::from_bytes(&msg_bytes).ok().unwrap();
        assert_eq!(msg.round, dse_msg.round);
        assert_eq!(msg.msg_no, dse_msg.msg_no);
        assert_eq!(msg.msg_type, dse_msg.msg_type);
        assert_eq!(msg.proof, dse_msg.proof);
        assert_eq!(msg.payload, dse_msg.payload);

        assert_eq!(msg.dst_id.get_id(), dse_msg.dst_id.get_id());
        assert_eq!(msg.src_id.get_id(), dse_msg.src_id.get_id());
        assert_eq!(msg.signer_id.get_id(), dse_msg.signer_id.get_id());
    }

}