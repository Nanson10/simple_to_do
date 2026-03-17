use crate::date_utils::{current_local_date, format_date_string, parse_day_selector};
use crate::storage::{
    collect_pending_tasks, ensure_data_dir, list_day_files, read_tasks_for_day, rebuild_todo_file,
    write_tasks_for_day,
};
use crate::types::{PromptChoice, Task};
use crate::ui::{
    paginated_pick, paginated_pick_read_only, print_main_command_help, prompt_choice, prompt_line,
};
use chrono::NaiveDate;
use std::io;

pub fn run() -> io::Result<()> {
    ensure_data_dir()?;
    rebuild_todo_file()?;
    let mut session_default_day = current_local_date();
    let mut omit_to_do = false;

    loop {
        println!();
        println!("=== Simple To-Do ===");
        println!(
            "Session default day: {}",
            format_date_string(session_default_day)
        );
        if omit_to_do {
            omit_to_do = false;
        } else {
            print_default_todo_view()?;
        }

        match prompt_choice("Choose an option: ")? {
            PromptChoice::Number(1) => add_task_flow(session_default_day)?,
            PromptChoice::Number(2) => complete_task_flow()?,
            PromptChoice::Number(3) => cancel_task_flow()?,
            PromptChoice::Number(4) => view_unfinished_flow()?,
            PromptChoice::Number(5) => browse_by_day_flow()?,

            PromptChoice::Number(6) => {
                if let Some(selected_day) = select_session_default_day_flow(session_default_day)? {
                    session_default_day = selected_day;
                }
            }
            PromptChoice::Number(0) => {
                println!("Goodbye.");
                break;
            }
            PromptChoice::Number(_) => println!("Please choose a valid option."),
            PromptChoice::NonParsable => {
                omit_to_do = true;
                print_main_command_help();
            }
        }
    }

    Ok(())
}

fn print_default_todo_view() -> io::Result<()> {
    let pending_tasks = collect_pending_tasks()?;
    println!("--- Unfinished Tasks ---");

    if pending_tasks.is_empty() {
        println!("No unfinished tasks available.");
        return Ok(());
    }

    for (index, task) in pending_tasks.iter().enumerate() {
        println!("{}. [{}] {}", index + 1, task.date, task.text);
    }

    Ok(())
}

fn add_task_flow(session_default_day: NaiveDate) -> io::Result<()> {
    println!();
    println!("--- Add Task ---");

    let input = prompt_line("Enter task text: ")?;
    if input.trim() == "0" {
        println!("Add task canceled.");
        return Ok(());
    }

    let task_text = input.trim();
    if task_text.is_empty() {
        println!("Task text cannot be empty.");
        return Ok(());
    }

    let target_day = format_date_string(session_default_day);
    let mut tasks = read_tasks_for_day(&target_day)?;
    tasks.push(Task {
        text: task_text.to_string(),
        done: false,
        cancelled: false,
    });

    write_tasks_for_day(&target_day, &tasks)?;
    rebuild_todo_file()?;
    println!("Added task to {}.", target_day);
    Ok(())
}

fn select_session_default_day_flow(current_default: NaiveDate) -> io::Result<Option<NaiveDate>> {
    println!();
    println!("--- Set Session Default Day ---");
    println!("Current default: {}", format_date_string(current_default));

    loop {
        let input = prompt_line("Enter day offset integer or YYYY-MM-DD: ")?;
        let trimmed = input.trim();

        if trimmed == "0" {
            println!("Session default day update canceled.");
            return Ok(None);
        }

        if let Some(day) = parse_day_selector(trimmed) {
            println!("Session default day set to {}.", format_date_string(day));
            return Ok(Some(day));
        }

        println!("Invalid input. Enter an integer offset or YYYY-MM-DD.");
    }
}

