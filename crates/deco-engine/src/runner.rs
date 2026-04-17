use std::ffi::{OsStr, OsString};
use std::io::{self, Write};
use std::process::{Command, Stdio};

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

    fn run_attached(&self, program: &OsStr, args: &[OsString]) -> Result<i32, EngineError> {
        let output = self.run(program, args)?;
        if !output.stdout.is_empty() {
            io::stdout().write_all(output.stdout.as_bytes()).map_err(|source| {
                EngineError::Spawn { program: program.to_string_lossy().into_owned(), source }
            })?;
        }
        if !output.stderr.is_empty() {
            io::stderr().write_all(output.stderr.as_bytes()).map_err(|source| {
                EngineError::Spawn { program: program.to_string_lossy().into_owned(), source }
            })?;
        }
        Ok(output.status)
    }
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

    fn run_attached(&self, program: &OsStr, args: &[OsString]) -> Result<i32, EngineError> {
        let status = Command::new(program)
            .args(args)
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .map_err(|source| EngineError::Spawn {
                program: program.to_string_lossy().into_owned(),
                source,
            })?;
        Ok(status.code().unwrap_or(-1))
    }
}
