pub mod crypto;

use libsm::sm3;
use rand;
use rand::Rng;
use std::{fmt::{self,Display,Debug}, hash::Hash};

use yulong::utils::AsBytes;
use yulong::error::DumbError;

use crate::error::TryfromSliceError;
use crypto::{PublicKey, PrivateKey, Signer, sm_signer::SmSigner};

pub struct Me {
    pub peer: Peer,
    pub privatekey: PrivateKey
}

impl Me {

    pub fn new() -> Self {
        
        let signer = SmSigner::new();
        let (pk, sk) = signer.keygen();
        
        let id = Peer::from_public_key(
            &PublicKey::SM2(pk.clone())
        ).raw_id;
        
        Self {
            peer: Peer { raw_id: id, pubkey: PublicKey::SM2(pk) },
            privatekey: PrivateKey::SM2(sk),
        }
    }

    pub fn from_keypair(pk: PublicKey, sk: PrivateKey) -> Self {
        let id = Peer::from_public_key(&pk).raw_id;
        
        Self {
            peer: Peer {
                raw_id: id,
                pubkey: pk,
            },
            privatekey: sk
        }
    }
}


#[derive(Clone)]
pub struct Peer {
    raw_id: [u8; Peer::ID_SIZE],
    pubkey: PublicKey
}


impl Hash for Peer {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.raw_id.hash(state);
    }
}

impl PartialEq for Peer {
    fn eq(&self, other: &Self) -> bool {
        self.raw_id == other.raw_id
    }
}

impl Eq for Peer {}

impl Display for Peer{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Peer id: {:?}", self.get_id())
    }
}

impl Debug for Peer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Peer").field("raw_id", &self.raw_id).field("pubkey", &self.pubkey.into_bytes()).finish()
    }
}

impl Peer {

    pub const ID_SIZE: usize = 32;

    // define non-common ids
    // see Peer::common
    pub const BROADCAST_ID: Peer = Peer{
        raw_id: [0; Peer::ID_SIZE],
        pubkey: PublicKey::NoKey
    };

    
    pub fn common(&self) -> bool {
        if *self == Self::BROADCAST_ID {
            return false;
        }
        true
    }


    pub fn get_id(&self) -> [u8; Peer::ID_SIZE] {
        self.raw_id
    }


    pub fn from_public_key(public_key: &PublicKey) -> Peer {
        match public_key {

            // if no valid key is provided, fallback to random id
            PublicKey::NoKey => {
                Self::from_random()
            }

            // else use public key's hash as id 
            _ => {
                
                let mut hash = sm3::hash::Sm3Hash::new(
                    // PublicKey::into_bytes do not throw error, safe unwrap
                    &public_key.into_bytes().unwrap()
                );

                Self {
                    raw_id: hash.get_hash(),
                    pubkey: public_key.to_owned(),
                }
            }
        }
    }


    pub fn from_random() -> Peer {
        let random_bytes = 
            rand::thread_rng().gen::<[u8; Peer::ID_SIZE]>();
        
        Self {
            raw_id: random_bytes,
            pubkey: PublicKey::NoKey
        }
    }


    pub fn from_bytes(bytes: &[u8]) -> Peer {
        let mut hash = sm3::hash::Sm3Hash::new(bytes);
        Self {
            raw_id: hash.get_hash(),
            pubkey: PublicKey::NoKey
        }
    }


    /// Try to initialize the Peer with a given id.
    ///
    /// *id* is stored as raw bytes. If its len is valid, a Peer with that id 
    /// and NoKey is returned. Otherwise TryfromSliceError is returned.
    pub fn try_from_id(id: &[u8]) -> Result<Peer, TryfromSliceError> {
        
        if id.len() == Peer::ID_SIZE {
            let ptr = id.as_ptr() as *const [u8; Peer::ID_SIZE];
            unsafe {
                Ok(Self{
                    raw_id: *ptr,
                    pubkey: PublicKey::NoKey
                })
            }
        }
        else {
            Err(TryfromSliceError::new(
                format!("Id should be [u8: {}].", Peer::ID_SIZE), 
                DumbError))
        }
    }

}