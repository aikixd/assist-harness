use crate::config::load_accounts;
use crate::error::AppError;
use crate::output::join_blocks;

pub fn run() -> Result<String, AppError> {
    let accounts = load_accounts()?;
    if accounts.is_empty() {
        return Ok("no accounts configured".to_string());
    }

    let mut blocks = Vec::new();
    for account in accounts {
        let mut lines = vec![format!("{} - {}", account.email, account.provider_name)];
        lines.push(format!("status: {}", account.status));
        if let Some(detail) = account.detail {
            lines.push(format!("detail: {detail}"));
        }
        blocks.push(lines.join("\n"));
    }

    Ok(join_blocks(&blocks))
}
