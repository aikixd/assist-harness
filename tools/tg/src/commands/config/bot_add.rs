use crate::config::store_bot;
use crate::error::AppError;
use crate::interactive::prompt;

pub fn run() -> Result<String, AppError> {
    let alias = prompt("bot alias")?;
    let token = prompt("bot token")?;

    let path = store_bot(&alias, &token)?;

    Ok(format!(
        "bot configured: {}\nconfig path: {}",
        alias.trim(),
        path.display()
    ))
}
