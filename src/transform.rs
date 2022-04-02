use csv::{StringRecord, WriterBuilder};
use std::{collections::HashMap, error::Error, fs::File, path::PathBuf, vec};

use cursive::{views::TextView, CbSink, Cursive};
use xlsxwriter::Workbook;

use crate::{
    errors::HeaderError,
    utils::{replace_all_invalid_characters, Header},
    Options,
};

pub struct Transformer {
    pub sink: CbSink,
    options: Options,
    headers: StringRecord,
}

impl Transformer {
    pub fn new(sink: CbSink, options: Options, headers: StringRecord) -> Transformer {
        Transformer {
            sink,
            options,
            headers,
        }
    }

    fn write_to_running_view(&mut self, text: String) {
        self.sink
            .send(Box::new(move |s: &mut Cursive| {
                s.call_on_name("running", |view: &mut TextView| {
                    view.set_content(text);
                });
            }))
            .unwrap();
    }

    pub fn execute(&mut self) -> Result<(i32, i32, i32, i32), Box<dyn Error>> {
        let (mut csv_rl, mut csv_wl, mut excel_wl, mut cat_total) = (0, 0, 0, 0);
        let mut categories: HashMap<String, Vec<StringRecord>> = HashMap::new();

        if let Ok(mut rdr) = csv::ReaderBuilder::new()
            .delimiter(b';')
            .from_path(&self.options.input)
        {
            let category_idx = rdr.get_field(&self.options.selected_category)?;

            for record in rdr.records() {
                csv_rl += 1;
                let record = record?;
                self.write_to_running_view(format!("CSV lines read {}", csv_rl));

                if let Some(cat_field) = record.get(category_idx) {
                    if !categories.contains_key(&cat_field.to_lowercase()) {
                        cat_total += 1;
                        categories
                            .entry(cat_field.to_string().to_lowercase())
                            .or_insert(vec![self.headers.clone()]);
                    }
                    categories
                        .get_mut(&cat_field.to_string().to_lowercase())
                        .unwrap()
                        .push(record);
                }
            }

            // CSV

            for (category, records) in categories.into_iter() {
                let (path_csv, path_xlsx) = self.get_csv_xlsx_path(category);

                //// WRITE CSV

                let mut wtr = WriterBuilder::new()
                    .delimiter(b';')
                    .from_path(path_csv)?;
                
                for record in records.iter() {
                    csv_wl += 1;
                    self.write_to_running_view(format!("CSV lines added: {}", csv_wl));
                    wtr.write_record(record)?;
                }
                csv_wl -= 1; // account for header
                //// WRITE EXCEL

                let workbook = Workbook::new(path_xlsx.to_str().unwrap());

                match workbook.add_worksheet(None) {
                    Ok(mut worksheet) => {
                        for (row, record) in records.into_iter().enumerate() {
                            excel_wl += 1;
                            self.write_to_running_view(format!("Excel lines added: {}", excel_wl));
                            for (col, field) in record.iter().enumerate() {
                                worksheet
                                    .write_string(row as u32, col as u16, field, None)
                                    .unwrap();
                            }
                        }
                        excel_wl -= 1; // account for header
                        workbook.close().unwrap();
                    }
                    Err(error) => return Err(Box::new(error)),
                }
            }
        }

        Ok((csv_rl as i32, csv_wl, excel_wl, cat_total as i32))
    }

    fn get_csv_xlsx_path(&mut self, category: String) -> (PathBuf, PathBuf) {
        let (mut path_csv, mut path_xlsx) =
            (self.options.output.clone(), self.options.output.clone());
        let valid_cat_name = replace_all_invalid_characters(&category);
        path_csv.push(valid_cat_name.clone() + ".csv");
        path_xlsx.push(valid_cat_name + ".xlsx");
        (path_csv, path_xlsx)
    }
}

pub fn get_headers_from_file(file: &PathBuf) -> Result<StringRecord, Box<dyn Error>> {
    if let Ok(mut rdr) = csv::ReaderBuilder::new().delimiter(b';').from_path(file) {
        return Ok(rdr.headers().cloned()?);
    }
    Err(Box::new(HeaderError))
}
