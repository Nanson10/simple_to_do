use crate::date_utils::current_local_date;
use crate::types::{PendingTask, Task};
use chrono::NaiveDate;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

const TODO_FILE_NAME: &str = "to-do.txt";

fn get_data_dir() -> &'static str {
    if std::env::var("SIMPLE_TODO_TEST_MODE").is_ok() {
        "test"
    } else {
        "data"
    }
}

pub fn ensure_data_dir() -> io::Result<()> {
    fs::create_dir_all(get_data_dir())
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
                    due_date: task.due_date.clone(),
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
        let with_due = format_text_with_due(&task.text, &task.due_date);
        writeln!(file, "{}. [{}] {}", display_index + 1, task.date, with_due)?;
    }

    Ok(())
}

fn data_dir_path() -> PathBuf {
    PathBuf::from(get_data_dir())
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
        let (text, due_date) = split_due_date(rest);
        return Some(Task {
            text,
            done: false,
            cancelled: false,
            due_date,
        });
    }

    if let Some(rest) = trimmed.strip_prefix("[x] ") {
        let (text, due_date) = split_due_date(rest);
        return Some(Task {
            text,
            done: true,
            cancelled: false,
            due_date,
        });
    }

    if let Some(rest) = trimmed.strip_prefix("[~] ") {
        let (text, due_date) = split_due_date(rest);
        return Some(Task {
            text,
            done: false,
            cancelled: true,
            due_date,
        });
    }

    None
}

fn format_task_line(task: &Task) -> String {
    if task.done {
        format!("[x] {}", format_text_with_due(&task.text, &task.due_date))
    } else if task.cancelled {
        format!("[~] {}", format_text_with_due(&task.text, &task.due_date))
    } else {
        format!("[ ] {}", format_text_with_due(&task.text, &task.due_date))
    }
}

fn split_due_date(text: &str) -> (String, Option<String>) {
    if let Some((task_text, due_text)) = text.rsplit_once(" | due: ") {
        if is_valid_date_string(due_text) {
            return (task_text.to_string(), Some(due_text.to_string()));
        }
    }

    (text.to_string(), None)
}

