use std::process::Command;

use crate::error::AppError;

pub fn current_local_timestamp() -> Result<String, AppError> {
    let output = Command::new("date")
        .arg("+%Y-%m-%dT%H:%M")
        .output()
        .map_err(|error| AppError::config(format!("failed to run date: {error}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(AppError::config(format!(
            "failed to format current local timestamp: {stderr}"
        )));
    }

    let stdout = String::from_utf8(output.stdout)
        .map_err(|error| AppError::config(format!("date output was not valid UTF-8: {error}")))?;
    Ok(stdout.trim().to_string())
}

pub fn epoch_seconds_to_local_timestamp(value: i64) -> Result<String, AppError> {
    let input = format!("@{value}");
    let output = Command::new("date")
        .arg("-d")
        .arg(input)
        .arg("+%Y-%m-%dT%H:%M")
        .output()
        .map_err(|error| AppError::config(format!("failed to run date: {error}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(AppError::config(format!(
            "failed to format provider timestamp {value}: {stderr}"
        )));
    }

    let stdout = String::from_utf8(output.stdout)
        .map_err(|error| AppError::config(format!("date output was not valid UTF-8: {error}")))?;
    Ok(stdout.trim().to_string())
}