fn cancel_task_flow() -> io::Result<()> {
    let pending_tasks = collect_pending_tasks()?;
    if pending_tasks.is_empty() {
        println!("No unfinished tasks available.");
        return Ok(());
    }

    let labels: Vec<String> = pending_tasks
        .iter()
        .map(|task| format!("[{}] {}", task.date, task.text))
        .collect();

    println!();
    println!("--- Cancel Task ---");
    let Some(selected_index) = paginated_pick(&labels, "Pick a task to cancel")? else {
        println!("Cancellation canceled.");
        return Ok(());
    };

    let selected = &pending_tasks[selected_index];
    let note_input = prompt_line("Cancellation note (Enter to skip, 0 to abort): ")?;
    let trimmed_note = note_input.trim();

    if trimmed_note == "0" {
        println!("Cancellation canceled.");
        return Ok(());
    }

    let mut day_tasks = read_tasks_for_day(&selected.date)?;
    if let Some(task) = day_tasks.get_mut(selected.index_in_day) {
        task.cancelled = true;
        if !trimmed_note.is_empty() {
            task.text = format!("{} (note: {})", task.text, trimmed_note);
        }
        write_tasks_for_day(&selected.date, &day_tasks)?;
        rebuild_todo_file()?;
        println!("Cancelled: [{}] {}", selected.date, selected.text);
    } else {
        println!("The task could not be found. Please try again.");
    }

    Ok(())
}

fn complete_task_flow() -> io::Result<()> {
    let pending_tasks = collect_pending_tasks()?;
    if pending_tasks.is_empty() {
        println!("No unfinished tasks available.");
        return Ok(());
    }

    let labels: Vec<String> = pending_tasks
        .iter()
        .map(|task| format!("[{}] {}", task.date, task.text))
        .collect();

    println!();
    println!("--- Complete Task ---");
    if let Some(selected_index) = paginated_pick(&labels, "Pick a task to mark complete")? {
        let selected = &pending_tasks[selected_index];
        let mut day_tasks = read_tasks_for_day(&selected.date)?;

        if let Some(task) = day_tasks.get_mut(selected.index_in_day) {
            task.done = true;
            write_tasks_for_day(&selected.date, &day_tasks)?;
            rebuild_todo_file()?;
            println!("Marked complete: [{}] {}", selected.date, selected.text);
        } else {
            println!("The task could not be found. Please try again.");
        }
    } else {
        println!("Completion canceled.");
    }

    Ok(())
}

fn view_unfinished_flow() -> io::Result<()> {
    let pending_tasks = collect_pending_tasks()?;
    if pending_tasks.is_empty() {
        println!("No unfinished tasks available.");
        return Ok(());
    }

    let labels: Vec<String> = pending_tasks
        .iter()
        .map(|task| format!("[{}] {}", task.date, task.text))
        .collect();

    println!();
    println!("--- Unfinished Tasks ---");
    let _ = paginated_pick_read_only(&labels, "Viewing unfinished tasks")?;
    Ok(())
}

fn browse_by_day_flow() -> io::Result<()> {
    let days = list_day_files()?;
    if days.is_empty() {
        println!("No day files found.");
        return Ok(());
    }

    let day_labels: Vec<String> = days.iter().map(|day| day.to_string()).collect();
    println!();
    println!("--- Browse By Day ---");

    let Some(selected_day_index) = paginated_pick(&day_labels, "Pick a day")? else {
        println!("Browse canceled.");
        return Ok(());
    };

    let day = &days[selected_day_index];
    let tasks = read_tasks_for_day(day)?;
    if tasks.is_empty() {
        println!("No tasks stored for {}.", day);
        return Ok(());
    }

    let labels: Vec<String> = tasks
        .iter()
        .map(|task| {
            let marker = if task.done {
                "[x]"
            } else if task.cancelled {
                "[~]"
            } else {
                "[ ]"
            };
            format!("{} {}", marker, task.text)
        })
        .collect();

    println!();
    println!("Tasks for {}", day);
    let _ = paginated_pick_read_only(&labels, "Viewing tasks for selected day")?;
    Ok(())
}