fn format_text_with_due(text: &str, due_date: &Option<String>) -> String {
    if let Some(due_date) = due_date {
        format!("{} | due: {}", text, due_date)
    } else {
        text.to_string()
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
    writeln!(
        file,
        "# Format: [ ] pending, [x] completed, [~] cancelled; optional suffix: | due: YYYY-MM-DD"
    )?;
    writeln!(file)?;

    for task in tasks {
        writeln!(file, "{}", format_task_line(task))?;
    }

    Ok(())
}

fn is_valid_date_string(text: &str) -> bool {
    NaiveDate::parse_from_str(text, "%Y-%m-%d").is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    // ============ parse_task_line tests ============

    #[test]
    fn test_parse_pending_task() {
        let task = parse_task_line("[ ] Buy groceries").unwrap();
        assert_eq!(task.text, "Buy groceries");
        assert!(!task.done);
        assert!(!task.cancelled);
    }

    #[test]
    fn test_parse_completed_task() {
        let task = parse_task_line("[x] File taxes").unwrap();
        assert_eq!(task.text, "File taxes");
        assert!(task.done);
        assert!(!task.cancelled);
    }

    #[test]
    fn test_parse_cancelled_task() {
        let task = parse_task_line("[~] Reschedule dentist").unwrap();
        assert_eq!(task.text, "Reschedule dentist");
        assert!(!task.done);
        assert!(task.cancelled);
    }

    #[test]
    fn test_parse_task_with_leading_whitespace() {
        let task = parse_task_line("  [ ] Task with spaces").unwrap();
        assert_eq!(task.text, "Task with spaces");
        assert!(!task.done);
        assert!(!task.cancelled);
    }

    #[test]
    fn test_parse_task_with_trailing_whitespace() {
        let task = parse_task_line("[ ] Task with spaces  ").unwrap();
        assert_eq!(task.text, "Task with spaces");
    }

    #[test]
    fn test_parse_invalid_prefix() {
        assert!(parse_task_line("(-) Invalid marker").is_none());
        assert!(parse_task_line("[!] Invalid marker").is_none());
    }

    #[test]
    fn test_parse_no_marker() {
        assert!(parse_task_line("Just text").is_none());
    }

    #[test]
    fn test_parse_empty_line() {
        assert!(parse_task_line("").is_none());
    }

    #[test]
    fn test_parse_task_with_special_characters() {
        let task = parse_task_line("[ ] Call Dr. Smith @ 3:00 PM").unwrap();
        assert_eq!(task.text, "Call Dr. Smith @ 3:00 PM");
    }

    #[test]
    fn test_parse_task_with_brackets_in_text() {
        let task = parse_task_line("[ ] Task [important] [urgent]").unwrap();
        assert_eq!(task.text, "Task [important] [urgent]");
    }

    #[test]
    fn test_parse_task_with_due_date_suffix() {
        let task = parse_task_line("[ ] Buy milk | due: 2026-03-21").unwrap();
        assert_eq!(task.text, "Buy milk");
        assert_eq!(task.due_date.as_deref(), Some("2026-03-21"));
        assert!(!task.done);
        assert!(!task.cancelled);
    }

    // ============ format_task_line tests ============

    #[test]
    fn test_format_pending_task() {
        let task = Task {
            text: "Buy milk".to_string(),
            done: false,
            cancelled: false,
            due_date: None,
        };
        assert_eq!(format_task_line(&task), "[ ] Buy milk");
    }

    #[test]
    fn test_format_completed_task() {
        let task = Task {
            text: "File taxes".to_string(),
            done: true,
            cancelled: false,
            due_date: None,
        };
        assert_eq!(format_task_line(&task), "[x] File taxes");
    }

    #[test]
    fn test_format_cancelled_task() {
        let task = Task {
            text: "Reschedule meeting".to_string(),
            done: false,
            cancelled: true,
            due_date: None,
        };
        assert_eq!(format_task_line(&task), "[~] Reschedule meeting");
    }

    #[test]
    fn test_format_done_and_cancelled_prefers_done() {
        // If both are true, should prefer done marker
        let task = Task {
            text: "Task".to_string(),
            done: true,
            cancelled: true,
            due_date: None,
        };
        assert_eq!(format_task_line(&task), "[x] Task");
    }

    #[test]
    fn test_format_task_empty_text() {
        let task = Task {
            text: "".to_string(),
            done: false,
            cancelled: false,
            due_date: None,
        };
        assert_eq!(format_task_line(&task), "[ ] ");
    }

    #[test]
    fn test_format_task_with_due_date() {
        let task = Task {
            text: "Buy milk".to_string(),
            done: false,
            cancelled: false,
            due_date: Some("2026-03-21".to_string()),
        };
        assert_eq!(format_task_line(&task), "[ ] Buy milk | due: 2026-03-21");
    }

    // ============ Round-trip tests (parse -> format -> parse) ============

    #[test]
    fn test_roundtrip_pending_task() {
        let original = "[ ] Buy milk";
        let task = parse_task_line(original).unwrap();
        let formatted = format_task_line(&task);
        assert_eq!(original, formatted);
        let reparsed = parse_task_line(&formatted).unwrap();
        assert_eq!(task.text, reparsed.text);
        assert_eq!(task.done, reparsed.done);
        assert_eq!(task.cancelled, reparsed.cancelled);
    }

    #[test]
    fn test_roundtrip_task_with_due_date() {
        let original = "[ ] Buy milk | due: 2026-03-21";
        let task = parse_task_line(original).unwrap();
        let formatted = format_task_line(&task);
        assert_eq!(original, formatted);
        let reparsed = parse_task_line(&formatted).unwrap();
        assert_eq!(task.text, reparsed.text);
        assert_eq!(task.due_date, reparsed.due_date);
    }

    #[test]
    fn test_roundtrip_completed_task() {
        let original = "[x] File taxes";
        let task = parse_task_line(original).unwrap();
        let formatted = format_task_line(&task);
        let reparsed = parse_task_line(&formatted).unwrap();
        assert_eq!(task.text, reparsed.text);
        assert!(reparsed.done);
        assert!(!reparsed.cancelled);
    }

    #[test]
    fn test_roundtrip_cancelled_task() {
        let original = "[~] Reschedule";
        let task = parse_task_line(original).unwrap();
        let formatted = format_task_line(&task);
        let reparsed = parse_task_line(&formatted).unwrap();
        assert_eq!(task.text, reparsed.text);
        assert!(!reparsed.done);
        assert!(reparsed.cancelled);
    }

    // ============ is_valid_date_string tests ============

    #[test]
    fn test_valid_date_string() {
        assert!(is_valid_date_string("2026-03-18"));
        assert!(is_valid_date_string("2025-01-01"));
        assert!(is_valid_date_string("2000-12-31"));
    }

    #[test]
    fn test_invalid_date_format() {
        assert!(!is_valid_date_string("03-18-2026")); // US format
        assert!(!is_valid_date_string("18-03-2026")); // EU format
        assert!(!is_valid_date_string("2026/03/18")); // Slash separator
    }

    #[test]
    fn test_invalid_date_values() {
        assert!(!is_valid_date_string("2026-13-01")); // Invalid month
        assert!(!is_valid_date_string("2026-02-30")); // Invalid day for February
        assert!(!is_valid_date_string("2026-04-31")); // Invalid day for April
    }

    #[test]
    fn test_invalid_date_strings() {
        assert!(!is_valid_date_string("not-a-date"));
        assert!(!is_valid_date_string(""));
        assert!(!is_valid_date_string("2026-03"));
        assert!(!is_valid_date_string("2026"));
    }

    #[test]
    fn test_leap_year_date() {
        assert!(is_valid_date_string("2024-02-29")); // Valid leap year
        assert!(!is_valid_date_string("2026-02-29")); // Invalid non-leap year
    }

    // ============ Task struct tests ============

    #[test]
    fn test_task_defaults() {
        let task = Task {
            text: "Test".to_string(),
            done: false,
            cancelled: false,
            due_date: None,
        };
        assert!(!task.done);
        assert!(!task.cancelled);
    }

    #[test]
    fn test_task_all_states() {
        let pending = Task {
            text: "Pending".to_string(),
            done: false,
            cancelled: false,
            due_date: None,
        };
        let done = Task {
            text: "Done".to_string(),
            done: true,
            cancelled: false,
            due_date: None,
        };
        let cancelled = Task {
            text: "Cancelled".to_string(),
            done: false,
            cancelled: true,
            due_date: None,
        };

        assert!(!pending.done && !pending.cancelled);
        assert!(done.done && !done.cancelled);
        assert!(!cancelled.done && cancelled.cancelled);
    }
}
