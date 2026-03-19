use crate::date_utils::current_local_date;
use crate::types::{PendingTask, Task, TaskMetadata};
use chrono::NaiveDate;
use std::cmp::Ordering;
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
                    metadata: task.metadata.clone(),
                });
            }
        }
    }

    Ok(pending)
}

pub fn rebuild_todo_file() -> io::Result<()> {
    let mut pending_tasks = collect_pending_tasks()?;
    sort_pending_tasks_by_precedence(&mut pending_tasks);
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
        let formatted = format_task_payload(&task.text, &task.due_date, &task.metadata);
        writeln!(file, "{}. [{}] {}", display_index + 1, task.date, formatted)?;
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
        let (text, due_date, metadata) = split_task_payload(rest);
        return Some(Task {
            text,
            done: false,
            cancelled: false,
            due_date,
            metadata,
        });
    }

    if let Some(rest) = trimmed.strip_prefix("[x] ") {
        let (text, due_date, metadata) = split_task_payload(rest);
        return Some(Task {
            text,
            done: true,
            cancelled: false,
            due_date,
            metadata,
        });
    }

    if let Some(rest) = trimmed.strip_prefix("[~] ") {
        let (text, due_date, metadata) = split_task_payload(rest);
        return Some(Task {
            text,
            done: false,
            cancelled: true,
            due_date,
            metadata,
        });
    }

    None
}

fn format_task_line(task: &Task) -> String {
    let payload = format_task_payload(&task.text, &task.due_date, &task.metadata);
    if task.done {
        format!("[x] {}", payload)
    } else if task.cancelled {
        format!("[~] {}", payload)
    } else {
        format!("[ ] {}", payload)
    }
}

fn split_task_payload(text: &str) -> (String, Option<String>, Vec<TaskMetadata>) {
    let trimmed = text.trim_end();

    let mut remaining = trimmed.to_string();
    let mut due_date = None;
    let mut metadata_reversed: Vec<TaskMetadata> = Vec::new();

    while let Some((prefix, key, content)) = parse_trailing_metadata_token(&remaining) {
        remaining = prefix;

        if key == "due" && due_date.is_none() && is_valid_date_string(&content) {
            due_date = Some(content);
        } else {
            metadata_reversed.push(TaskMetadata { key, content });
        }
    }

    metadata_reversed.reverse();
    (
        remaining.trim_end().to_string(),
        due_date,
        metadata_reversed,
    )
}

fn format_task_payload(text: &str, due_date: &Option<String>, metadata: &[TaskMetadata]) -> String {
    let mut payload = text.to_string();

    if let Some(due_date) = due_date {
        payload = append_metadata_token(&payload, "due", due_date);
    }

    for entry in metadata {
        payload = append_metadata_token(&payload, &entry.key, &entry.content);
    }

    payload
}

fn parse_trailing_metadata_token(text: &str) -> Option<(String, String, String)> {
    let token_start = text.rfind(" (")? + 1;
    let token = &text[token_start..];
    let (key, value) = parse_metadata_token(token)?;

    Some((text[..token_start - 1].to_string(), key, value))
}

fn parse_metadata_token(token: &str) -> Option<(String, String)> {
    if !token.starts_with('(') || !token.ends_with(')') {
        return None;
    }

    let inner = &token[1..token.len() - 1];
    let separator_index = find_unescaped_colon(inner)?;

    let key = inner[..separator_index].trim();
    if key.is_empty() {
        return None;
    }

    let raw_value = &inner[separator_index + 1..];
    let value = unescape_metadata_content(raw_value);
    Some((key.to_string(), value))
}

fn find_unescaped_colon(text: &str) -> Option<usize> {
    let bytes = text.as_bytes();
    for (index, byte) in bytes.iter().enumerate() {
        if *byte == b':' && !is_escaped(text, index) {
            return Some(index);
        }
    }

    None
}

fn is_escaped(text: &str, index: usize) -> bool {
    if index == 0 {
        return false;
    }

    let bytes = text.as_bytes();
    let mut slash_count = 0;
    let mut cursor = index;

    while cursor > 0 {
        cursor -= 1;
        if bytes[cursor] == b'\\' {
            slash_count += 1;
        } else {
            break;
        }
    }

    slash_count % 2 == 1
}

