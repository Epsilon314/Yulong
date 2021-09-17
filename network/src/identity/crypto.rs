#![allow(unused_variables)]

use crate::error::DeserializeError;

pub trait SeDer: Sized {
    fn into_bytes(&self) -> Vec<u8>;
    fn from_bytes(buf: &[u8]) -> Result<Self, DeserializeError>;
}

pub trait Signer: 'static + Send {

    type PK: SeDer + Send + Clone;
    type SK: SeDer + Send + Clone;
    type SIG: SeDer + Send;
    
    fn keygen(self) -> (Self::PK, Self::SK);

    fn sign(self, msg: &[u8], sk: &Self::SK, pk: &Self::PK) -> Self::SIG;
    fn verify(self, msg: &[u8], pk: &Self::PK, sig: &Self::SIG) -> bool;
}


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

    fn keygen(self) -> (Self::PK, Self::SK) {
        unimplemented!()
    }

    fn sign(self, msg: &[u8], sk: &Self::SK, pk: &Self::PK) -> Self::SIG {
        unimplemented!()
    }

    fn verify(self, msg: &[u8], pk: &Self::PK, sig: &Self::SIG) -> bool {
        unimplemented!()
    }
}