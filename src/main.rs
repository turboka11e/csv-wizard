mod errors;
mod transform;
mod utils;

use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use csv::StringRecord;
use cursive::{
    align::HAlign,
    theme::Effect,
    traits::{Nameable, Resizable, Scrollable},
    views::{Dialog, DummyView, LinearLayout, Panel, SelectView, TextArea, TextView},
    Cursive,
};
use transform::{Options, Transformer};
use utils::{get_headers_from_file, select_directory, select_file};

fn main() {
    // Creates the cursive root - required for every application.
    let mut siv = cursive::default();
    siv.set_window_title("CSV Wizard");
    siv.add_global_callback('q', |s| s.quit());

    select_file_and_directory(&mut siv);

    // Starts the event loop.
    siv.run();
}

fn select_file_and_directory(siv: &mut Cursive) {
    siv.add_layer(
        Dialog::around(
            LinearLayout::vertical()
                .child(DummyView)
                .child(TextView::new(
                    "Split CSV-file by category and filter (optional).",
                ))
                .child(TextView::new("Output files are in format CSV and Excel."))
                .child(DummyView)
                .child(
                    LinearLayout::horizontal()
                        .child(TextView::new(" Input:  ").style(Effect::Bold))
                        .child(TextView::new("").with_name("input").min_width(30)),
                )
                .child(
                    LinearLayout::horizontal()
                        .child(TextView::new("Output:  ").style(Effect::Bold))
                        .child(TextView::new("").with_name("output").min_width(30)),
                ),
        )
        .title("CSV Wizard")
        .button("Input file", |s| match select_file() {
            Ok(input_path) => s
                .call_on_name("input", |view: &mut TextView| {
                    view.set_content(input_path.to_str().unwrap())
                })
                .unwrap(),
            Err(err) => s.add_layer(Dialog::info(err)),
        })
        .button("Output folder", |s| match select_directory() {
            Ok(input_path) => s
                .call_on_name("output", |view: &mut TextView| {
                    view.set_content(input_path.to_str().unwrap())
                })
                .unwrap(),
            Err(err) => s.add_layer(Dialog::info(err)),
        })
        .button("Next", |s| select_category(s))
        .button("Quit", |s| s.quit()),
    );
}

/// Select Category Display
///
fn select_category(s: &mut Cursive) {
    let input_path = s
        .call_on_name("input", |view: &mut TextView| view.get_content())
        .unwrap();
    let output_path = s
        .call_on_name("output", |view: &mut TextView| view.get_content())
        .unwrap();

    if input_path.source().is_empty() || output_path.source().is_empty() {
        return s.add_layer(Dialog::info("Input or output missing."));
    }

    let mut select = SelectView::new()
        // Center the text horizontally
        .h_align(HAlign::Center)
        // Use keyboard to jump to the pressed letters
        .autojump();

    let headers = match get_headers_from_file(&PathBuf::from(&input_path.source())) {
        Ok(iter) => iter,
        Err(error) => {
            return s.add_layer(
                Dialog::text(format!("Failed with {}", error.to_string()))
                    .title("Error")
                    .button("Close", |s| s.quit()),
            )
        }
    };

    headers
        .clone()
        .iter()
        .for_each(|s| select.add_item(s.to_string(), s.to_string()));

    s.pop_layer();
    s.add_layer(
        Dialog::around(
            LinearLayout::vertical()
                .child(DummyView)
                .child(TextView::new("Select a category:").style(Effect::Bold))
                .child(DummyView)
                .child(
                    select
                        .on_submit(move |s, selected_category: &str| {
                            // Show a popup whenever the user presses <Enter>
                            let options = Options::new(
                                selected_category.to_string(),
                                PathBuf::from(input_path.source()),
                                PathBuf::from(output_path.source()),
                                None,
                            );

                            select_filter(s, options, headers.clone());
                        })
                        .scrollable(),
                ),
        )
        .title("Configuration"),
    );
}

fn select_filter(s: &mut Cursive, options: Options, headers: StringRecord) {
    let (skip_options, skip_headers) = (options.clone(), headers.clone());
    let mut select = SelectView::new()
        // Center the text horizontally
        .h_align(HAlign::Center)
        // Use keyboard to jump to the pressed letters
        .autojump();

    headers
        .clone()
        .into_iter()
        .for_each(|s| select.add_item(s.clone(), s.to_string()));

    s.pop_layer();
    s.add_layer(
        Dialog::around(
            LinearLayout::vertical()
                .child(DummyView)
                .child(TextView::new("Select a filter:").style(Effect::Bold))
                .child(DummyView)
                .child(
                    LinearLayout::horizontal()
                        .child(
                            Panel::new(select.with_name("filterView").scrollable())
                                .title("Field")
                                .fixed_width(20),
                        )
                        .child(
                            LinearLayout::vertical()
                                .child(TextView::new("Equals to").center().min_width(20))
                                .child(TextArea::new().with_name("filterEquals")),
                        ),
                ),
        )
        .button("Skip", move |s| {
            execute(s, skip_options.clone(), skip_headers.clone())
        })
        .button("Next", move |s| {
            let mut options = options.clone();

            options.set_filter(Some((
                s.call_on_name("filterView", |view: &mut SelectView| {
                    view.selection().unwrap().to_string()
                })
                .unwrap(),
                s.call_on_name("filterEquals", |view: &mut TextArea| {
                    view.get_content().to_string()
                })
                .unwrap(),
            )));
            execute(s, options, headers.clone())
            // execute(s, options)
        })
        .title("Configuration"),
    );
}

fn execute(s: &mut Cursive, options: Options, headers: StringRecord) {
    s.pop_layer();
    s.add_layer(
        Dialog::new()
            .title("Execution")
            .content(TextView::new("").with_name("running").min_width(15)),
    );
    let transformer = Arc::new(Mutex::new(Transformer::new(
        s.cb_sink().clone(),
        options,
        headers,
    )));

    std::thread::spawn(move || {
        let mut transformer = transformer.lock().unwrap();
        match transformer.execute() {
            Ok((csv_lines, csv_wl, excel_files, categories_total)) => transformer
                .sink
                .send(Box::new(move |s: &mut Cursive| {
                    s.add_layer(
                        Dialog::around(
                            LinearLayout::vertical()
                                .child(TextView::new("Finished."))
                                .child(DummyView)
                                .child(TextView::new(format!(
                                    "Categories:          {}",
                                    categories_total
                                )))
                                .child(TextView::new(format!("CSV lines read:      {}", csv_lines)))
                                .child(TextView::new(format!("CSV lines written:   {}", csv_wl)))
                                .child(TextView::new(format!(
                                    "Excel lines written: {}",
                                    excel_files
                                ))),
                        )
                        .title("Success")
                        .button("Close", |s| s.quit()),
                    );
                }))
                .unwrap(),
            Err(error) => {
                let error = error.to_string();
                transformer
                    .sink
                    .send(Box::new(move |s: &mut Cursive| {
                        s.add_layer(
                            Dialog::text(format!("Failed with {}", error))
                                .title("Error")
                                .button("Close", |s| s.quit()),
                        );
                    }))
                    .unwrap()
            }
        };
    });
}
