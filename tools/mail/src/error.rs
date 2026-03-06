use std::error::Error;
use std::fmt::{self, Display, Formatter};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppErrorKind {
    Usage,
    Config,
    Query,
    NotImplemented,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppError {
    kind: AppErrorKind,
    message: String,
}

impl AppError {
    pub fn usage(message: impl Into<String>) -> Self {
        Self {
            kind: AppErrorKind::Usage,
            message: message.into(),
        }
    }

    pub fn config(message: impl Into<String>) -> Self {
        Self {
            kind: AppErrorKind::Config,
            message: message.into(),
        }
    }

    pub fn query(message: impl Into<String>) -> Self {
        Self {
            kind: AppErrorKind::Query,
            message: message.into(),
        }
    }

    pub fn not_implemented(message: impl Into<String>) -> Self {
        Self {
            kind: AppErrorKind::NotImplemented,
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
