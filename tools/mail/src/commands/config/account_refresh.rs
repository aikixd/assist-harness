use oauth::{account_token_path, store_token};

use crate::config::{
    ensure_local_storage, load_accounts, AccountEntry, Provider, ACCOUNT_CONFIG_FILE,
};
use crate::error::AppError;
use crate::interactive::{confirm, prompt};
use crate::providers::resolve_account;

use super::account_add::exchange_google_token_with_browser;

const TOOL_NAME: &str = "mail";

pub fn run() -> Result<String, AppError> {
    let accounts = load_accounts()?;
    if accounts.is_empty() {
        return Ok("no accounts configured".to_string());
    }

    let resolved_accounts = accounts.iter().map(resolve_account).collect::<Vec<_>>();
    let candidates = refresh_candidates(&resolved_accounts);
    if candidates.is_empty() {
        return Ok("no accounts need refresh".to_string());
    }

    println!("{}", format_candidates(&candidates));

    let selection = prompt("email index")?;
    let selected_index = parse_selection(&selection, candidates.len())?;
    let account = &candidates[selected_index];

    let paths = ensure_local_storage()?;
    let token_path = account_token_path(TOOL_NAME, &account.email)
        .map_err(|error| AppError::config(format!("failed to resolve token path: {error}")))?;
    let info = format!(
        "config path: {}\ntoken path: {}\ncontinue? [y/N]",
        paths.config_dir.join(ACCOUNT_CONFIG_FILE).display(),
        token_path.display(),
    );

    if !confirm(&info)? {
        return Ok("account refresh cancelled".to_string());
    }

    match account.provider.as_ref() {
        Some(Provider::Google) => refresh_google_account(&account.email),
        None => Err(AppError::config(format!(
            "account {} is not refreshable",
            account.email
        ))),
    }
}

fn refresh_google_account(email: &str) -> Result<String, AppError> {
    let token = exchange_google_token_with_browser()?;
    store_token(TOOL_NAME, email, &token.raw_json)
        .map_err(|error| AppError::config(format!("failed to store token: {error}")))?;

    Ok(format!(
        "account refreshed: {email}\nprovider: {}\nstatus: ready",
        Provider::Google
    ))
}

fn refresh_candidates(accounts: &[AccountEntry]) -> Vec<AccountEntry> {
    accounts
        .iter()
        .cloned()
        .filter(|account| !account.is_ready() && account.provider.is_some())
        .collect()
}

fn format_candidates(accounts: &[AccountEntry]) -> String {
    let mut lines = vec!["Accounts needing refresh:".to_string()];

    for (index, account) in accounts.iter().enumerate() {
        lines.push(String::new());
        lines.push(format!(
            "{}. {} - {}",
            index + 1,
            account.email,
            account.provider_name
        ));
        lines.push(format!("   status: {}", account.status));
        if let Some(detail) = &account.detail {
            lines.push(format!("   detail: {detail}"));
        }
    }

    lines.join("\n")
}

fn parse_selection(input: &str, candidate_count: usize) -> Result<usize, AppError> {
    let parsed = input
        .trim()
        .parse::<usize>()
        .map_err(|_| AppError::usage(format!("invalid email index: {input}")))?;

    if parsed == 0 || parsed > candidate_count {
        return Err(AppError::usage(format!("invalid email index: {input}")));
    }

    Ok(parsed - 1)
}

#[cfg(test)]
mod tests {
    use crate::config::{AccountStatus, Provider};

    use super::*;

    #[test]
    fn refresh_candidates_exclude_ready_and_unsupported_accounts() {
        let accounts = vec![
            AccountEntry {
                email: "ready@example.com".to_string(),
                provider_name: Provider::Google.to_string(),
                provider: Some(Provider::Google),
                status: AccountStatus::Ready,
                detail: None,
            },
            AccountEntry {
                email: "expired@example.com".to_string(),
                provider_name: Provider::Google.to_string(),
                provider: Some(Provider::Google),
                status: AccountStatus::TokenExpired,
                detail: Some("stored refresh token is invalid or revoked".to_string()),
            },
            AccountEntry {
                email: "<line:3>".to_string(),
                provider_name: "unknown".to_string(),
                provider: None,
                status: AccountStatus::Misconfigured,
                detail: Some(
                    "expected account config line in the form: <email> <provider>".to_string(),
                ),
            },
        ];

        let candidates = refresh_candidates(&accounts);
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].email, "expired@example.com");
    }

    #[test]
    fn format_candidates_includes_number_status_and_detail() {
        let rendered = format_candidates(&[AccountEntry {
            email: "expired@example.com".to_string(),
            provider_name: Provider::Google.to_string(),
            provider: Some(Provider::Google),
            status: AccountStatus::TokenExpired,
            detail: Some("stored refresh token is invalid or revoked".to_string()),
        }]);

        assert!(rendered.contains("Accounts needing refresh:"));
        assert!(rendered.contains("1. expired@example.com - google"));
        assert!(rendered.contains("status: token_expired"));
        assert!(rendered.contains("detail: stored refresh token is invalid or revoked"));
    }

    #[test]
    fn parse_selection_accepts_one_based_index() {
        assert_eq!(parse_selection("2", 3), Ok(1));
    }

    #[test]
    fn parse_selection_rejects_out_of_range_index() {
        let error = parse_selection("0", 2).expect_err("selection should fail");
        assert_eq!(error.to_string(), "invalid email index: 0");
    }
}
