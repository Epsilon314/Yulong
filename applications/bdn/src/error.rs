use std::error::Error;
use std::fmt::{self, Debug, Display};

use yulong::error::DumbError;

#[derive(Debug)]
pub struct BadFieldError {
    describe: String,
    boxed_error: Box<dyn Error>
}


impl BadFieldError {
    pub fn new<S: ToString>(des: S, err: impl Error + 'static) -> Self {
        Self {
            describe: des.to_string(),
            boxed_error: Box::new(err)
        }
    }
}


impl Error for BadFieldError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}


impl Display for BadFieldError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Bad field error: {}", self.describe)
    }
}