use serde::{Deserialize, Serialize};

/// Результат выполнения UseCase
pub type UseCaseResult<T> = Result<T, UseCaseError>;

/// Ошибка выполнения UseCase
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UseCaseError {
    pub code: String,
    pub message: String,
    pub details: Option<String>,
}

impl UseCaseError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            details: None,
        }
    }

    pub fn with_details(mut self, details: impl Into<String>) -> Self {
        self.details = Some(details.into());
        self
    }

    pub fn validation(message: impl Into<String>) -> Self {
        Self::new("VALIDATION_ERROR", message)
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new("NOT_FOUND", message)
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new("INTERNAL_ERROR", message)
    }

    pub fn external(message: impl Into<String>) -> Self {
        Self::new("EXTERNAL_ERROR", message)
    }
}

impl std::fmt::Display for UseCaseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)?;
        if let Some(details) = &self.details {
            write!(f, ": {}", details)?;
        }
        Ok(())
    }
}

impl std::error::Error for UseCaseError {}

impl From<anyhow::Error> for UseCaseError {
    fn from(err: anyhow::Error) -> Self {
        UseCaseError::internal(err.to_string())
    }
}
