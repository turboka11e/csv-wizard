use std::{path::PathBuf, fs::File, error::Error};
use csv::Reader;
use native_dialog::FileDialog;

use crate::errors::HeaderError;

const INVALID_CHARS: [char; 14] = ['$', '%', '^', '*', '/', ' ', '.', ':', '<', '>', '"', '\\', '|', '?'];

pub fn select_file() -> Result<PathBuf, String> {
    // let path = std::env::current_exe();
    // println!("{:?}", path.unwrap().as_os_str());
    match FileDialog::new()
        .set_location("~/Downloads")
        .add_filter("CSV File", &["csv"])
        .show_open_single_file()
    {
        Ok(path) => match path {
            Some(path) => Ok(path),
            None => Err("File not found".to_string()),
        },
        Err(error) => Err(error.to_string()),
    }
}

pub fn select_directory() -> Result<PathBuf, String> {
    match FileDialog::new().show_open_single_dir() {
        Ok(path) => match path {
            Some(path) => Ok(path),
            None => Err("Directory not found".to_string()),
        },
        Err(error) => Err(error.to_string()),
    }
}

pub trait Header {
    fn get_field(&mut self, field: &str) -> Result<usize, Box<dyn Error>>;
}

impl Header for Reader<File> {
    fn get_field(&mut self, name: &str) -> Result<usize, Box<dyn Error>> {
        match self
            .headers()?
            .iter()
            .position(|field| field.contains(&name))
        {
            Some(idx) => Ok(idx),
            None => Err(Box::new(HeaderError)),
        }
    }
}

pub fn replace_all_invalid_characters(field: &str) -> String {
    let mut field = String::from(field);
    INVALID_CHARS
        .iter()
        .for_each(|&c| field = field.replace(c, "_"));
    field
}