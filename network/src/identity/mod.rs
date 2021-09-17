pub mod crypto;
pub mod sm_signer;

use libsm::sm3;
use crate::{peer_id, error::DeserializeError, error::DumbError, error::TryfromSliceError};
use prost::Message;
use rand;
use rand::Rng;
use crypto::{SeDer};
use sm_signer::{SmPubKey, SmSecKey};

pub struct Me {
    raw_id: [u8; Peer::ID_SIZE],
    pubkey: PublicKey,
    privatekey: PrivateKey
}

// impl Me {
//     pub fn new() -> Self {
//     }
// }


#[derive(Clone)]
pub struct Peer {
    raw_id: [u8; Peer::ID_SIZE],
    pubkey: PublicKey
}

impl Peer {

    const ID_SIZE: usize = 32;

    pub fn get_id(&self) -> [u8; Peer::ID_SIZE] {
        self.raw_id
    }

    pub fn from_public_key(public_key: PublicKey) -> Peer {
        let mut hash = sm3::hash::Sm3Hash::new(&public_key.into_bytes());
        Self {
            raw_id: hash.get_hash(),
            pubkey: public_key,
        }
    }

    pub fn from_random() -> Peer {
        let random_bytes = rand::thread_rng().gen::<[u8; Peer::ID_SIZE]>();
        Self {
            raw_id: random_bytes,
            pubkey: PublicKey::Unknown
        }
    }

    pub fn from_bytes(bytes: &[u8]) -> Peer {
        let mut hash = sm3::hash::Sm3Hash::new(bytes);
        Self {
            raw_id: hash.get_hash(),
            pubkey: PublicKey::Unknown
        }
    }

    pub fn try_from_id(id: &[u8]) -> Result<Peer, TryfromSliceError> {
        
        if id.len() == Peer::ID_SIZE {
            let ptr = id.as_ptr() as *const [u8; Peer::ID_SIZE];
            unsafe {
                Ok(Self{
                    raw_id: *ptr,
                    pubkey: PublicKey::Unknown
                })
            }
        }
        else {
            Err(TryfromSliceError::new(format!("Id should be [u8: {}].", Peer::ID_SIZE), DumbError))
        }
    }
}

#[derive(Clone)]
pub enum PublicKey {
    SM2(SmPubKey),
    Unknown
}

impl crypto::SeDer for PublicKey {

    fn into_bytes(&self) -> Vec<u8> {
        let proto_message = match self {

            PublicKey::SM2(key) => {
                peer_id::PublicKey {
                    r#type: peer_id::CryptoType::Sm2 as i32,
                    data: key.into_bytes()
                }
            }

            PublicKey::Unknown => {
                peer_id::PublicKey {
                    r#type: peer_id::CryptoType::Unknown as i32,
                    data: vec![]
                }
            }
        };
        let mut buf = Vec::with_capacity(proto_message.encoded_len());
        proto_message.encode(&mut buf).unwrap();
        buf
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self, DeserializeError> {

        #[allow(unused_mut)]
        let mut key = peer_id::PublicKey::decode(bytes)
            .map_err(|e| DeserializeError::new("Deserialize public key", e))?;

        let key_type = peer_id::CryptoType::from_i32(key.r#type).ok_or_else(
            || DeserializeError::new("Deserialize public key, unknown type", DumbError)
        )?;

        match key_type {
            peer_id::CryptoType::Sm2 => {
                match  SmPubKey::from_bytes(&key.data) {
                    Ok(sm2_key) => {
                        Ok(PublicKey::SM2(sm2_key))
                    }
                    Err(_) => {
                        Err(DeserializeError::new("Deserialize SM2 public key", DumbError))
                    }
                }
            }

            peer_id::CryptoType::Unknown => {
                Ok(PublicKey::Unknown)
            }
        }
    }

}

pub enum PrivateKey {
    SM2(SmSecKey)
}

impl PrivateKey {}