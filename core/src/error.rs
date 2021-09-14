use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub struct DumbError;
impl fmt::Display for DumbError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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
}

impl fmt::Display for DeserializeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Deserialize error: {}", self.describe)
    }
}