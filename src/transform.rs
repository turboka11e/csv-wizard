use csv::{StringRecord, WriterBuilder};
use std::{collections::HashMap, error::Error, fs::File, path::PathBuf, sync::MutexGuard, vec};

use cursive::{reexports::crossbeam_channel::Sender, views::TextView, Cursive};
use xlsxwriter::Workbook;

use crate::{
    errors::HeaderError,
    utils::{replace_all_invalid_characters, Header},
};

pub fn get_headers_from_file(input_path: PathBuf) -> Result<Vec<String>, Box<dyn Error>> {
    if let Ok(mut rdr) = csv::ReaderBuilder::new()
        .delimiter(b';')
        .from_path(input_path)
    {
        if let Ok(headers) = rdr.headers() {
            return Ok(headers.into_iter().map(|s| s.to_string()).collect());
        }
    }
    Err(Box::new(HeaderError))
}

pub fn iterate_over_csv_file(
    s: &MutexGuard<Sender<Box<dyn FnOnce(&mut Cursive) + Send>>>,
    input_path: &PathBuf,
    output_path: &PathBuf,
    category: &String,
) -> Result<(i32, i32, i32), Box<dyn Error>> {
    let mut categories: HashMap<String, (csv::Writer<File>, Vec<StringRecord>, Workbook)> =
        HashMap::new();
    let mut rows = 0;
    let mut excel_lines = 0;
    let mut categories_total = 0;
    if let Ok(mut rdr) = csv::ReaderBuilder::new()
        .delimiter(b';')
        .from_path(input_path)
    {
        let category_idx = rdr.get_field(&category)?;
        let headers = rdr.headers().cloned()?;
        for row in rdr.records() {
            // The iterator yields Result<StringRecord, Error>, so we check the
            // error here.;
            rows += 1;

            s.send(Box::new(move |s: &mut Cursive| {
                s.call_on_name("running", |view: &mut TextView| {
                    view.set_content("CSV lines read: ".to_string() + &rows.to_string())
                });
            }))
            .unwrap();

            let row = row?;
            if let Some(cat_field) = row.get(category_idx) {
                if !categories.contains_key(&cat_field.to_lowercase()) {
                    categories_total += 1;
                    let mut path_csv = output_path.clone();
                    let mut path_xlsx = output_path.clone();
                    let valid_cat_name = replace_all_invalid_characters(cat_field);

                    path_csv.push(valid_cat_name.clone() + ".csv");
                    path_xlsx.push(valid_cat_name + ".xlsx");

                    let mut wtr = WriterBuilder::new()
                        .delimiter(b';')
                        .from_path(path_csv)?;
                    wtr.write_record(&headers)?;

                    categories.entry(cat_field.to_string().to_lowercase()).or_insert((
                        wtr,
                        vec![headers.clone()],
                        Workbook::new(path_xlsx.to_str().unwrap()),
                    ));
                }

                let (wtr, records, _) = categories.get_mut(&cat_field.to_lowercase()).unwrap();
                records.push(row.clone());
                wtr.write_record(&row)?;
                wtr.flush()?;
            } else {
                return Err(Box::new(HeaderError));
            }
        }

        for (_, (_, records, workbook)) in categories.into_iter() {
            // let cat_name = transform_to_valid(&key);

            match workbook.add_worksheet(None) {
                Ok(mut worksheet) => {
                    for (row, record) in records.into_iter().enumerate() {
                        if row != 0 {
                            excel_lines += 1;
                        }
                        s.send(Box::new(move |s: &mut Cursive| {
                            s.call_on_name("running", |view: &mut TextView| {
                                view.set_content(
                                    "Excel lines added: ".to_string() + &excel_lines.to_string(),
                                )
                            });
                        }))
                        .unwrap();
                        for (col, field) in record.iter().enumerate() {
                            worksheet
                                .write_string(row as u32, col as u16, field, None)
                                .unwrap();
                        }
                    }

                    workbook.close().unwrap();
                }
                Err(error) => return Err(Box::new(error)),
            }
        }
    }

    Ok((rows, excel_lines, categories_total))
}
