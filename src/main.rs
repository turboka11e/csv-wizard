mod errors;
mod transform;
mod utils;

use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use cursive::{
    align::HAlign,
    theme::Effect,
    traits::{Nameable, Resizable},
    views::{Dialog, DummyView, LinearLayout, SelectView, TextView},
    CbSink, Cursive,
};
use transform::{get_headers_from_file, iterate_over_csv_file};
use utils::{select_directory, select_file};

fn main() {
    // Creates the cursive root - required for every application.
    let mut siv = cursive::default();
    siv.set_window_title("CSV Wizard");
    siv.add_global_callback('q', |s| s.quit());

    // Creates a dialog with a single "Quit" button
    select_file_and_directory(&mut siv);

    // Starts the event loop.
    siv.run();
}

fn select_file_and_directory(siv: &mut Cursive) {
    siv.add_layer(
        Dialog::around(
            LinearLayout::vertical()
                .child(TextView::new("Split a file in mulitple files by category."))
                .child(DummyView)
                .child(
                    LinearLayout::horizontal()
                        .child(TextView::new(" Input:").min_width(18))
                        .child(TextView::new("").with_name("input")),
                )
                .child(
                    LinearLayout::horizontal()
                        .child(TextView::new("Output:").min_width(18))
                        .child(TextView::new("").with_name("output")),
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

    match get_headers_from_file(PathBuf::from(&input_path.source())) {
        Ok(iter) => iter.into_iter().for_each(|s| select.add_item(s.clone(), s)),
        Err(error) => {
            return s.add_layer(
                Dialog::text(format!("Failed with {}", error.to_string()))
                    .title("Error")
                    .button("Close", |s| s.quit()),
            )
        }
    }

    s.pop_layer();
    s.add_layer(
        Dialog::around(
            LinearLayout::vertical()
                .child(DummyView)
                .child(TextView::new("Select a category:").style(Effect::Bold))
                .child(DummyView)
                .child(select.on_submit(move |s, selected_category: &String| {
                    // Show a popup whenever the user presses <Enter>
                    select_filter(
                        s,
                        PathBuf::from(input_path.source()),
                        PathBuf::from(output_path.source()),
                        selected_category.clone(),
                    )
                })),
        )
        .title("Configuration"),
    );
}

fn select_filter(
    s: &mut Cursive,
    input_path: PathBuf,
    output_path: PathBuf,
    selected_category: String,
) {
    s.pop_layer();
    s.add_layer(
        Dialog::new()
            .title("Execution")
            .content(TextView::new("").with_name("running").min_width(15)),
    );
    let model = Arc::new(Mutex::new(s.cb_sink().clone()));
    execute(
        model,
        input_path.clone(),
        output_path.clone(),
        selected_category.clone(),
    )
}

fn execute(
    cb_sink: Arc<Mutex<CbSink>>,
    input_path: PathBuf,
    output_path: PathBuf,
    selected_category: String,
) {
    std::thread::spawn(move || {
        let cb_sink = cb_sink.lock().unwrap();

        match iterate_over_csv_file(&cb_sink, &input_path, &output_path, &selected_category) {
            Ok((csv_lines, excel_files, categories_total)) => cb_sink
                .send(Box::new(move |s: &mut Cursive| {
                    s.add_layer(
                        Dialog::around(
                            LinearLayout::vertical()
                                .child(TextView::new("Finished."))
                                .child(DummyView)
                                .child(TextView::new(format!(
                                    "Number of categories:          {}",
                                    categories_total
                                )))
                                .child(TextView::new(format!(
                                    "Number of csv lines read:      {}",
                                    csv_lines
                                )))
                                .child(TextView::new(format!(
                                    "Number of Excel lines written: {}",
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
                cb_sink
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
