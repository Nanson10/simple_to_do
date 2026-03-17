use crate::types::PromptChoice;
use std::io::{self, Write};

const PAGE_SIZE: usize = 7;

pub fn print_main_command_help() {
    println!("Command list:");
    println!("1. Add a task for session default day");
    println!("2. Edit a task");
    println!("3. Complete a task from to-do list");
    println!("4. Cancel a task from to-do list");
    println!("5. View unfinished to-do list");
    println!("6. Browse tasks by day");
    println!("7. Set session default day");
    println!("0. Exit");
}

pub fn print_pagination_help(page_item_count: usize) {
    println!("Command list:");
    if page_item_count > 0 {
        println!("1-{}. Select item on current page", page_item_count);
    }
    println!("8. Previous page");
    println!("9. Next page");
    println!("0. Cancel/Exit");
}

pub fn paginated_pick(items: &[String], title: &str) -> io::Result<Option<usize>> {
    if items.is_empty() {
        return Ok(None);
    }

    let mut page = 0;
    let page_count = items.len().div_ceil(PAGE_SIZE);

    loop {
        let start = page * PAGE_SIZE;
        let end = usize::min(start + PAGE_SIZE, items.len());
        let page_slice = &items[start..end];

        println!();
        println!("{} (Page {}/{})", title, page + 1, page_count);
        for (offset, item) in page_slice.iter().enumerate() {
            println!("{}. {}", offset + 1, item);
        }

        match prompt_choice("Choose: ")? {
            PromptChoice::Number(0) => return Ok(None),
            PromptChoice::Number(8) => {
                page = page.saturating_sub(1);
            }
            PromptChoice::Number(9) => {
                if page + 1 < page_count {
                    page += 1;
                }
            }
            PromptChoice::Number(selection) => {
                let local_index = selection.saturating_sub(1) as usize;
                if local_index < page_slice.len() {
                    return Ok(Some(start + local_index));
                }
                println!("Please choose a valid option for this page.");
            }
            PromptChoice::NonParsable => print_pagination_help(page_slice.len()),
        }
    }
}

pub fn paginated_pick_read_only(items: &[String], title: &str) -> io::Result<Option<usize>> {
    if items.is_empty() {
        return Ok(None);
    }

    let mut page = 0;
    let page_count = items.len().div_ceil(PAGE_SIZE);

    loop {
        let start = page * PAGE_SIZE;
        let end = usize::min(start + PAGE_SIZE, items.len());
        let page_slice = &items[start..end];

        println!();
        println!("{} (Page {}/{})", title, page + 1, page_count);
        for (offset, item) in page_slice.iter().enumerate() {
            println!("{}. {}", offset + 1, item);
        }

        match prompt_choice("Choose: ")? {
            PromptChoice::Number(0) => return Ok(None),
            PromptChoice::Number(8) => {
                page = page.saturating_sub(1);
            }
            PromptChoice::Number(9) => {
                if page + 1 < page_count {
                    page += 1;
                }
            }
            PromptChoice::Number(selection) => {
                let local_index = selection.saturating_sub(1) as usize;
                if local_index < page_slice.len() {
                    println!("Selected: {}", page_slice[local_index]);
                    continue;
                }
                println!("Please choose a valid option for this page.");
            }
            PromptChoice::NonParsable => print_pagination_help(page_slice.len()),
        }
    }
}

pub fn prompt_choice(prompt: &str) -> io::Result<PromptChoice> {
    print_flush(prompt)?;
    let input = read_input_line()?;
    match input.trim().parse::<u32>() {
        Ok(number) => Ok(PromptChoice::Number(number)),
        Err(_) => Ok(PromptChoice::NonParsable),
    }
}

pub fn prompt_line(prompt: &str) -> io::Result<String> {
    print_flush(prompt)?;
    read_input_line()
}

fn print_flush(text: &str) -> io::Result<()> {
    print!("{}", text);
    io::stdout().flush()
}

fn read_input_line() -> io::Result<String> {
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(input.trim_end().to_string())
}
