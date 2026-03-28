use std::fmt;

#[derive(Debug, Clone)]
pub struct EiriadError {
    pub message: String,
}

impl EiriadError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for EiriadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for EiriadError {}

pub type EiriadResult<T> = Result<T, EiriadError>;
