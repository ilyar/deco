use serde::{Deserialize, Serialize};

use crate::{CommandKind, ErrorCategory};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandOutcome {
    Success,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandEnvelope<T> {
    pub command: CommandKind,
    pub outcome: CommandOutcome,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<CommandFailure>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandFailure {
    pub category: ErrorCategory,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandSuccess<T> {
    pub command: CommandKind,
    pub data: T,
}

impl<T> CommandEnvelope<T> {
    pub fn success(command: CommandKind, data: T) -> Self {
        Self {
            command,
            outcome: CommandOutcome::Success,
            data: Some(data),
            error: None,
            warnings: Vec::new(),
        }
    }

    pub fn error(
        command: CommandKind,
        category: ErrorCategory,
        message: impl Into<String>,
        details: Option<String>,
    ) -> Self {
        Self {
            command,
            outcome: CommandOutcome::Error,
            data: None,
            error: Some(CommandFailure { category, message: message.into(), details }),
            warnings: Vec::new(),
        }
    }
}
