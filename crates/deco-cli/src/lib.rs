mod cli;
mod commands;

use std::fmt;
use std::process::ExitCode;

use clap::Parser;
use deco_core_model::{CommandEnvelope, CommandKind, DecoError};

use crate::cli::{Cli, Commands};

pub fn main_entry() -> ExitCode {
    let cli = Cli::parse();
    if let Err(error) = run(cli) {
        eprintln!("error: {}", error);
        let envelope = CommandEnvelope::<serde_json::Value>::error(
            error.command(),
            error.inner.category,
            error.inner.message.clone(),
            error.inner.details.clone(),
        );
        print_json(&envelope);
        return ExitCode::from(error.inner.exit_code() as u8);
    }
    ExitCode::SUCCESS
}

fn run(cli: Cli) -> Result<(), CommandExecutionError> {
    match cli.command {
        Commands::ReadConfiguration(args) => {
            let result = commands::read_configuration::run(args).map_err(|error| {
                CommandExecutionError::new(CommandKind::ReadConfiguration, error)
            })?;
            print_json(&CommandEnvelope::success(CommandKind::ReadConfiguration, result));
            Ok(())
        }
        Commands::Build(args) => {
            let result = commands::build::run(args)
                .map_err(|error| CommandExecutionError::new(CommandKind::Build, error))?;
            print_json(&CommandEnvelope::success(CommandKind::Build, result));
            Ok(())
        }
        Commands::Up(args) => {
            let result = commands::up::run(args)
                .map_err(|error| CommandExecutionError::new(CommandKind::Up, error))?;
            print_json(&CommandEnvelope::success(CommandKind::Up, result));
            Ok(())
        }
        Commands::Exec(args) => {
            let result = commands::exec::run(args)
                .map_err(|error| CommandExecutionError::new(CommandKind::Exec, error))?;
            print_json(&CommandEnvelope::success(CommandKind::Exec, result));
            Ok(())
        }
        Commands::RunUserCommands(args) => {
            let result = commands::run_user_commands::run(args)
                .map_err(|error| CommandExecutionError::new(CommandKind::RunUserCommands, error))?;
            print_json(&CommandEnvelope::success(CommandKind::RunUserCommands, result));
            Ok(())
        }
        Commands::SetUp(args) => {
            let result = commands::set_up::run(args)
                .map_err(|error| CommandExecutionError::new(CommandKind::SetUp, error))?;
            print_json(&CommandEnvelope::success(CommandKind::SetUp, result));
            Ok(())
        }
        Commands::Features(args) => {
            let result = commands::features::run(args)
                .map_err(|error| CommandExecutionError::new(CommandKind::Features, error))?;
            print_json(&CommandEnvelope::success(CommandKind::Features, result));
            Ok(())
        }
        Commands::Templates(args) => {
            let result = commands::templates::run(args)
                .map_err(|error| CommandExecutionError::new(CommandKind::Templates, error))?;
            print_json(&CommandEnvelope::success(CommandKind::Templates, result));
            Ok(())
        }
        Commands::Outdated(args) => {
            let result = commands::outdated::run(args)
                .map_err(|error| CommandExecutionError::new(CommandKind::Outdated, error))?;
            print_json(&CommandEnvelope::success(CommandKind::Outdated, result));
            Ok(())
        }
        Commands::Upgrade(args) => {
            let result = commands::upgrade::run(args)
                .map_err(|error| CommandExecutionError::new(CommandKind::Upgrade, error))?;
            print_json(&CommandEnvelope::success(CommandKind::Upgrade, result));
            Ok(())
        }
    }
}

fn print_json<T: serde::Serialize>(value: &T) {
    let payload =
        serde_json::to_string_pretty(value).expect("serializing command result should not fail");
    println!("{payload}");
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
