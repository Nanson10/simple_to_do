#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskMetadata {
    pub key: String,
    pub content: String,
}

#[derive(Clone, Debug)]
pub struct Task {
    pub text: String,
    pub done: bool,
    pub cancelled: bool,
    pub due_date: Option<String>,
    pub metadata: Vec<TaskMetadata>,
}

#[derive(Clone, Debug)]
pub struct PendingTask {
    pub date: String,
    pub index_in_day: usize,
    pub text: String,
    pub due_date: Option<String>,
    pub metadata: Vec<TaskMetadata>,
}

pub enum PromptChoice {
    Number(u32),
    NonParsable,
}