fn escape_metadata_content(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());

    for ch in value.chars() {
        match ch {
            '\\' | '(' | ')' | ':' => {
                escaped.push('\\');
                escaped.push(ch);
            }
            _ => escaped.push(ch),
        }
    }

    escaped
}

fn unescape_metadata_content(value: &str) -> String {
    let mut unescaped = String::with_capacity(value.len());
    let mut pending_escape = false;

    for ch in value.chars() {
        if pending_escape {
            unescaped.push(ch);
            pending_escape = false;
        } else if ch == '\\' {
            pending_escape = true;
        } else {
            unescaped.push(ch);
        }
    }

    if pending_escape {
        unescaped.push('\\');
    }

    unescaped
}

fn append_metadata_token(text: &str, key: &str, content: &str) -> String {
    let escaped_content = escape_metadata_content(content);
    if text.is_empty() {
        format!("({}:{})", key, escaped_content)
    } else {
        format!("{} ({}:{})", text, key, escaped_content)
    }
}

fn sort_pending_tasks_by_precedence(tasks: &mut [PendingTask]) {
    tasks.sort_by(compare_pending_tasks);
}

fn compare_pending_tasks(left: &PendingTask, right: &PendingTask) -> Ordering {
    let left_due = parse_valid_due_date(left.due_date.as_deref());
    let right_due = parse_valid_due_date(right.due_date.as_deref());

    // 1) Due date precedence: due dates first, earliest date first (most overdue first)
    let due_cmp = match (left_due, right_due) {
        (Some(left_due), Some(right_due)) => left_due.cmp(&right_due),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    };

    if due_cmp != Ordering::Equal {
        return due_cmp;
    }

    // 2) Descending by age: older task-day first
    let age_cmp = left.date.cmp(&right.date);
    if age_cmp != Ordering::Equal {
        return age_cmp;
    }

    // 3) Original insertion order inside a day
    left.index_in_day.cmp(&right.index_in_day)
}

