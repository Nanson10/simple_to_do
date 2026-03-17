use crate::date_utils::current_local_date;
use crate::types::{PendingTask, Task};
use chrono::NaiveDate;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

const DATA_DIR: &str = "data";
const TODO_FILE_NAME: &str = "to-do.txt";

pub fn ensure_data_dir() -> io::Result<()> {
    fs::create_dir_all(DATA_DIR)
}

pub fn read_tasks_for_day(date: &str) -> io::Result<Vec<Task>> {
    let path = day_file_path(date);
    read_tasks_from_file(&path)
}

pub fn write_tasks_for_day(date: &str, tasks: &[Task]) -> io::Result<()> {
    let path = day_file_path(date);
    write_tasks_to_file(&path, tasks)
}

pub fn list_day_files() -> io::Result<Vec<String>> {
    let mut dates = Vec::new();

    for entry in fs::read_dir(data_dir_path())? {
        let entry = entry?;
        let path = entry.path();

        if !path.is_file() {
            continue;
        }

        let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };

        if file_name == TODO_FILE_NAME || !file_name.ends_with(".txt") {
            continue;
        }

        let date = file_name.trim_end_matches(".txt");
        if is_valid_date_string(date) {
            dates.push(date.to_string());
        }
    }

    dates.sort();
    dates.reverse();
    Ok(dates)
}

pub fn collect_pending_tasks() -> io::Result<Vec<PendingTask>> {
    let days = list_day_files()?;
    let today = current_local_date();
    let mut pending = Vec::new();

    for day in days {
        let Ok(day_date) = NaiveDate::parse_from_str(&day, "%Y-%m-%d") else {
            continue;
        };

        if day_date > today {
            continue;
        }

        let tasks = read_tasks_for_day(&day)?;
        for (index, task) in tasks.iter().enumerate() {
            if !task.done && !task.cancelled {
                pending.push(PendingTask {
                    date: day.clone(),
                    index_in_day: index,
                    text: task.text.clone(),
                });
            }
        }
    }

    Ok(pending)
}

pub fn rebuild_todo_file() -> io::Result<()> {
    let pending_tasks = collect_pending_tasks()?;
    let path = todo_file_path();
    let mut file = File::create(path)?;

    writeln!(file, "# Unfinished tasks (auto-generated)")?;
    writeln!(file, "# Edit daily files through the app for consistency")?;
    writeln!(file)?;

    if pending_tasks.is_empty() {
        writeln!(file, "No unfinished tasks.")?;
        return Ok(());
    }

    for (display_index, task) in pending_tasks.iter().enumerate() {
        writeln!(file, "{}. [{}] {}", display_index + 1, task.date, task.text)?;
    }

    Ok(())
}

fn data_dir_path() -> PathBuf {
    PathBuf::from(DATA_DIR)
}

fn day_file_path(date: &str) -> PathBuf {
    data_dir_path().join(format!("{}.txt", date))
}

fn todo_file_path() -> PathBuf {
    data_dir_path().join(TODO_FILE_NAME)
}

fn parse_task_line(line: &str) -> Option<Task> {
    let trimmed = line.trim();
    if let Some(rest) = trimmed.strip_prefix("[ ] ") {
        return Some(Task {
            text: rest.to_string(),
            done: false,
            cancelled: false,
        });
    }

    if let Some(rest) = trimmed.strip_prefix("[x] ") {
        return Some(Task {
            text: rest.to_string(),
            done: true,
            cancelled: false,
        });
    }

    if let Some(rest) = trimmed.strip_prefix("[~] ") {
        return Some(Task {
            text: rest.to_string(),
            done: false,
            cancelled: true,
        });
    }

    None
}

fn format_task_line(task: &Task) -> String {
    if task.done {
        format!("[x] {}", task.text)
    } else if task.cancelled {
        format!("[~] {}", task.text)
    } else {
        format!("[ ] {}", task.text)
    }
}

fn read_tasks_from_file(path: &Path) -> io::Result<Vec<Task>> {
    if !path.exists() {
        return Ok(Vec::new());
    }

    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut tasks = Vec::new();

    for line in reader.lines() {
        let line = line?;
        if let Some(task) = parse_task_line(&line) {
            tasks.push(task);
        }
    }

    Ok(tasks)
}

fn write_tasks_to_file(path: &Path, tasks: &[Task]) -> io::Result<()> {
    let mut file = File::create(path)?;
    writeln!(file, "# Tasks")?;
    writeln!(file, "# Format: [ ] pending, [x] completed, [~] cancelled")?;
    writeln!(file)?;

    for task in tasks {
        writeln!(file, "{}", format_task_line(task))?;
    }

    Ok(())
}

fn is_valid_date_string(text: &str) -> bool {
    NaiveDate::parse_from_str(text, "%Y-%m-%d").is_ok()
}
