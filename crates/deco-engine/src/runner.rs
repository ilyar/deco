use std::ffi::{OsStr, OsString};
use std::process::Command;

use crate::error::EngineError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandInvocation {
    pub program: OsString,
    pub args: Vec<OsString>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandOutput {
    pub status: i32,
    pub stdout: String,
    pub stderr: String,
}

pub trait CommandRunner {
    fn run(&self, program: &OsStr, args: &[OsString]) -> Result<CommandOutput, EngineError>;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct SystemCommandRunner;

impl CommandRunner for SystemCommandRunner {
    fn run(&self, program: &OsStr, args: &[OsString]) -> Result<CommandOutput, EngineError> {
        let output = Command::new(program).args(args).output().map_err(|source| {
            EngineError::Spawn { program: program.to_string_lossy().into_owned(), source }
        })?;

        Ok(CommandOutput {
            status: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        })
    }
}
