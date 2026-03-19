use crate::date_utils::{current_local_date, format_date_string, parse_day_selector};
use crate::storage::{
    collect_pending_tasks_sorted, ensure_data_dir, list_day_files, read_tasks_for_day,
    rebuild_todo_file, write_tasks_for_day,
};
use crate::types::{PendingTask, PromptChoice, Task, TaskMetadata};
use crate::ui::{
    paginated_pick, paginated_pick_read_only, print_main_command_help, prompt_choice, prompt_line,
};
use chrono::NaiveDate;
use std::io;

enum EditSource {
    TodoList,
    SpecificDay(NaiveDate),
}

enum EditSubcommand {
    Text,
    DueDate,
    StartDay,
}

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
            PromptChoice::Number(2) => edit_task_flow()?,
            PromptChoice::Number(3) => complete_task_flow()?,
            PromptChoice::Number(4) => cancel_task_flow()?,
            PromptChoice::Number(5) => view_unfinished_flow()?,
            PromptChoice::Number(6) => browse_by_day_flow()?,
            PromptChoice::Number(7) => {
                if let Some(selected_day) = select_session_default_day_flow(session_default_day)? {
                    session_default_day = selected_day;
                }
            }
            PromptChoice::Number(0) => {
                println!("Goodbye!");
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
    let pending_tasks = collect_pending_tasks_sorted()?;
    println!("--- Unfinished Tasks ---");

    if pending_tasks.is_empty() {
        println!("No unfinished tasks available.");
        return Ok(());
    }

    for (index, task) in pending_tasks.iter().enumerate() {
        println!(
            "{}. [{}] {}",
            index + 1,
            task.date,
            format_task_text_with_due(task)
        );
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
        due_date: None,
        metadata: Vec::new(),
    });

    write_tasks_for_day(&target_day, &tasks)?;
    rebuild_todo_file()?;
    println!("Added task to {}.", target_day);
    Ok(())
}

fn edit_task_flow() -> io::Result<()> {
    println!();
    println!("--- Edit Task ---");

    let Some(edit_subcommand) = select_edit_subcommand_flow()? else {
        println!("Edit canceled.");
        return Ok(());
    };

    let Some(edit_source) = select_edit_source_flow()? else {
        println!("Edit canceled.");
        return Ok(());
    };

    let (current_label, target_day, target_index) = match edit_source {
        EditSource::TodoList => {
            let pending_tasks = collect_pending_tasks_sorted()?;
            if pending_tasks.is_empty() {
                println!("No unfinished tasks available.");
                return Ok(());
            }

            let labels: Vec<String> = pending_tasks
                .iter()
                .map(|task| format!("[{}] {}", task.date, format_task_text_with_due(task)))
                .collect();

            let Some(selected_index) = paginated_pick(&labels, "Pick a task to edit")? else {
                println!("Edit canceled.");
                return Ok(());
            };

            (
                labels[selected_index].clone(),
                pending_tasks[selected_index].date.clone(),
                pending_tasks[selected_index].index_in_day,
            )
        }
        EditSource::SpecificDay(day) => {
            let target_day = format_date_string(day);
            let tasks = read_tasks_for_day(&target_day)?;
            if tasks.is_empty() {
                println!("No tasks stored for {}.", target_day);
                return Ok(());
            }

            let labels: Vec<String> = tasks.iter().map(format_task_label).collect();
            let Some(selected_index) = paginated_pick(&labels, "Pick a task to edit")? else {
                println!("Edit canceled.");
                return Ok(());
            };

            (labels[selected_index].clone(), target_day, selected_index)
        }
    };

    println!("Current: {}", current_label);

    match edit_subcommand {
        EditSubcommand::Text => edit_task_text(&target_day, target_index),
        EditSubcommand::DueDate => edit_task_due_date(&target_day, target_index),
        EditSubcommand::StartDay => move_task_start_day(&target_day, target_index),
    }?;

    Ok(())
}

fn select_edit_subcommand_flow() -> io::Result<Option<EditSubcommand>> {
    print_edit_subcommand_help();

    loop {
        match prompt_choice("Edit command (1 = text, 2 = due date, 3 = start day, 0 = cancel): ")? {
            PromptChoice::Number(1) => return Ok(Some(EditSubcommand::Text)),
            PromptChoice::Number(2) => return Ok(Some(EditSubcommand::DueDate)),
            PromptChoice::Number(3) => return Ok(Some(EditSubcommand::StartDay)),
            PromptChoice::Number(0) => return Ok(None),
            PromptChoice::Number(_) => println!("Please choose a valid option."),
            PromptChoice::NonParsable => print_edit_subcommand_help(),
        }
    }
}

