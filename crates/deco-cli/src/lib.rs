mod cli;
mod commands;
#[cfg(test)]
mod test_support;

use std::fmt;
use std::process::ExitCode;

use clap::Parser;
use deco_core_model::{CommandEnvelope, CommandKind, DecoError};
use serde::Serialize;
use serde_json::Value;

use crate::cli::{Cli, Commands};

pub fn main_entry() -> ExitCode {
    let cli = Cli::parse();
    let output_mode = OutputMode::from_json_flag(cli.json);
    match run(cli, output_mode) {
        Ok(code) => code,
        Err(error) => {
            eprintln!("error: {}", error);
            if output_mode == OutputMode::Json {
                let envelope = CommandEnvelope::<serde_json::Value>::error(
                    error.command(),
                    error.inner.category,
                    error.inner.message.clone(),
                    error.inner.details.clone(),
                );
                print_json(&envelope);
            }
            ExitCode::from(error.inner.exit_code() as u8)
        }
    }
}

fn run(cli: Cli, output_mode: OutputMode) -> Result<ExitCode, CommandExecutionError> {
    match cli.command {
        Commands::ReadConfiguration(args) => {
            let result = commands::read_configuration::run(args).map_err(|error| {
                CommandExecutionError::new(CommandKind::ReadConfiguration, error)
            })?;
            render_success(CommandKind::ReadConfiguration, result, output_mode);
            Ok(ExitCode::SUCCESS)
        }
        Commands::Build(args) => {
            let result = commands::build::run(args)
                .map_err(|error| CommandExecutionError::new(CommandKind::Build, error))?;
            render_success(CommandKind::Build, result, output_mode);
            Ok(ExitCode::SUCCESS)
        }
        Commands::Up(args) => {
            let result = commands::up::run(args)
                .map_err(|error| CommandExecutionError::new(CommandKind::Up, error))?;
            render_success(CommandKind::Up, result, output_mode);
            Ok(ExitCode::SUCCESS)
        }
        Commands::Exec(args) => match output_mode {
            OutputMode::Json => {
                let result = commands::exec::run(args)
                    .map_err(|error| CommandExecutionError::new(CommandKind::Exec, error))?;
                let exit_status = result.exit_status;
                render_success(CommandKind::Exec, result, output_mode);
                Ok(exit_code_from_status(exit_status))
            }
            OutputMode::Text => {
                let exit_status = commands::exec::run_attached(args)
                    .map_err(|error| CommandExecutionError::new(CommandKind::Exec, error))?;
                Ok(exit_code_from_status(exit_status))
            }
        },
        Commands::RunUserCommands(args) => {
            let result = commands::run_user_commands::run(args)
                .map_err(|error| CommandExecutionError::new(CommandKind::RunUserCommands, error))?;
            render_success(CommandKind::RunUserCommands, result, output_mode);
            Ok(ExitCode::SUCCESS)
        }
        Commands::SetUp(args) => {
            let result = commands::set_up::run(args)
                .map_err(|error| CommandExecutionError::new(CommandKind::SetUp, error))?;
            render_success(CommandKind::SetUp, result, output_mode);
            Ok(ExitCode::SUCCESS)
        }
        Commands::Features(args) => {
            let result = commands::features::run(args)
                .map_err(|error| CommandExecutionError::new(CommandKind::Features, error))?;
            render_success(CommandKind::Features, result, output_mode);
            Ok(ExitCode::SUCCESS)
        }
        Commands::Templates(args) => {
            let result = commands::templates::run(args)
                .map_err(|error| CommandExecutionError::new(CommandKind::Templates, error))?;
            render_success(CommandKind::Templates, result, output_mode);
            Ok(ExitCode::SUCCESS)
        }
        Commands::Outdated(args) => {
            let result = commands::outdated::run(args)
                .map_err(|error| CommandExecutionError::new(CommandKind::Outdated, error))?;
            render_success(CommandKind::Outdated, result, output_mode);
            Ok(ExitCode::SUCCESS)
        }
        Commands::Upgrade(args) => {
            let result = commands::upgrade::run(args)
                .map_err(|error| CommandExecutionError::new(CommandKind::Upgrade, error))?;
            render_success(CommandKind::Upgrade, result, output_mode);
            Ok(ExitCode::SUCCESS)
        }
    }
}

