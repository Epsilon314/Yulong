use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub struct DumbError;
impl fmt::Display for DumbError {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        unimplemented!("dumb error")
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

    #[allow(dead_code)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Deserialize error: {}", self.describe)
    }
}

pub struct TransportError {
    describe: String,
    #[allow(dead_code)]
    boxed_error: Box<dyn Error>
}

impl TransportError {
    pub fn new<S: ToString>(des: S, err: impl Error + 'static) -> Self {
        Self {
            describe: des.to_string(),
            boxed_error: Box::new(err)
        }
    }

    #[allow(dead_code)]
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

    #[allow(dead_code)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TryfromSliceError error: {}", self.describe)
    }
}