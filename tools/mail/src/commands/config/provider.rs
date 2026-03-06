use crate::config::{store_provider_client_config, Provider};
use crate::error::AppError;
use crate::interactive::prompt;

pub fn run() -> Result<String, AppError> {
    let provider_input = prompt("provider")?;
    let provider = parse_provider(&provider_input)?;

    match provider {
        Provider::Google => configure_google(),
    }
}

fn configure_google() -> Result<String, AppError> {
    let client_id = prompt("google client id")?;
    let client_secret = prompt("google client secret")?;

    let path = store_provider_client_config(&Provider::Google, &client_id, &client_secret)?;

    Ok(format!(
        "provider configured: google\nconfig path: {}",
        path.display()
    ))
}

fn parse_provider(input: &str) -> Result<Provider, AppError> {
    match input.trim() {
        "google" => Ok(Provider::Google),
        other => Err(AppError::usage(format!("unsupported provider: {other}"))),
    }
}
