pub mod google;

use crate::config::{AccountEntry, Provider};
use crate::domain::{MessageDetail, MessageSummary};
use crate::error::AppError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageState {
    Read,
    Unread,
    All,
}

impl MessageState {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "read" => Some(Self::Read),
            "unread" => Some(Self::Unread),
            "all" => Some(Self::All),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListQuery {
    pub since: String,
    pub until: Option<String>,
    pub state: MessageState,
    pub label: Option<String>,
    pub limit: Option<usize>,
}

pub fn resolve_account(account: &AccountEntry) -> AccountEntry {
    match account.provider.as_ref() {
        Some(Provider::Google) => google::resolve_account(account),
        None => account.clone(),
    }
}

pub fn account_not_ready_error(account: &AccountEntry) -> AppError {
    let mut message = format!("account {} is not ready: {}", account.email, account.status);
    if let Some(detail) = &account.detail {
        message.push_str(&format!("\ndetail: {detail}"));
    }
    AppError::query(message)
}

pub fn validate_list_query(account: &AccountEntry, query: &ListQuery) -> Result<(), AppError> {
    let Some(provider) = account.provider.as_ref() else {
        return Err(account_not_ready_error(account));
    };

    if query.label.is_some() && !supports_label_filter(provider) {
        return Err(AppError::query(
            "filter --label is not supported by this account's provider",
        ));
    }

    Ok(())
}

pub fn list_messages(
    account: &AccountEntry,
    query: &ListQuery,
) -> Result<Vec<MessageSummary>, AppError> {
    match account.provider.as_ref() {
        Some(Provider::Google) => google::list_messages(account, query),
        None => Err(account_not_ready_error(account)),
    }
}

pub fn get_message(
    account: &AccountEntry,
    message_id: &str,
    raw_body: bool,
) -> Result<MessageDetail, AppError> {
    match account.provider.as_ref() {
        Some(Provider::Google) => google::get_message(account, message_id, raw_body),
        None => Err(account_not_ready_error(account)),
    }
}

fn supports_label_filter(provider: &Provider) -> bool {
    match provider {
        Provider::Google => google::supports_label_filter(),
    }
}

#[cfg(test)]
mod tests {
    use crate::config::{AccountStatus, Provider};

    use super::*;

    #[test]
    fn account_not_ready_error_includes_detail_when_present() {
        let account = AccountEntry {
            email: "me@example.com".to_string(),
            provider_name: Provider::Google.to_string(),
            provider: Some(Provider::Google),
            status: AccountStatus::TokenExpired,
            detail: Some("stored refresh token is invalid or revoked".to_string()),
        };

        assert_eq!(
            account_not_ready_error(&account).to_string(),
            "account me@example.com is not ready: token_expired\ndetail: stored refresh token is invalid or revoked"
        );
    }
}
