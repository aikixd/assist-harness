#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageSummary {
    pub id: String,
    pub date: String,
    pub from: String,
    pub to: String,
    pub subject: String,
    pub labels: Vec<String>,
    pub body_preview: String,
    pub thread_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountMessageBlock {
    pub account: String,
    pub unread: usize,
    pub total: usize,
    pub messages: Vec<MessageSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attachment {
    pub name: String,
    pub mime_type: String,
    pub size_bytes: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageDetail {
    pub account: String,
    pub id: String,
    pub thread_id: Option<String>,
    pub date: String,
    pub from: String,
    pub to: String,
    pub cc: Option<String>,
    pub subject: String,
    pub labels: Vec<String>,
    pub body_text: String,
    pub links: Vec<String>,
    pub attachments: Vec<Attachment>,
}
