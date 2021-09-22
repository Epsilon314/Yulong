#![allow(unused_variables)]
pub mod sm_signer;

use crate::error::{DeserializeError, DumbError};
use sm_signer::{SmPubKey, SmSecKey};
use crate::peer_id;
use prost::Message;

pub trait SeDer: Sized {
    fn into_bytes(&self) -> Vec<u8>;
    fn from_bytes(buf: &[u8]) -> Result<Self, DeserializeError>;
}

pub trait Signer: 'static + Send {

    type PK: SeDer + Send + Clone;
    type SK: SeDer + Send + Clone;
    type SIG: SeDer + Send;
    
    fn keygen(&self) -> (Self::PK, Self::SK);

    fn sign(&self, msg: &[u8], sk: &Self::SK, pk: &Self::PK) -> Self::SIG;
    fn verify(&self, msg: &[u8], pk: &Self::PK, sig: &Self::SIG) -> bool;
}

#[derive(Clone)]
pub enum PublicKey {
    SM2(SmPubKey),
    NoKey
}

impl SeDer for PublicKey {

    fn into_bytes(&self) -> Vec<u8> {
        let proto_message = match self {

            PublicKey::SM2(key) => {
                peer_id::PublicKey {
                    r#type: peer_id::CryptoType::Sm2 as i32,
                    data: key.into_bytes()
                }
            }

            PublicKey::NoKey => {
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
                Ok(PublicKey::NoKey)
            }
        }
    }

}

pub enum PrivateKey {
    SM2(SmSecKey),
    NoKey
}

impl PrivateKey {}

// placeholder Signer, do nothing and cannot be called
// Todo: find a better way to express Optional Generics

#[derive(Clone, Copy)]
pub struct NoneSigner {}

#[derive(Clone, Copy)]
pub struct NoneKey {}

impl SeDer for NoneKey {
    fn into_bytes(&self) -> Vec<u8> {
        unimplemented!()
    }

    fn from_bytes(buf: &[u8]) -> Result<Self, DeserializeError> {
        unimplemented!()
    }
}

impl Signer for NoneSigner {
    type PK = NoneKey;
    type SK = NoneKey;
    type SIG = NoneKey;

    fn keygen(&self) -> (Self::PK, Self::SK) {
        unimplemented!()
    }

    fn sign(&self, msg: &[u8], sk: &Self::SK, pk: &Self::PK) -> Self::SIG {
        unimplemented!()
    }

    fn verify(&self, msg: &[u8], pk: &Self::PK, sig: &Self::SIG) -> bool {
        unimplemented!()
    }
}