fn parse_valid_due_date(value: Option<&str>) -> Option<NaiveDate> {
    value.and_then(|text| NaiveDate::parse_from_str(text, "%Y-%m-%d").ok())
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
        "# Format: [ ] pending, [x] completed, [~] cancelled; optional metadata tokens: (key:content)"
    )?;
    writeln!(
        file,
        "# Example metadata: (due:YYYY-MM-DD) (note:some\\:text)"
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
        let task = parse_task_line("[ ] Buy milk (due:2026-03-21)").unwrap();
        assert_eq!(task.text, "Buy milk");
        assert_eq!(task.due_date.as_deref(), Some("2026-03-21"));
        assert!(task.metadata.is_empty());
        assert!(!task.done);
        assert!(!task.cancelled);
    }

    #[test]
    fn test_parse_task_with_escaped_metadata_content() {
        let task = parse_task_line("[ ] Buy milk (due:2026-03-21\\))").unwrap();
        assert_eq!(task.text, "Buy milk");
        assert_eq!(task.due_date, None);
        assert_eq!(task.metadata.len(), 1);
        assert_eq!(task.metadata[0].key, "due");
        assert_eq!(task.metadata[0].content, "2026-03-21)");
    }

    #[test]
    fn test_parse_task_with_generic_metadata_token() {
        let task = parse_task_line("[ ] Task (tag:school)").unwrap();
        assert_eq!(task.text, "Task");
        assert_eq!(task.due_date, None);
        assert_eq!(task.metadata.len(), 1);
        assert_eq!(task.metadata[0].key, "tag");
        assert_eq!(task.metadata[0].content, "school");
    }

    // ============ format_task_line tests ============

    #[test]
    fn test_format_pending_task() {
        let task = Task {
            text: "Buy milk".to_string(),
            done: false,
            cancelled: false,
            due_date: None,
            metadata: Vec::new(),
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
            metadata: Vec::new(),
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
            metadata: Vec::new(),
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
            metadata: Vec::new(),
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
            metadata: Vec::new(),
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
            metadata: Vec::new(),
        };
        assert_eq!(format_task_line(&task), "[ ] Buy milk (due:2026-03-21)");
    }

    #[test]
    fn test_format_task_with_generic_metadata() {
        let task = Task {
            text: "Buy milk".to_string(),
            done: false,
            cancelled: false,
            due_date: None,
            metadata: vec![TaskMetadata {
                key: "note".to_string(),
                content: "call mom".to_string(),
            }],
        };
        assert_eq!(format_task_line(&task), "[ ] Buy milk (note:call mom)");
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
        let original = "[ ] Buy milk (due:2026-03-21)";
        let task = parse_task_line(original).unwrap();
        let formatted = format_task_line(&task);
        assert_eq!(original, formatted);
        let reparsed = parse_task_line(&formatted).unwrap();
        assert_eq!(task.text, reparsed.text);
        assert_eq!(task.due_date, reparsed.due_date);
    }

    #[test]
    fn test_roundtrip_task_with_multiple_metadata_tokens() {
        let original = "[ ] Buy milk (due:2026-03-21) (note:call mom) (tag:home)";
        let task = parse_task_line(original).unwrap();
        let formatted = format_task_line(&task);
        assert_eq!(original, formatted);
        let reparsed = parse_task_line(&formatted).unwrap();
        assert_eq!(task.text, reparsed.text);
        assert_eq!(task.due_date, reparsed.due_date);
        assert_eq!(task.metadata, reparsed.metadata);
    }

    #[test]
    fn test_escape_and_unescape_metadata_content() {
        let original = "a:b(c)\\d";
        let escaped = escape_metadata_content(original);
        assert_eq!(escaped, "a\\:b\\(c\\)\\\\d");
        assert_eq!(unescape_metadata_content(&escaped), original);
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
            metadata: Vec::new(),
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
            metadata: Vec::new(),
        };
        let done = Task {
            text: "Done".to_string(),
            done: true,
            cancelled: false,
            due_date: None,
            metadata: Vec::new(),
        };
        let cancelled = Task {
            text: "Cancelled".to_string(),
            done: false,
            cancelled: true,
            due_date: None,
            metadata: Vec::new(),
        };

        assert!(!pending.done && !pending.cancelled);
        assert!(done.done && !done.cancelled);
        assert!(!cancelled.done && cancelled.cancelled);
    }

    // ============ precedence sorting tests ============

    #[test]
    fn test_sort_precedence_due_dates_before_none() {
        let mut tasks = vec![
            PendingTask {
                date: "2026-03-18".to_string(),
                index_in_day: 0,
                text: "No due".to_string(),
                due_date: None,
                metadata: Vec::new(),
            },
            PendingTask {
                date: "2026-03-18".to_string(),
                index_in_day: 1,
                text: "Has due".to_string(),
                due_date: Some("2026-03-20".to_string()),
                metadata: Vec::new(),
            },
        ];

        sort_pending_tasks_by_precedence(&mut tasks);

        assert_eq!(tasks[0].text, "Has due");
        assert_eq!(tasks[1].text, "No due");
    }

    #[test]
    fn test_sort_precedence_most_overdue_first() {
        let mut tasks = vec![
            PendingTask {
                date: "2026-03-18".to_string(),
                index_in_day: 0,
                text: "Due later".to_string(),
                due_date: Some("2026-03-25".to_string()),
                metadata: Vec::new(),
            },
            PendingTask {
                date: "2026-03-18".to_string(),
                index_in_day: 1,
                text: "Most overdue".to_string(),
                due_date: Some("2026-03-10".to_string()),
                metadata: Vec::new(),
            },
        ];

        sort_pending_tasks_by_precedence(&mut tasks);

        assert_eq!(tasks[0].text, "Most overdue");
        assert_eq!(tasks[1].text, "Due later");
    }

    #[test]
    fn test_sort_precedence_age_then_insertion_order() {
        let mut tasks = vec![
            PendingTask {
                date: "2026-03-18".to_string(),
                index_in_day: 1,
                text: "Newer day".to_string(),
                due_date: None,
                metadata: Vec::new(),
            },
            PendingTask {
                date: "2026-03-16".to_string(),
                index_in_day: 1,
                text: "Older day later insert".to_string(),
                due_date: None,
                metadata: Vec::new(),
            },
            PendingTask {
                date: "2026-03-16".to_string(),
                index_in_day: 0,
                text: "Older day earlier insert".to_string(),
                due_date: None,
                metadata: Vec::new(),
            },
        ];

        sort_pending_tasks_by_precedence(&mut tasks);

        assert_eq!(tasks[0].text, "Older day earlier insert");
        assert_eq!(tasks[1].text, "Older day later insert");
        assert_eq!(tasks[2].text, "Newer day");
    }
}
