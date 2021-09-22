use libsm::sm2;
use crate::identity::crypto::{SeDer, Signer};
use crate::error::{DeserializeError, DumbError};

pub struct SmSigner {
    ctx: sm2::signature::SigCtx
}

impl SmSigner {
    pub fn new() -> Self {
        Self {
            ctx: sm2::signature::SigCtx::new()
        }
    }
}

#[derive(Clone)]
pub struct SmPubKey {
    pk: sm2::signature::Pubkey
}

#[derive(Clone)]
pub struct SmSecKey {
    sk: sm2::signature::Seckey
}

pub struct SmSig {
    sig: sm2::signature::Signature
}

impl SeDer for SmPubKey {
    fn into_bytes(&self) -> Vec<u8> {
        let cx = sm2::signature::SigCtx::new();
        cx.serialize_pubkey(&self.pk, true)
    }

    fn from_bytes(buf: &[u8]) -> Result<Self, DeserializeError> {
        let cx = sm2::signature::SigCtx::new();
        match cx.load_pubkey(buf) {
            Ok(key) => {
                Ok(SmPubKey{pk: key})
            }
            Err(_) => {
                Err(DeserializeError::new("Load pubkey error", DumbError))
            }
        } 
    }
}

impl SeDer for SmSecKey {
    fn into_bytes(&self) -> Vec<u8> {
        let cx = sm2::signature::SigCtx::new();
        cx.serialize_seckey(&self.sk)
    }

    fn from_bytes(buf: &[u8]) -> Result<Self, crate::error::DeserializeError> {
        let cx = sm2::signature::SigCtx::new();
        match cx.load_seckey(buf) {
            Ok(key) => {
                Ok(SmSecKey{sk: key})
            }
            Err(_) => {
                Err(DeserializeError::new("Load seckey error", DumbError))
            }
        }
    }
}

impl SeDer for SmSig {
    fn into_bytes(&self) -> Vec<u8> {
        self.sig.der_encode()
    }

    fn from_bytes(buf: &[u8]) -> Result<Self, DeserializeError> {
        match sm2::signature::Signature::der_decode(buf) {
            Ok(sig) => {
                Ok(Self {sig})
            }

            Err(error) => {
                Err(DeserializeError::new("Load signature failed", error))
            }
        }
    }
}

impl Signer for SmSigner {
    
    type PK = SmPubKey;

    type SK = SmSecKey;

    type SIG = SmSig;

    fn keygen(&self) -> (Self::PK, Self::SK) {
        let (pk, sk) = self.ctx.new_keypair();
        (Self::PK{pk}, Self::SK{sk})
    }

    fn sign(&self, msg: &[u8], sk: &Self::SK, pk: &Self::PK) -> Self::SIG {
        Self::SIG {
            sig: self.ctx.sign(msg, &sk.sk, &pk.pk)
        }
    }

    fn verify(&self, msg: &[u8], pk: &Self::PK, sig: &Self::SIG) -> bool {
        self.ctx.verify(msg, &pk.pk, &sig.sig)
    }
}