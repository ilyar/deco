use std::io::{self, Write};

/// Human-facing diagnostics that belong on stderr.
///
/// The convention is intentionally small:
/// - stdout stays reserved for machine-readable command payloads;
/// - stderr carries info, progress, and warnings;
/// - each line is prefixed so future command code can emit stable text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StderrSeverity {
    Info,
    Progress,
    Warning,
}

impl StderrSeverity {
    fn as_str(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Progress => "progress",
            Self::Warning => "warning",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StderrMessage {
    pub severity: StderrSeverity,
    pub stage: Option<String>,
    pub message: String,
}

impl StderrMessage {
    pub fn info(message: impl Into<String>) -> Self {
        Self { severity: StderrSeverity::Info, stage: None, message: message.into() }
    }

    pub fn progress(stage: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            severity: StderrSeverity::Progress,
            stage: Some(stage.into()),
            message: message.into(),
        }
    }

    pub fn warning(message: impl Into<String>) -> Self {
        Self { severity: StderrSeverity::Warning, stage: None, message: message.into() }
    }

    pub fn render(&self) -> String {
        match self.stage.as_deref() {
            Some(stage) => format!("[deco:{}] {}: {}", self.severity.as_str(), stage, self.message),
            None => format!("[deco:{}] {}", self.severity.as_str(), self.message),
        }
    }
}

pub fn write_stderr_message<W: Write>(writer: &mut W, message: &StderrMessage) -> io::Result<()> {
    writeln!(writer, "{}", message.render())
}

pub fn emit_info<W: Write>(writer: &mut W, message: impl Into<String>) -> io::Result<()> {
    write_stderr_message(writer, &StderrMessage::info(message))
}

pub fn emit_progress<W: Write>(
    writer: &mut W,
    stage: impl Into<String>,
    message: impl Into<String>,
) -> io::Result<()> {
    write_stderr_message(writer, &StderrMessage::progress(stage, message))
}

pub fn emit_warning<W: Write>(writer: &mut W, message: impl Into<String>) -> io::Result<()> {
    write_stderr_message(writer, &StderrMessage::warning(message))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_info_line() {
        let message = StderrMessage::info("booting");
        assert_eq!(message.render(), "[deco:info] booting");
    }

    #[test]
    fn renders_progress_line_with_stage() {
        let message = StderrMessage::progress("up", "resolving target");
        assert_eq!(message.render(), "[deco:progress] up: resolving target");
    }

    #[test]
    fn emits_warning_line() {
        let mut buffer = Vec::new();
        emit_warning(&mut buffer, "fallback in use").unwrap();
        assert_eq!(String::from_utf8(buffer).unwrap(), "[deco:warning] fallback in use\n");
    }
}
