use oauth::{exchange_code_with_curl, load_client_config, OAuthClientConfig, TokenResponse};

use crate::config::{load_provider_client_config, AccountEntry, Provider};
use crate::domain::{MessageDetail, MessageSummary};
use crate::error::AppError;

use super::ListQuery;

const AUTH_ENDPOINT: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const TOKEN_ENDPOINT: &str = "https://oauth2.googleapis.com/token";
const GMAIL_READONLY_SCOPE: &str = "https://www.googleapis.com/auth/gmail.readonly";
const CLIENT_ID_ENV: &str = "PA_MAIL_GOOGLE_CLIENT_ID";
const CLIENT_SECRET_ENV: &str = "PA_MAIL_GOOGLE_CLIENT_SECRET";

pub fn supports_label_filter() -> bool {
    true
}

pub fn list_messages(
    _account: &AccountEntry,
    _query: &ListQuery,
) -> Result<Vec<MessageSummary>, AppError> {
    Err(AppError::not_implemented(
        "provider google access is not implemented yet",
    ))
}

pub fn get_message(_account: &AccountEntry, _message_id: &str) -> Result<MessageDetail, AppError> {
    Err(AppError::not_implemented(
        "provider google access is not implemented yet",
    ))
}

pub fn provider_name() -> &'static str {
    Provider::Google.as_str()
}

pub fn load_oauth_client() -> Result<OAuthClientConfig, AppError> {
    if let Ok(config) = load_provider_client_config(&Provider::Google) {
        return Ok(config);
    }

    load_client_config(CLIENT_ID_ENV, CLIENT_SECRET_ENV)
        .map_err(|error| AppError::config(format!("{error}")))
}

pub fn build_authorization_url(client_id: &str, redirect_uri: &str, state: &str) -> String {
    format!(
        "{}?response_type=code&client_id={}&redirect_uri={}&scope={}&state={}&access_type=offline&prompt=consent",
        AUTH_ENDPOINT,
        percent_encode(client_id),
        percent_encode(redirect_uri),
        percent_encode(GMAIL_READONLY_SCOPE),
        percent_encode(state),
    )
}

pub fn exchange_code(code: &str, redirect_uri: &str) -> Result<TokenResponse, AppError> {
    let client = load_oauth_client()?;
    exchange_code_with_curl(TOKEN_ENDPOINT, &client, code, redirect_uri)
        .map_err(|error| AppError::config(format!("{error}")))
}

fn percent_encode(input: &str) -> String {
    let mut output = String::new();
    for byte in input.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                output.push(byte as char)
            }
            _ => output.push_str(&format!("%{:02X}", byte)),
        }
    }
    output
}
