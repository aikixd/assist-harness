pub mod google;

use crate::config::{AccountEntry, Provider};
use crate::domain::{MessageDetail, MessageSummary};
use crate::error::AppError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListQuery {
    pub since: String,
    pub until: Option<String>,
    pub label: Option<String>,
    pub limit: Option<usize>,
}

pub fn validate_list_query(account: &AccountEntry, query: &ListQuery) -> Result<(), AppError> {
    let Some(provider) = account.provider.as_ref() else {
        return Err(AppError::query(format!(
            "account {} is not ready: {}",
            account.email, account.status
        )));
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
        None => Err(AppError::query(format!(
            "account {} is not ready: {}",
            account.email, account.status
        ))),
    }
}

pub fn get_message(account: &AccountEntry, message_id: &str) -> Result<MessageDetail, AppError> {
    match account.provider.as_ref() {
        Some(Provider::Google) => google::get_message(account, message_id),
        None => Err(AppError::query(format!(
            "account {} is not ready: {}",
            account.email, account.status
        ))),
    }
}

fn supports_label_filter(provider: &Provider) -> bool {
    match provider {
        Provider::Google => google::supports_label_filter(),
    }
}
