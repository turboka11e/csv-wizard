use csv::{StringRecord, WriterBuilder};
use std::{collections::HashMap, error::Error, path::PathBuf, vec};

use cursive::{views::TextView, CbSink, Cursive};
use xlsxwriter::Workbook;

use crate::{
    errors::DirectoryError,
    utils::{replace_all_invalid_characters, try_parse_time, Header},
};

pub struct Transformer {
    pub sink: CbSink,
    options: Options,
    headers: StringRecord,
}

#[derive(Clone, Debug)]
pub struct Options {
    selected_category: String,
    pub input: PathBuf,
    pub output: PathBuf,
    filter: Option<(String, String)>,
}

/// (csv_rl, cat_total, HashMap<category_key.lowercase, ([Records], first_cat_name))
type CsvReadResult =
    Result<(i32, i32, HashMap<String, (Vec<StringRecord>, String)>), Box<dyn Error>>;

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

    /// Execute will read a csv and then write to files by category and filter (optional).
    ///
    /// The value in the Hashmap is (Records, first field name for that category) -> Background: Windows doesn't differentiate between upper and lowercase.
    /// Hence test.csv and Test.csv would overwrite each other and corrupt the result.
    ///
    /// Returns Ok(cat_total, csv_rl, csv_wl, excel_wl)
    pub fn execute(&mut self) -> Result<(i32, i32, i32, i32), Box<dyn Error>> {
        match csv::ReaderBuilder::new()
            .delimiter(b';')
            .from_path(&self.options.input)
        {
            Ok(rdr) => {
                let (csv_rl, cat_total, categories) = self.read_csv(rdr)?;

                let (mut csv_wl, mut excel_wl) = (0, 0);

                self.create_dir_for_csv_and_xslx()?;

                for (records, category_sub_collection) in categories.values() {
                    let (path_csv, path_xlsx) =
                        self.get_csv_xlsx_path(category_sub_collection.to_string());
                    self.write_csv(path_csv, records, &mut csv_wl)?;
                    self.write_xlsx(path_xlsx, records, &mut excel_wl)?;
                }

                Ok((cat_total as i32, csv_rl as i32, csv_wl, excel_wl))
            }
            Err(error) => Err(Box::new(error)),
        }
    }

    fn create_dir_for_csv_and_xslx(&mut self) -> Result<(), Box<dyn Error>> {
        self.options.output = self.options.output.join(match &self.options.filter {
            Some((filter_field, filter_value)) => replace_all_invalid_characters(&format!(
                "{}_{}_{}",
                self.options.selected_category, filter_field, filter_value
            )),
            None => replace_all_invalid_characters(&self.options.selected_category),
        });
        if self.options.output.exists() {
            Err(Box::new(DirectoryError))
        } else {
            std::fs::create_dir(self.options.output.as_path())?;
            Ok(())
        }
    }

    fn write_xlsx(
        &mut self,
        path_xlsx: PathBuf,
        records: &[StringRecord],
        excel_wl: &mut i32,
    ) -> Result<(), Box<dyn Error>> {
        let workbook = Workbook::new(path_xlsx.to_str().unwrap());
        let date_format = Some(workbook.add_format().set_num_format("dd.mm.yyyy hh:mm:ss"));
        match workbook.add_worksheet(None) {
            Ok(mut worksheet) => {
                for (row, record) in records.iter().enumerate() {
                    *excel_wl += 1;
                    self.write_to_running_view(format!("Excel lines added: {}", excel_wl));

                    for (col, field) in record.iter().enumerate() {
                        match try_parse_time(field) {
                            Ok(datetime) => worksheet.write_datetime(
                                row as u32,
                                col as u16,
                                &datetime,
                                date_format.as_ref(),
                            )?,
                            Err(_) => {
                                worksheet.write_string(row as u32, col as u16, field, None)?
                            }
                        };
                    }
                }
                *excel_wl -= 1; // account for header
                workbook.close()?;
                Ok(())
            }
            Err(error) => Err(Box::new(error)),
        }
    }

    fn write_csv(
        &mut self,
        path_csv: PathBuf,
        records: &[StringRecord],
        csv_wl: &mut i32,
    ) -> Result<(), Box<dyn Error>> {
        let mut wtr = WriterBuilder::new().delimiter(b';').from_path(path_csv)?;
        for record in records.iter() {
            *csv_wl += 1;
            self.write_to_running_view(format!("CSV lines added: {}", csv_wl));
            wtr.write_record(record)?;
        }
        *csv_wl -= 1;
        Ok(())
    }

    fn read_csv(&mut self, mut rdr: csv::Reader<std::fs::File>) -> CsvReadResult {
        let mut categories: HashMap<String, (Vec<StringRecord>, String)> = HashMap::new();
        let (mut csv_rl, mut cat_total) = (0, 0);
        let category_idx = rdr.get_field(&self.options.selected_category)?;
        let mut filter_option = None;
        if let Some((filter_field, filter_value)) = self.options.filter.clone() {
            filter_option = Some((rdr.get_field(&filter_field)?, filter_value));
        }
        for record in rdr.records() {
            let record = record?;

            if let Some((field_idx, filter_name)) = &filter_option {
                let value = record.get(*field_idx).unwrap();
                if value != filter_name {
                    continue;
                }
            };

            csv_rl += 1;
            self.write_to_running_view(format!("CSV lines read {}", csv_rl));

            if let Some(cat_field) = record.get(category_idx) {
                let cat_field_key = cat_field.to_string().to_lowercase();
                if !categories.contains_key(&cat_field.to_lowercase()) {
                    cat_total += 1;
                    categories
                        .entry(cat_field_key.clone())
                        .or_insert((vec![self.headers.clone()], cat_field.to_string()));
                }
                categories.get_mut(&cat_field_key).unwrap().0.push(record);
            }
        }
        Ok((csv_rl, cat_total, categories))
    }

    /// Creates the file paths for csv and xlsx.
    fn get_csv_xlsx_path(&mut self, mut category_sub_collection: String) -> (PathBuf, PathBuf) {
        let (mut path_csv, mut path_xlsx) =
            (self.options.output.clone(), self.options.output.clone());
        if category_sub_collection.is_empty() {
            category_sub_collection = format!("Empty_{}", self.options.selected_category)
        }
        let valid_cat_name = replace_all_invalid_characters(&category_sub_collection);
        path_csv.push(valid_cat_name.clone() + ".csv");
        path_xlsx.push(valid_cat_name + ".xlsx");
        (path_csv, path_xlsx)
    }

    pub fn get_input_output_path(&self) -> Option<(String, String)> {
        let input = self.options.input.to_str().unwrap().to_string();
        let output = self.options.output.to_str().unwrap().to_string();
        Some((input, output))
    }
}

impl Options {
    pub fn new(
        selected_category: String,
        input: PathBuf,
        output: PathBuf,
        filter: Option<(String, String)>,
    ) -> Options {
        Options {
            selected_category,
            input,
            output,
            filter,
        }
    }

    pub fn set_filter(&mut self, filter: Option<(String, String)>) -> Self {
        self.filter = filter;
        self.to_owned()
    }

    pub fn get_selected_category(&self) -> String {
        self.selected_category.clone()
    }

    pub fn get_filter(&self) -> Option<(String, String)> {
        self.filter.clone()
    }
}
