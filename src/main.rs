mod commands;
mod date_utils;
mod storage;
mod types;
mod ui;

fn main() {
    if let Err(error) = commands::run() {
        eprintln!("Error: {error}");
    }
}
