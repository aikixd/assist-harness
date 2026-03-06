use std::process::Command;

use crate::error::AppError;

pub fn local_timestamp_to_epoch_seconds(value: &str) -> Result<i64, AppError> {
    let output = Command::new("date")
        .arg("-d")
        .arg(value)
        .arg("+%s")
        .output()
        .map_err(|error| AppError::config(format!("failed to run date: {error}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(AppError::usage(format!(
            "invalid time value {value}: {stderr}"
        )));
    }

    let stdout = String::from_utf8(output.stdout)
        .map_err(|error| AppError::config(format!("date output was not valid UTF-8: {error}")))?;
    stdout
        .trim()
        .parse::<i64>()
        .map_err(|_| AppError::config(format!("failed to parse epoch output for {value}")))
}

pub fn epoch_millis_to_local_timestamp(value: i64) -> Result<String, AppError> {
    let seconds = value / 1000;
    let input = format!("@{seconds}");
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
