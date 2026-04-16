pub mod command;
pub mod diagnostics;
pub mod error;
pub mod output;

pub use command::{CommandKind, OutputFormat};
pub use diagnostics::{
    StderrMessage, StderrSeverity, emit_info, emit_progress, emit_warning, write_stderr_message,
};
pub use error::{DecoError, ErrorCategory};
pub use output::{CommandEnvelope, CommandFailure, CommandOutcome, CommandSuccess};