fn print_edit_subcommand_help() {
    println!("Command list:");
    println!("1. Edit task text");
    println!("2. Edit due date");
    println!("3. Move task start day");
    println!("0. Cancel");
}

fn edit_task_text(target_day: &str, target_index: usize) -> io::Result<()> {
    let revised_text = prompt_line("Enter revised task text (Enter deletes task): ")?;
    let action_prompt = if revised_text.trim().is_empty() {
        "Confirm delete? 1 = yes, 0 = no: "
    } else {
        "Confirm edit? 1 = yes, 0 = no: "
    };

    if !confirm_action(action_prompt)? {
        println!("Edit canceled.");
        return Ok(());
    }

    let mut day_tasks = read_tasks_for_day(target_day)?;
    if target_index >= day_tasks.len() {
        println!("The task could not be found. Please try again.");
        return Ok(());
    }

    if revised_text.trim().is_empty() {
        let removed_task = day_tasks.remove(target_index);
        write_tasks_for_day(target_day, &day_tasks)?;
        rebuild_todo_file()?;
        println!("Deleted: [{}] {}", target_day, removed_task.text);
        return Ok(());
    }

    day_tasks[target_index].text = revised_text.trim().to_string();
    write_tasks_for_day(target_day, &day_tasks)?;
    rebuild_todo_file()?;
    println!("Updated: [{}] {}", target_day, day_tasks[target_index].text);
    Ok(())
}

fn edit_task_due_date(target_day: &str, target_index: usize) -> io::Result<()> {
    let mut day_tasks = read_tasks_for_day(target_day)?;
    if target_index >= day_tasks.len() {
        println!("The task could not be found. Please try again.");
        return Ok(());
    }

    let task = &day_tasks[target_index];
    match &task.due_date {
        Some(due_date) => println!("Current due date: {}", due_date),
        None => println!("Current due date: none"),
    }

    println!("Enter due day offset integer or YYYY-MM-DD (Enter clears due date).");

    let new_due_date = loop {
        let input = prompt_line("New due date: ")?;
        let trimmed = input.trim();

        if trimmed.is_empty() {
            break None;
        }

        if let Some(day) = parse_day_selector(trimmed) {
            break Some(format_date_string(day));
        }

        println!("Invalid input. Enter an integer offset, YYYY-MM-DD, or press Enter to clear.");
    };

    if !confirm_action("Confirm due date update? 1 = yes, 0 = no: ")? {
        println!("Due date update canceled.");
        return Ok(());
    }

    day_tasks[target_index].due_date = new_due_date;
    write_tasks_for_day(target_day, &day_tasks)?;
    rebuild_todo_file()?;

    let updated_task = &day_tasks[target_index];
    println!(
        "Updated: [{}] {}",
        target_day,
        format_task_label(updated_task)
    );

    Ok(())
}

fn move_task_start_day(target_day: &str, target_index: usize) -> io::Result<()> {
    let mut source_tasks = read_tasks_for_day(target_day)?;
    if target_index >= source_tasks.len() {
        println!("The task could not be found. Please try again.");
        return Ok(());
    }

    let task_label = format_task_label(&source_tasks[target_index]);
    println!("Current start day: {}", target_day);
    println!("Task: {}", task_label);

    let new_start_day = loop {
        let input =
            prompt_line("Enter new start day offset integer or YYYY-MM-DD (0 to cancel): ")?;
        let trimmed = input.trim();

        if trimmed == "0" {
            println!("Move start day canceled.");
            return Ok(());
        }

        if let Some(day) = parse_day_selector(trimmed) {
            break format_date_string(day);
        }

        println!("Invalid input. Enter an integer offset, YYYY-MM-DD, or 0 to cancel.");
    };

    if new_start_day == target_day {
        println!("Task start day is unchanged.");
        return Ok(());
    }

    if !confirm_action("Confirm move start day? 1 = yes, 0 = no: ")? {
        println!("Move start day canceled.");
        return Ok(());
    }

    let moved_task = source_tasks.remove(target_index);
    write_tasks_for_day(target_day, &source_tasks)?;

    let mut destination_tasks = read_tasks_for_day(&new_start_day)?;
    destination_tasks.push(moved_task);
    write_tasks_for_day(&new_start_day, &destination_tasks)?;

    rebuild_todo_file()?;
    println!("Moved task from [{}] to [{}].", target_day, new_start_day);
    Ok(())
}

