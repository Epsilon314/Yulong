use libsm::sm3;
use libsm::sm2;
use crate::{peer_id, error::DeserializeError, error::DumbError};
use prost::Message;
use rand;
use rand::Rng;

#[derive(Clone, Copy, Hash, PartialOrd, PartialEq)]
pub struct Peer {
    raw_id: [u8;32],

}

impl Peer {

    pub fn from_public_key(public_key: PublicKey) -> Peer {
        let mut hash = sm3::hash::Sm3Hash::new(&public_key.serialize());
        Peer {
            raw_id: hash.get_hash()
        }
    }

    pub fn from_random() -> Peer {
        let random_bytes = rand::thread_rng().gen::<[u8;32]>();
        Peer {
            raw_id: random_bytes
        }
    }

    pub fn from_bytes(bytes: &[u8]) -> Peer {
        let mut hash = sm3::hash::Sm3Hash::new(bytes);
        Peer {
            raw_id: hash.get_hash()
        }
    }

}

pub enum PublicKey {
    SM2(sm2::signature::Pubkey)
}

impl PublicKey {

    pub fn serialize(self) -> Vec<u8> {
        let proto_message = match self {

            PublicKey::SM2(key) => {
                let ctx = sm2::signature::SigCtx::new();
                peer_id::PublicKey {
                    r#type: peer_id::CryptoType::Sm2 as i32,
                    data: ctx.serialize_pubkey(&key, true)
                }
            }
        };
        let mut buf = Vec::with_capacity(proto_message.encoded_len());
        proto_message.encode(&mut buf).unwrap();
        buf
    }

    pub fn deserialize(bytes: &[u8]) -> Result<PublicKey, DeserializeError> {

        #[allow(unused_mut)]
        let mut key = peer_id::PublicKey::decode(bytes)
            .map_err(|e| DeserializeError::new("Deserialize public key", e))?;

        let key_type = peer_id::CryptoType::from_i32(key.r#type).ok_or_else(
            || DeserializeError::new("Deserialize public key, unknown type", DumbError)
        )?;

        match key_type {
            peer_id::CryptoType::Sm2 => {
                let ctx = sm2::signature::SigCtx::new();
                match ctx.load_pubkey(&key.data) {
                    Ok(sm2_key) => {
                        Ok(PublicKey::SM2(sm2_key))
                    }
                    Err(_) => {
                        Err(DeserializeError::new("Deserialize SM2 public key", DumbError))
                    }
                }
            },
        }
    }

}

pub enum PrivateKey {
    SM2(sm2::signature::Seckey)
}

impl PrivateKey {}