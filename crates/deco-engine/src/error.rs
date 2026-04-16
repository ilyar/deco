use deco_core_model::{DecoError, ErrorCategory};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum EngineError {
    #[error("failed to spawn `{program}`")]
    Spawn {
        program: String,
        #[source]
        source: std::io::Error,
    },
    #[error("`{program}` exited with status {status}")]
    Exit { program: String, status: i32, stdout: String, stderr: String },
    #[error("invalid request: {message}")]
    InvalidRequest { message: String },
    #[error("failed to parse docker inspect output: {message}")]
    InvalidInspectOutput { message: String, stdout: String, stderr: String },
    #[error("failed to parse docker compose output: {message}")]
    InvalidComposeOutput { message: String, stdout: String, stderr: String },
}

impl EngineError {
    pub fn to_deco_error(&self) -> DecoError {
        let details = match self {
            EngineError::Spawn { source, .. } => source.to_string(),
            EngineError::Exit { stdout, stderr, .. } => {
                format!("stdout:\n{stdout}\nstderr:\n{stderr}")
            }
            EngineError::InvalidRequest { message } => message.clone(),
            EngineError::InvalidInspectOutput { stdout, stderr, .. } => {
                format!("stdout:\n{stdout}\nstderr:\n{stderr}")
            }
            EngineError::InvalidComposeOutput { stdout, stderr, .. } => {
                format!("stdout:\n{stdout}\nstderr:\n{stderr}")
            }
        };

        let mut error = DecoError::new(ErrorCategory::Engine, self.to_string());
        if !details.is_empty() {
            error = error.with_details(details);
        }
        error
    }
}

impl From<EngineError> for DecoError {
    fn from(value: EngineError) -> Self {
        value.to_deco_error()
    }
}
