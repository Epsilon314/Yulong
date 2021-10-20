pub mod bidirct_hashmap;
pub mod type_alias;

use super::error::{SerializeError, DeserializeError};

pub trait AsBytes: Sized {
    fn into_bytes(&self) -> Result<Vec<u8>, SerializeError>;
    fn from_bytes(buf: &[u8]) -> Result<Self, DeserializeError>;
}