fn render_success<T: Serialize>(command: CommandKind, result: T, output_mode: OutputMode) {
    if output_mode == OutputMode::Json {
        print_json(&CommandEnvelope::success(command, result));
        return;
    }

    let value = serde_json::to_value(result).expect("serializing command result should not fail");
    println!("{}", render_text(command, &value));
}

fn print_json<T: serde::Serialize>(value: &T) {
    let payload =
        serde_json::to_string_pretty(value).expect("serializing command result should not fail");
    println!("{payload}");
}

fn render_text(command: CommandKind, value: &Value) -> String {
    match command {
        CommandKind::ReadConfiguration => format!(
            "resolved {} config from {}",
            str_field(value, "kind").unwrap_or("unknown"),
            str_field(value, "config_file").unwrap_or("unknown config")
        ),
        CommandKind::Build => {
            let status = str_field(value, "execution_status").unwrap_or("build-finished");
            match opt_str_field(value, "image") {
                Some(image) => format!("{status}: {image}"),
                None => status.to_string(),
            }
        }
        CommandKind::Up | CommandKind::SetUp => format!(
            "{}: {}",
            str_field(value, "execution_status").unwrap_or("runtime-ready"),
            str_field(value, "container_id").unwrap_or("unknown-container")
        ),
        CommandKind::Exec => format!(
            "command executed in {} (exit {})",
            str_field(value, "container_id").unwrap_or("unknown-container"),
            value.get("exit_status").and_then(Value::as_i64).unwrap_or_default()
        ),
        CommandKind::RunUserCommands => format!(
            "{}: {}",
            str_field(value, "execution_status").unwrap_or("user-commands-completed"),
            str_field(value, "container_id").unwrap_or("unknown-container")
        ),
        CommandKind::Features => format!(
            "features: {}",
            str_field(value, "source")
                .or_else(|| str_field(value, "execution_status"))
                .unwrap_or("completed")
        ),
        CommandKind::Templates => format!(
            "templates: {}",
            str_field(value, "execution_status")
                .or_else(|| str_field(value, "source"))
                .unwrap_or("completed")
        ),
        CommandKind::Outdated => format!(
            "outdated targets: {}",
            value.get("outdated_count").and_then(Value::as_u64).unwrap_or_default()
        ),
        CommandKind::Upgrade => format!(
            "{}{}",
            str_field(value, "execution_status").unwrap_or("upgrade-completed"),
            if value.get("dry_run").and_then(Value::as_bool).unwrap_or(false) {
                " (dry-run)"
            } else {
                ""
            }
        ),
    }
}

fn str_field<'a>(value: &'a Value, field: &str) -> Option<&'a str> {
    value.get(field).and_then(Value::as_str)
}

fn opt_str_field<'a>(value: &'a Value, field: &str) -> Option<&'a str> {
    str_field(value, field).filter(|value| !value.is_empty())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutputMode {
    Text,
    Json,
}

impl OutputMode {
    fn from_json_flag(json: bool) -> Self {
        if json { Self::Json } else { Self::Text }
    }
}

fn exit_code_from_status(status: i32) -> ExitCode {
    let normalized = if status < 0 { 1 } else { status.min(u8::MAX as i32) as u8 };
    ExitCode::from(normalized)
}

#[derive(Debug)]
struct CommandExecutionError {
    command: CommandKind,
    inner: DecoError,
}

impl CommandExecutionError {
    fn new(command: CommandKind, inner: DecoError) -> Self {
        Self { command, inner }
    }

    fn command(&self) -> CommandKind {
        self.command
    }
}

impl fmt::Display for CommandExecutionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.inner)
    }
}
