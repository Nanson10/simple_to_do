mod app;
mod date_utils;
mod storage;
mod types;
mod ui;

fn main() {
    // Check for --test flag and set environment variable
    if std::env::args().any(|arg| arg == "--test") {
        unsafe {
            std::env::set_var("SIMPLE_TODO_TEST_MODE", "1");
        }
    }

    if let Err(error) = app::run() {
        eprintln!("Error: {error}");
    }
}
