use std::{error::Error, fmt};

#[derive(Debug, Clone)]
pub struct HeaderError;

impl fmt::Display for HeaderError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Invalid header!")
    }
}

impl Error for HeaderError {}
