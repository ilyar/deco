use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ErrorCategory {
    User,
    Config,
    Engine,
    Lifecycle,
    Compatibility,
    Internal,
    Unimplemented,
}

#[derive(Debug, Error)]
#[error("{message}")]
pub struct DecoError {
    pub category: ErrorCategory,
    pub message: String,
    pub details: Option<String>,
}

impl DecoError {
    pub fn new(category: ErrorCategory, message: impl Into<String>) -> Self {
        Self { category, message: message.into(), details: None }
    }

    pub fn with_details(mut self, details: impl Into<String>) -> Self {
        self.details = Some(details.into());
        self
    }

    pub fn exit_code(&self) -> i32 {
        match self.category {
            ErrorCategory::User => 2,
            ErrorCategory::Config => 3,
            ErrorCategory::Engine => 4,
            ErrorCategory::Lifecycle => 5,
            ErrorCategory::Compatibility => 6,
            ErrorCategory::Internal => 70,
            ErrorCategory::Unimplemented => 90,
        }
    }
}
