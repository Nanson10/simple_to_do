#[derive(Clone, Debug)]
pub struct Task {
    pub text: String,
    pub done: bool,
    pub cancelled: bool,
}

#[derive(Clone, Debug)]
pub struct PendingTask {
    pub date: String,
    pub index_in_day: usize,
    pub text: String,
}

pub enum PromptChoice {
    Number(u32),
    NonParsable,
}
