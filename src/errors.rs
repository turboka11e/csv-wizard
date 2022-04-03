use std::{error::Error, fmt};

#[derive(Debug, Clone)]
pub struct HeaderError;

impl fmt::Display for HeaderError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Invalid header!")
    }
}

impl Error for HeaderError {}

#[derive(Debug, Clone)]
pub struct DirectoryError;

impl fmt::Display for DirectoryError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "directory already exist!\n
            Choose different output folder or remove\n
            folder with same name as selected options."
        )
    }
}

impl Error for DirectoryError {}
