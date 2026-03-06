use std::fmt::{self, Display, Formatter};
use std::fs;
use std::path::Path;
use std::str::FromStr;

use oauth::{token_status, tool_paths, TokenStatus, ToolPaths};

use crate::error::AppError;

const TOOL_NAME: &str = "mail";
pub const ACCOUNT_CONFIG_FILE: &str = "accounts.txt";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Provider {
    Google,
}

impl Provider {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Google => "google",
        }
    }
}

impl Display for Provider {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for Provider {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "google" => Ok(Self::Google),
            _ => Err(()),
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccountStatus {
    Ready,
    AuthRequired,
    TokenExpired,
    Misconfigured,
}

impl AccountStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::AuthRequired => "auth_required",
            Self::TokenExpired => "token_expired",
            Self::Misconfigured => "misconfigured",
        }
    }
}

impl Display for AccountStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountEntry {
    pub email: String,
    pub provider_name: String,
    pub provider: Option<Provider>,
    pub status: AccountStatus,
    pub detail: Option<String>,
}

impl AccountEntry {
    pub fn is_ready(&self) -> bool {
        self.status == AccountStatus::Ready
    }
}

pub fn load_accounts() -> Result<Vec<AccountEntry>, AppError> {
    let paths = local_paths()?;
    let config_path = paths.config_dir.join(ACCOUNT_CONFIG_FILE);

    if !config_path.exists() {
        return Ok(Vec::new());
    }

    let contents = fs::read_to_string(&config_path).map_err(|error| {
        AppError::config(format!(
            "failed to read account config {}: {error}",
            config_path.display()
        ))
    })?;

    Ok(parse_accounts(&contents))
}

pub fn find_account<'a>(accounts: &'a [AccountEntry], email: &str) -> Option<&'a AccountEntry> {
    accounts.iter().find(|account| account.email == email)
}

pub fn local_paths() -> Result<ToolPaths, AppError> {
    tool_paths(TOOL_NAME).map_err(|error| {
        AppError::config(format!("failed to resolve local storage paths: {error}"))
    })
}

pub fn ensure_local_storage() -> Result<ToolPaths, AppError> {
    let paths = local_paths()?;
    ensure_dir(&paths.config_dir)?;
    ensure_dir(&paths.data_dir)?;
    ensure_dir(&paths.data_dir.join("tokens"))?;
    ensure_dir(&paths.cache_dir)?;
    Ok(paths)
}

pub fn add_account(email: &str, provider: &Provider) -> Result<(), AppError> {
    let paths = ensure_local_storage()?;
    let config_path = paths.config_dir.join(ACCOUNT_CONFIG_FILE);
    let mut accounts = load_accounts()?;

    if find_account(&accounts, email).is_some() {
        return Err(AppError::config(format!("account {email} already exists")));
    }

    accounts.push(AccountEntry {
        email: email.to_string(),
        provider_name: provider.to_string(),
        provider: Some(provider.clone()),
        status: AccountStatus::Ready,
        detail: None,
    });

    accounts.sort_by(|left, right| left.email.cmp(&right.email));

    let mut output = String::new();
    for account in accounts {
        if account.provider.is_some() {
            output.push_str(&format!("{} {}\n", account.email, account.provider_name));
        }
    }

    fs::write(&config_path, output).map_err(|error| {
        AppError::config(format!(
            "failed to write account config {}: {error}",
            config_path.display()
        ))
    })
}

pub fn validate_email(email: &str) -> Result<(), AppError> {
    if email.trim().is_empty() {
        return Err(AppError::usage("account email cannot be empty"));
    }

    if !email.contains('@') {
        return Err(AppError::usage(format!(
            "account email does not look valid: {email}"
        )));
    }

    Ok(())
}

fn ensure_dir(path: &Path) -> Result<(), AppError> {
    fs::create_dir_all(path).map_err(|error| {
        AppError::config(format!(
            "failed to create directory {}: {error}",
            path.display()
        ))
    })
}

fn parse_accounts(contents: &str) -> Vec<AccountEntry> {
    let mut entries = Vec::new();

    for (index, raw_line) in contents.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let parts = line.split_whitespace().collect::<Vec<_>>();
        if parts.len() != 2 {
            entries.push(AccountEntry {
                email: format!("<line:{}>", index + 1),
                provider_name: "unknown".to_string(),
                provider: None,
                status: AccountStatus::Misconfigured,
                detail: Some(
                    "expected account config line in the form: <email> <provider>".to_string(),
                ),
            });
            continue;
        }

        let email = parts[0].to_string();
        let provider_name = parts[1].to_string();
        let provider = Provider::from_str(&provider_name).ok();

        let (status, detail) = match provider {
            None => (
                AccountStatus::Misconfigured,
                Some(format!("unsupported provider: {provider_name}")),
            ),
            Some(_) => match token_status(TOOL_NAME, &email) {
                Ok(TokenStatus::Present) => (AccountStatus::Ready, None),
                Ok(TokenStatus::Missing) => (AccountStatus::AuthRequired, None),
                Err(error) => (
                    AccountStatus::Misconfigured,
                    Some(format!("failed to resolve token path: {error}")),
                ),
            },
        };

        entries.push(AccountEntry {
            email,
            provider_name,
            provider,
            status,
            detail,
        });
    }

    entries
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parser_supports_comment_and_data_lines() {
        let accounts = parse_accounts(
            r#"
            # personal inbox
            personal@gmail.com google
            work@example.com google
            "#,
        );

        assert_eq!(accounts.len(), 2);
        assert_eq!(accounts[0].email, "personal@gmail.com");
        assert_eq!(accounts[0].provider_name, "google");
    }

    #[test]
    fn parser_marks_invalid_lines_as_misconfigured() {
        let accounts = parse_accounts("this line is invalid");
        assert_eq!(accounts.len(), 1);
        assert_eq!(accounts[0].status, AccountStatus::Misconfigured);
    }

    #[test]
    fn validate_email_requires_at_sign() {
        let result = validate_email("not-an-email");
        assert!(result.is_err());
    }
}
