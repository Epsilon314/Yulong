use std::error::Error;
use std::fmt::{self, Debug, Display};


/// Errors happened in Transport trait
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


/// Errors happens dealing with identity
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