use chrono::{Datelike, NaiveDateTime, Timelike};
use csv::{Reader, StringRecord};
use native_dialog::FileDialog;
use std::{error::Error, fs::File, path::PathBuf};

use crate::errors::HeaderError;

const INVALID_CHARS: [char; 14] = [
    '$', '%', '^', '*', '/', ' ', '.', ':', '<', '>', '"', '\\', '|', '?',
];

pub fn select_file() -> Result<PathBuf, String> {
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

pub fn get_headers_from_file(file: &PathBuf) -> Result<StringRecord, Box<dyn Error>> {
    if let Ok(mut rdr) = csv::ReaderBuilder::new().delimiter(b';').from_path(file) {
        return Ok(rdr.headers().cloned()?);
    }
    Err(Box::new(HeaderError))
}

pub fn try_parse_time(field: &str) -> Result<xlsxwriter::DateTime, ()> {
    match NaiveDateTime::parse_from_str(field, "%-d.%-m.%Y %H:%M:%S") {
        Ok(datetime) => {
            let d = datetime.date();
            let t = datetime.time();
            let (year, month, day, hour, minute, second) = (
                d.year() as i16,
                d.month() as i8,
                d.day() as i8,
                t.hour() as i8,
                t.minute() as i8,
                t.second(),
            );
            Ok(xlsxwriter::DateTime::new(
                year,
                month,
                day,
                hour,
                minute,
                second.into(),
            ))
        }
        Err(_) => Err(()),
    }
}
