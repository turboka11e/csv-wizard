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
    views::{Dialog, DialogFocus, DummyView, LinearLayout, Panel, SelectView, TextArea, TextView},
    Cursive,
};
use transform::{Options, Transformer};
use utils::{get_headers_from_file, select_directory, select_file};

fn main() {
    // Creates the cursive root - required for every application.
    let mut siv = cursive::default();
    siv.set_window_title("CSV Wizard");
    siv.add_global_callback('q', |s| s.quit());

    select_file_and_directory_display(&mut siv, None);

    // Starts the event loop.
    siv.run();
}

/// Start screen
fn select_file_and_directory_display(siv: &mut Cursive, file_paths: Option<(String, String)>) {
    let (input_path, output_path) = match file_paths {
        Some((input, output)) => (input, output),
        None => ("".to_string(), "".to_string()),
    };

    siv.pop_layer();
    let select_file_and_directory_dialog = Dialog::around(
        LinearLayout::vertical()
            .child(DummyView)
            .child(TextView::new(
                "Split CSV-file by category and filter (optional).\nOutput files are in format CSV and Excel.",
            ))
            .child(DummyView)
            .child(TextView::new("Note:"))
            .child(TextView::new("Dates are automatically recognized with following format: \"d.m.yyyy hh:mm:ss\""))
            .child(TextView::new("For example: \"1.3.2022 14:23:22\""))
            .child(TextView::new("This will allow dates to be correctly formatted in excel files."))
            .child(DummyView)
            .child(
                LinearLayout::horizontal()
                    .child(TextView::new(" Input:  ").style(Effect::Bold))
                    .child(TextView::new(input_path).with_name("input").min_width(30)),
            )
            .child(
                LinearLayout::horizontal()
                    .child(TextView::new("Output:  ").style(Effect::Bold))
                    .child(TextView::new(output_path).with_name("output").min_width(30)),
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
    .button("Next", |s| {
        let input_path = s
            .call_on_name("input", |view: &mut TextView| view.get_content())
            .unwrap();
        let output_path = s
            .call_on_name("output", |view: &mut TextView| view.get_content())
            .unwrap();

        if input_path.source().is_empty() || output_path.source().is_empty() {
            return s.add_layer(Dialog::info("Input or output missing."));
        }

        select_category_display(s, input_path.source().to_string(), output_path.source().to_string())
    }
    )
    .button("Quit", |s| s.quit());
    siv.add_layer(select_file_and_directory_dialog);
}

/// Select Category Display
///
/// Reads file for headers. Allows user to select a category.
fn select_category_display(s: &mut Cursive, input_path: String, output_path: String) {
    let mut select = SelectView::new()
        // Center the text horizontally
        .h_align(HAlign::Center)
        // Use keyboard to jump to the pressed letters
        .autojump();

    let headers = match get_headers_from_file(&PathBuf::from(&input_path)) {
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

    let file_paths = Some((input_path.clone(), output_path.clone()));

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
                                PathBuf::from(input_path.clone()),
                                PathBuf::from(output_path.clone()),
                                None,
                            );

                            select_filter_display(s, options, headers.clone());
                        })
                        .scrollable(),
                ),
        )
        .title("Configuration")
        .button("Back", move |s| {
            select_file_and_directory_display(s, file_paths.clone())
        }),
    );
}

/// Select filter display
fn select_filter_display(s: &mut Cursive, options: Options, headers: StringRecord) {
    let (back_options, skip_options, skip_headers) =
        (options.clone(), options.clone(), headers.clone());
    let mut select = SelectView::new()
        // Center the text horizontally
        .h_align(HAlign::Center)
        // Use keyboard to jump to the pressed letters
        .autojump();

    headers
        .clone()
        .into_iter()
        .for_each(|s| select.add_item(s.clone(), s.to_string()));

    let mut select_dialog = Dialog::around(
        LinearLayout::vertical()
            .child(DummyView)
            .child(TextView::new(format!(
                "Selected category: {}",
                options.get_selected_category()
            )))
            .child(DummyView)
            .child(TextView::new("Select a filter (optional):").style(Effect::Bold))
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
    .button("Back", move |s| {
        select_category_display(
            s,
            back_options.input.to_str().unwrap().to_string(),
            back_options.output.to_str().unwrap().to_string(),
        )
    })
    .button("Next without filter", move |s| {
        overview_display(s, skip_options.clone(), skip_headers.clone());
    })
    .button("Next with filter", move |s| {
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
        overview_display(s, options, headers.clone())
    })
    .title("Configuration");

    select_dialog.set_focus(DialogFocus::Button(3));

    s.pop_layer();
    s.add_layer(select_dialog);
}

/// Overview display
fn overview_display(siv: &mut Cursive, options: Options, headers: StringRecord) {
    let mut overview = LinearLayout::vertical()
        .child(DummyView)
        .child(TextView::new(format!(
            "Category: {}",
            options.get_selected_category()
        )));

    if let Some((filter_field, filter_value)) = options.get_filter() {
        overview = overview.child(TextView::new(format!(
            "Filter: '{}' equals to '{}'",
            filter_field, filter_value
        )))
    }

    let (back_options, back_headers) = (options.clone(), headers.clone());

    let mut dialog = Dialog::around(overview)
        .title("Overview")
        .button("Back", move |s| {
            select_filter_display(s, back_options.clone(), back_headers.clone())
        })
        .button("Abort", |s| s.quit())
        .button("Execute", move |s| {
            execute(s, options.clone(), headers.clone())
        })
        .h_align(HAlign::Right);

    dialog.set_focus(DialogFocus::Button(2));

    siv.pop_layer();
    siv.add_layer(dialog);
}

fn execute(s: &mut Cursive, options: Options, headers: StringRecord) {
    progress_display(s);

    let transformer = Arc::new(Mutex::new(Transformer::new(
        s.cb_sink().clone(),
        options,
        headers,
    )));

    std::thread::spawn(move || {
        let mut transformer = transformer.lock().unwrap();
        let file_paths = transformer.get_input_output_path();
        match transformer.execute() {
            Ok(stats) => transformer
                .sink
                .send(Box::new(move |s: &mut Cursive| {
                    finished_display(s, stats, file_paths);
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

/// Will be displayed during execution. Content will be updated within [`Transformer`].
fn progress_display(s: &mut Cursive) {
    s.pop_layer();
    s.add_layer(
        Dialog::new()
            .title("Execution")
            .content(TextView::new("").with_name("running").min_width(15)),
    );
}

/// Finished display
fn finished_display(
    s: &mut Cursive,
    (categories_total, csv_lines, csv_wl, excel_files): (i32, i32, i32, i32),
    file_paths: Option<(String, String)>,
) {
    s.pop_layer();
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
        .button("New", move |s| {
            select_file_and_directory_display(s, file_paths.clone())
        })
        .button("Close", |s| s.quit()),
    );
}
