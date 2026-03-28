#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PeerStatus {
    Trusted,
    Revoked,
}

impl PeerStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Trusted => "trusted",
            Self::Revoked => "revoked",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BotStatus {
    pub alias: String,
    pub username: Option<String>,
    pub status: String,
    pub trusted_peers: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeerRecord {
    pub bot_alias: String,
    pub alias: String,
    pub status: PeerStatus,
    pub chat_id: i64,
    pub user_id: i64,
    pub username: Option<String>,
    pub display_name: Option<String>,
    pub paired_at: String,
    pub pairing_update_id: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageKind {
    Text,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecvMessage {
    pub peer_alias: String,
    pub chat_id: i64,
    pub update_id: u64,
    pub message_id: Option<u64>,
    pub date: String,
    pub from: String,
    pub text: String,
    pub kind: MessageKind,
    pub content_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecvBlock {
    pub peer_alias: String,
    pub chat_id: i64,
    pub total: usize,
    pub messages: Vec<RecvMessage>,
}
