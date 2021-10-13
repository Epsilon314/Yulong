use std::error::Error;
use std::fmt::{self, Debug, Display};


// dumb error
// no concrete meaning or usage, just the result of fighting the complier :(

#[derive(Debug)]
pub struct DumbError;
impl Display for DumbError {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        unreachable!("dumb error")
    }
}


impl Error for DumbError {}



#[derive(Debug)]
pub struct DeserializeError {
    describe: String,
    boxed_error: Box<dyn Error>
}


impl DeserializeError {
    pub fn new<S: ToString>(des: S, err: impl Error + 'static) -> Self {
        Self {
            describe: des.to_string(),
            boxed_error: Box::new(err)
        }
    }
}


impl Error for DeserializeError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        
        // todo: how to turn self.boxed_error into &(dyn Error + 'static) ?
        // Sized is not satisfied 

        None
    }
}


impl Display for DeserializeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Deserialize error: {}", self.describe)
    }
}



#[derive(Debug)]
pub struct SerializeError {
    describe: String,
    boxed_error: Box<dyn Error>
}


impl SerializeError {
    pub fn new<S: ToString>(des: S, err: impl Error + 'static) -> Self {
        Self {
            describe: des.to_string(),
            boxed_error: Box::new(err)
        }
    }
}


impl Error for SerializeError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}


impl Display for SerializeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Serialize error: {}", self.describe)
    }
}



#[derive(Debug)]
pub struct TransportError {
    describe: String,
    boxed_error: Box<dyn Error>
}


impl TransportError {
    pub fn new<S: ToString>(des: S, err: impl Error + 'static) -> Self {
        Self {
            describe: des.to_string(),
            boxed_error: Box::new(err)
        }
    }
}


impl Error for TransportError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}


impl Display for TransportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Transport error: {}", self.describe)
    }
}



#[derive(Debug)]
pub struct TryfromSliceError {
    describe: String,
    boxed_error: Box<dyn Error>
}


impl TryfromSliceError {
    pub fn new<S: ToString>(des: S, err: impl Error + 'static) -> Self {
        Self {
            describe: des.to_string(),
            boxed_error: Box::new(err)
        }
    }
}


impl Error for TryfromSliceError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}


impl Display for TryfromSliceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TryfromSliceError error: {}", self.describe)
    }
}



#[derive(Debug)]
pub struct BadMessageError {
    describe: String,
    #[allow(dead_code)]
    boxed_error: Box<dyn Error>
}


impl BadMessageError {
    pub fn new<S: ToString>(des: S, err: impl Error + 'static) -> Self {
        Self {
            describe: des.to_string(),
            boxed_error: Box::new(err)
        }
    }
}


impl Error for BadMessageError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}


impl Display for BadMessageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "BadMessage error: {}", self.describe)
    }
}