fn select_session_default_day_flow(current_default: NaiveDate) -> io::Result<Option<NaiveDate>> {
    println!();
    println!("--- Set Session Default Day ---");
    println!("Current default: {}", format_date_string(current_default));

    loop {
        let input = prompt_line("Enter day offset integer or YYYY-MM-DD: ")?;
        let trimmed = input.trim();

        if let Some(day) = parse_day_selector(trimmed) {
            println!("Session default day set to {}.", format_date_string(day));
            return Ok(Some(day));
        }

        println!("Invalid input. Enter an integer offset or YYYY-MM-DD.");
    }
}

fn select_edit_source_flow() -> io::Result<Option<EditSource>> {
    loop {
        let input = prompt_line(
            "Enter day offset integer or YYYY-MM-DD (Enter for to-do list, 0 to cancel): ",
        )?;
        let trimmed = input.trim();

        if trimmed == "0" {
            return Ok(None);
        }

        if trimmed.is_empty() {
            return Ok(Some(EditSource::TodoList));
        }

        if let Some(day) = parse_day_selector(trimmed) {
            return Ok(Some(EditSource::SpecificDay(day)));
        }

        println!(
            "Invalid input. Enter an integer offset, YYYY-MM-DD, or press Enter for the to-do list."
        );
    }
}

fn confirm_action(prompt: &str) -> io::Result<bool> {
    loop {
        match prompt_choice(prompt)? {
            PromptChoice::Number(1) => return Ok(true),
            PromptChoice::Number(0) => return Ok(false),
            _ => println!("Please enter 1 to confirm or 0 to cancel."),
        }
    }
}

fn cancel_task_flow() -> io::Result<()> {
    let pending_tasks = collect_pending_tasks_sorted()?;
    if pending_tasks.is_empty() {
        println!("No unfinished tasks available.");
        return Ok(());
    }

    let labels: Vec<String> = pending_tasks
        .iter()
        .map(|task| format!("[{}] {}", task.date, format_task_text_with_due(task)))
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
            task.metadata.push(TaskMetadata {
                key: "note".to_string(),
                content: trimmed_note.to_string(),
            });
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
    let pending_tasks = collect_pending_tasks_sorted()?;
    if pending_tasks.is_empty() {
        println!("No unfinished tasks available.");
        return Ok(());
    }

    let labels: Vec<String> = pending_tasks
        .iter()
        .map(|task| format!("[{}] {}", task.date, format_task_text_with_due(task)))
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
    let pending_tasks = collect_pending_tasks_sorted()?;
    if pending_tasks.is_empty() {
        println!("No unfinished tasks available.");
        return Ok(());
    }

    let labels: Vec<String> = pending_tasks
        .iter()
        .map(|task| format!("[{}] {}", task.date, format_task_text_with_due(task)))
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

    let labels: Vec<String> = tasks.iter().map(format_task_label).collect();

    println!();
    println!("Tasks for {}", day);
    let _ = paginated_pick_read_only(&labels, "Viewing tasks for selected day")?;
    Ok(())
}

fn format_task_label(task: &Task) -> String {
    let marker = if task.done {
        "[x]"
    } else if task.cancelled {
        "[~]"
    } else {
        "[ ]"
    };

    format!(
        "{} {}",
        marker,
        format_task_payload_with_metadata(&task.text, &task.due_date, &task.metadata)
    )
}

fn format_task_text_with_due(task: &PendingTask) -> String {
    format_task_payload_with_metadata(&task.text, &task.due_date, &task.metadata)
}

fn format_task_payload_with_metadata(
    text: &str,
    due_date: &Option<String>,
    metadata: &[TaskMetadata],
) -> String {
    let mut payload = text.to_string();

    if let Some(due_date) = due_date {
        payload = append_task_metadata_token(&payload, "due", due_date);
    }

    for entry in metadata {
        payload = append_task_metadata_token(&payload, &entry.key, &entry.content);
    }

    payload
}

fn append_task_metadata_token(text: &str, key: &str, content: &str) -> String {
    let escaped_content = escape_metadata_content(content);
    if text.is_empty() {
        format!("({}:{})", key, escaped_content)
    } else {
        format!("{} ({}:{})", text, key, escaped_content)
    }
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
