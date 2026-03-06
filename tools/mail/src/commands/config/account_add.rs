use std::time::Duration;

use oauth::{account_token_path, start_loopback_listener, store_token};

use crate::config::{
    add_account, ensure_local_storage, load_accounts, validate_email, Provider, ACCOUNT_CONFIG_FILE,
};
use crate::error::AppError;
use crate::interactive::{confirm, prompt};
use crate::providers::google;

const TOOL_NAME: &str = "mail";

pub fn run() -> Result<String, AppError> {
    let email = prompt("account email")?;
    validate_email(&email)?;

    let provider_input = prompt("provider")?;
    let provider = parse_provider(&provider_input)?;

    let accounts = load_accounts()?;
    if accounts.iter().any(|account| account.email == email) {
        return Err(AppError::config(format!("account {email} already exists")));
    }

    let paths = ensure_local_storage()?;
    let token_path = account_token_path(TOOL_NAME, &email)
        .map_err(|error| AppError::config(format!("failed to resolve token path: {error}")))?;

    let info = format!(
        "config path: {}\ntoken path: {}\ncontinue? [y/N]",
        paths.config_dir.join(ACCOUNT_CONFIG_FILE).display(),
        token_path.display(),
    );

    if !confirm(&info)? {
        return Ok("account setup cancelled".to_string());
    }

    match provider {
        Provider::Google => add_google_account(&email),
    }
}

fn add_google_account(email: &str) -> Result<String, AppError> {
    let client = google::load_oauth_client()?;
    let listener =
        start_loopback_listener().map_err(|error| AppError::config(format!("{error}")))?;
    let redirect_uri = listener.config.redirect_uri.clone();
    let auth_url =
        google::build_authorization_url(&client.client_id, &redirect_uri, &listener.config.state);

    println!("Open this URL in your browser:\n");
    println!("{auth_url}\n");
    println!("Waiting for OAuth callback on {}", redirect_uri);

    let callback = listener
        .wait_for_callback(Duration::from_secs(180))
        .map_err(|error| AppError::config(format!("{error}")))?;

    let token = google::exchange_code(&callback.code, &redirect_uri)?;
    let stored_token_path = store_token(TOOL_NAME, email, &token.raw_json)
        .map_err(|error| AppError::config(format!("failed to store token: {error}")))?;
    if let Err(error) = add_account(email, &Provider::Google) {
        let _ = std::fs::remove_file(&stored_token_path);
        return Err(error);
    }

    Ok(format!(
        "account added: {email}\nprovider: {}\nstatus: ready",
        google::provider_name()
    ))
}

fn parse_provider(input: &str) -> Result<Provider, AppError> {
    match input.trim() {
        "google" => Ok(Provider::Google),
        other => Err(AppError::usage(format!("unsupported provider: {other}"))),
    }
}
