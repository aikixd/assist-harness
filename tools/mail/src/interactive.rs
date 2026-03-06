use std::io::{self, Write};

use crate::error::AppError;

pub fn prompt(label: &str) -> Result<String, AppError> {
    print!("{label}: ");
    io::stdout()
        .flush()
        .map_err(|error| AppError::config(format!("failed to flush stdout: {error}")))?;

    let mut buffer = String::new();
    io::stdin()
        .read_line(&mut buffer)
        .map_err(|error| AppError::config(format!("failed to read user input: {error}")))?;

    Ok(buffer.trim().to_string())
}

pub fn confirm(label: &str) -> Result<bool, AppError> {
    let answer = prompt(label)?;
    Ok(matches!(answer.as_str(), "y" | "Y" | "yes" | "YES"))
}
