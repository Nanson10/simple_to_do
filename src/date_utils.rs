use chrono::{Duration, Local, NaiveDate};

pub fn current_local_date() -> NaiveDate {
    Local::now().date_naive()
}

pub fn format_date_string(date: NaiveDate) -> String {
    date.format("%Y-%m-%d").to_string()
}

pub fn parse_day_selector(value: &str) -> Option<NaiveDate> {
    let today = current_local_date();

    if let Ok(offset) = value.parse::<i64>() {
        return today.checked_add_signed(Duration::days(offset));
    }

    NaiveDate::parse_from_str(value, "%Y-%m-%d").ok()
}
