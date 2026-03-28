use std::error::Error;
use std::fmt::{self, Display, Formatter};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppError {
    message: String,
}

impl AppError {
    pub fn usage(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    pub fn config(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    pub fn query(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl Display for AppError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Error for AppError {}
