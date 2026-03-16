mod app;
mod date_utils;
mod storage;
mod types;
mod ui;

fn main() {
    if let Err(error) = app::run() {
        eprintln!("Error: {error}");
    }
}
