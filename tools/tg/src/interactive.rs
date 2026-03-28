use std::io::{self, Read, Write};

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

pub fn read_stdin() -> Result<String, AppError> {
    let mut buffer = String::new();
    io::stdin()
        .read_to_string(&mut buffer)
        .map_err(|error| AppError::config(format!("failed to read stdin: {error}")))?;
    Ok(strip_single_trailing_newline(buffer))
}

fn strip_single_trailing_newline(mut value: String) -> String {
    if value.ends_with("\r\n") {
        value.truncate(value.len() - 2);
    } else if value.ends_with('\n') {
        value.pop();
    }
    value
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_single_trailing_newline_removes_one_lf() {
        assert_eq!(
            strip_single_trailing_newline("hello\n".to_string()),
            "hello"
        );
        assert_eq!(
            strip_single_trailing_newline("hello\n\n".to_string()),
            "hello\n"
        );
    }

    #[test]
    fn strip_single_trailing_newline_removes_one_crlf() {
        assert_eq!(
            strip_single_trailing_newline("hello\r\n".to_string()),
            "hello"
        );
    }
}
