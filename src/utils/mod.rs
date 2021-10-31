pub mod bidirct_hashmap;
pub mod type_alias;

use std::time::SystemTime;

use log::warn;

use super::error::{SerializeError, DeserializeError};

pub trait AsBytes: Sized {
    fn into_bytes(&self) -> Result<Vec<u8>, SerializeError>;
    fn from_bytes(buf: &[u8]) -> Result<Self, DeserializeError>;
}


/// count down timer
/// set a count-down value, start the timer, and query whether the count-down value reaches 0
#[derive(Clone)]
pub struct CasualTimer {
    last_seen: Option<SystemTime>,
    counter: u128,
}


impl CasualTimer {

    pub fn new(counter: u128) -> Self {
        Self {
            last_seen: None,
            counter,
        }
    }


    pub fn set_now(&mut self) {
        self.last_seen = Some(SystemTime::now());
    }

    
    pub fn is_timeout(&self) -> bool {
        match self.last_seen {

            Some(earlier) => {
                let escaped = SystemTime::now().duration_since(earlier);
                if escaped.is_err() {
                    warn!("CasualTimer::is_timeout clock go backwards {}", 
                        escaped.unwrap_err());
                    return false;
                }
                let escaped = escaped.unwrap();
                escaped.as_millis() > self.counter
            }

            None => {
                warn!("CasualTimer::is_timeout Timer is not sent thus cannot timeout");
                false
            }
        }
    }

}