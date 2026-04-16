use std::env;

use deco_config::resolve_read_configuration;
use deco_core_model::{DecoError, ErrorCategory};
use deco_engine::{CommandRunner, DockerEngine, ExecRequest};
use deco_lifecycle::{
    LifecycleCommand, LifecycleExecutionReport, LifecycleHooks, LifecyclePlanner, LifecycleStep,
    LifecycleStepError, LifecycleStepRunner, execute_plan,
};
use serde::Serialize;
use serde_json::Value;

use crate::cli::RunUserCommandsArgs;
use crate::commands::target::{resolve_named_target, resolve_runtime_container_id};

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RunUserCommandsResult {
    pub container_id: String,
    pub planned_steps: usize,
    pub execution_status: &'static str,
    pub report: LifecycleExecutionReport,
}

pub fn run(args: RunUserCommandsArgs) -> Result<RunUserCommandsResult, DecoError> {
    run_with_engine(args, DockerEngine::new())
}

pub(crate) fn run_with_engine<R: CommandRunner>(
    args: RunUserCommandsArgs,
    engine: DockerEngine<R>,
) -> Result<RunUserCommandsResult, DecoError> {
    let current_dir = env::current_dir().map_err(|error| {
        DecoError::new(ErrorCategory::Internal, "failed to determine current working directory")
            .with_details(error.to_string())
    })?;

    let target = resolve_named_target(crate::cli::TargetArgs {
        workspace_folder: args.workspace_folder.clone(),
        config: args.config.clone(),
    })?;
    let resolved = resolve_read_configuration(
        &current_dir,
        args.workspace_folder.as_deref(),
        args.config.as_deref(),
        false,
    )?;
    let hooks = hooks_from_config(&resolved.configuration);
    let plan = LifecyclePlanner::default().plan(&hooks);

    let container_id = match args.container_id {
        Some(container_id) => container_id,
        None => resolve_runtime_container_id(&target, &engine)?,
    };
    let mut runner = DockerLifecycleRunner {
        container_id: container_id.clone(),
        default_workdir: Some(target.remote_workspace_folder.clone()),
        engine,
    };
    let report = execute_plan(&plan, &mut runner);
    let execution_status = match &report.status {
        deco_lifecycle::LifecycleExecutionStatus::Completed => "completed",
        deco_lifecycle::LifecycleExecutionStatus::Failed { .. } => "failed",
    };

    Ok(RunUserCommandsResult {
        container_id,
        planned_steps: plan.steps.len(),
        execution_status,
        report,
    })
}

fn hooks_from_config(configuration: &Value) -> LifecycleHooks {
    LifecycleHooks {
        initialize: commands_for_key(configuration, "initializeCommand"),
        on_create: commands_for_key(configuration, "onCreateCommand"),
        update_content: commands_for_key(configuration, "updateContentCommand"),
        post_create: commands_for_key(configuration, "postCreateCommand"),
        post_start: commands_for_key(configuration, "postStartCommand"),
        post_attach: commands_for_key(configuration, "postAttachCommand"),
    }
}

fn commands_for_key(configuration: &Value, key: &str) -> Vec<LifecycleCommand> {
    match configuration.get(key) {
        Some(Value::String(command)) => vec![LifecycleCommand::new(command.clone())],
        Some(Value::Array(commands)) => commands
            .iter()
            .filter_map(|value| {
                value.as_str().map(|command| LifecycleCommand::new(command.to_string()))
            })
            .collect(),
        Some(Value::Object(commands)) => commands
            .values()
            .filter_map(|value| {
                value.as_str().map(|command| LifecycleCommand::new(command.to_string()))
            })
            .collect(),
        _ => Vec::new(),
    }
}

struct DockerLifecycleRunner<R> {
    container_id: String,
    default_workdir: Option<String>,
    engine: DockerEngine<R>,
}

impl<R: CommandRunner> LifecycleStepRunner for DockerLifecycleRunner<R> {
    fn run_step(&mut self, step: &LifecycleStep) -> Result<(), LifecycleStepError> {
        self.engine
            .exec(ExecRequest {
                container: self.container_id.clone(),
                command: vec![
                    "/bin/sh".to_string(),
                    "-lc".to_string(),
                    step.command.command.clone(),
                ],
                env: step
                    .command
                    .environment
                    .iter()
                    .map(|(key, value)| (key.clone(), value.clone()))
                    .collect(),
                labels: Vec::new(),
                workdir: step
                    .command
                    .working_directory
                    .as_ref()
                    .map(|value| value.display().to_string())
                    .or_else(|| self.default_workdir.clone()),
                user: step.command.user.clone(),
                tty: false,
                interactive: false,
                detach: false,
                privileged: false,
                remove: false,
            })
            .map(|_| ())
            .map_err(|error| LifecycleStepError {
                message: error.to_string(),
                details: Some(DecoError::from(error).to_string()),
            })
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::{OsStr, OsString};
    use std::sync::{Arc, Mutex};

    use deco_engine::{CommandInvocation, CommandOutput, EngineError};

    use super::*;

    #[derive(Debug, Clone)]
    struct RecordingRunner {
        invocations: Arc<Mutex<Vec<CommandInvocation>>>,
    }

    impl RecordingRunner {
        fn new() -> Self {
            Self { invocations: Arc::new(Mutex::new(Vec::new())) }
        }
    }

    impl CommandRunner for RecordingRunner {
        fn run(&self, program: &OsStr, args: &[OsString]) -> Result<CommandOutput, EngineError> {
            self.invocations
                .lock()
                .expect("lock should work")
                .push(CommandInvocation { program: program.to_os_string(), args: args.to_vec() });
            Ok(CommandOutput { status: 0, stdout: String::new(), stderr: String::new() })
        }
    }

    #[derive(Debug, Clone)]
    struct SequencedRunner {
        invocations: Arc<Mutex<Vec<CommandInvocation>>>,
        outputs: Arc<Mutex<Vec<CommandOutput>>>,
    }

    impl SequencedRunner {
        fn new(outputs: Vec<CommandOutput>) -> Self {
            Self {
                invocations: Arc::new(Mutex::new(Vec::new())),
                outputs: Arc::new(Mutex::new(outputs)),
            }
        }
    }

    impl CommandRunner for SequencedRunner {
        fn run(&self, program: &OsStr, args: &[OsString]) -> Result<CommandOutput, EngineError> {
            self.invocations
                .lock()
                .expect("lock should work")
                .push(CommandInvocation { program: program.to_os_string(), args: args.to_vec() });
            Ok(self.outputs.lock().expect("lock should work").remove(0))
        }
    }

    #[test]
    fn run_user_commands_executes_lifecycle_hooks_in_order() {
        let temp = tempfile::tempdir().expect("tempdir should be created");
        let config_dir = temp.path().join(".devcontainer");
        std::fs::create_dir_all(&config_dir).expect("config dir should be created");
        std::fs::write(
            config_dir.join("devcontainer.json"),
            r#"{
                "initializeCommand": "echo init",
                "postCreateCommand": ["echo post-create"],
                "postAttachCommand": { "attach": "echo attach" }
            }"#,
        )
        .expect("config should be written");

        let previous_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        std::env::set_current_dir(temp.path()).expect("cwd should be changed");

        let runner = RecordingRunner::new();
        let captured = runner.invocations.clone();
        let result = run_with_engine(
            RunUserCommandsArgs {
                container_id: Some("container-123".to_string()),
                workspace_folder: Some(temp.path().to_path_buf()),
                config: None,
            },
            DockerEngine::with_runner(runner),
        )
        .expect("run-user-commands should succeed");

        std::env::set_current_dir(previous_dir).expect("cwd should be restored");

        assert_eq!(result.planned_steps, 3);
        assert_eq!(result.execution_status, "completed");

        let invocations = captured.lock().expect("lock should work");
        assert_eq!(invocations.len(), 3);
        assert_eq!(invocations[0].args[0], OsString::from("exec"));
        assert_eq!(invocations[0].args[1], OsString::from("--workdir"));
        assert!(invocations[0].args[2].to_string_lossy().starts_with("/workspaces/"));
        assert_eq!(invocations[0].args[3], OsString::from("container-123"));
        assert_eq!(invocations[0].args[4], OsString::from("/bin/sh"));
        assert_eq!(invocations[0].args[6], OsString::from("echo init"));
        assert_eq!(invocations[1].args[6], OsString::from("echo post-create"));
        assert_eq!(invocations[2].args[6], OsString::from("echo attach"));
    }

    #[test]
    fn run_user_commands_can_resolve_container_from_workspace() {
        let temp = tempfile::tempdir().expect("tempdir should be created");
        let config_dir = temp.path().join(".devcontainer");
        std::fs::create_dir_all(&config_dir).expect("config dir should be created");
        std::fs::write(
            config_dir.join("devcontainer.json"),
            r#"{ "initializeCommand": "echo init" }"#,
        )
        .expect("config should be written");

        let previous_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        std::env::set_current_dir(temp.path()).expect("cwd should be changed");
        let runner = SequencedRunner::new(vec![
            CommandOutput {
                status: 0,
                stdout: r#"[{"ID":"compose-container-1","Name":"project-app-1","Service":"app","State":"running","Status":"Up"}]"#.to_string(),
                stderr: String::new(),
            },
            CommandOutput { status: 0, stdout: String::new(), stderr: String::new() },
        ]);
        let captured = runner.invocations.clone();
        let result = run_with_engine(
            RunUserCommandsArgs {
                container_id: None,
                workspace_folder: Some(temp.path().to_path_buf()),
                config: None,
            },
            DockerEngine::with_runner(runner),
        )
        .expect("run-user-commands should succeed");
        std::env::set_current_dir(previous_dir).expect("cwd should be restored");

        let invocations = captured.lock().expect("lock should work");
        assert_eq!(invocations[0].args[1], OsString::from("--workdir"));
        assert!(invocations[0].args[2].to_string_lossy().starts_with("/workspaces/"));
        assert_eq!(result.container_id, invocations[0].args[3].to_string_lossy());
    }

    #[test]
    fn run_user_commands_can_resolve_compose_container_from_workspace() {
        let temp = tempfile::tempdir().expect("tempdir should be created");
        let config_dir = temp.path().join(".devcontainer");
        std::fs::create_dir_all(&config_dir).expect("config dir should be created");
        std::fs::write(
            config_dir.join("devcontainer.json"),
            r#"{ "dockerComposeFile": "compose.yml", "service": "app", "initializeCommand": "echo init" }"#,
        )
        .expect("config should be written");

        let previous_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        std::env::set_current_dir(temp.path()).expect("cwd should be changed");
        let runner = SequencedRunner::new(vec![
            CommandOutput {
                status: 0,
                stdout: r#"[{"ID":"compose-container-1","Name":"project-app-1","Service":"app","State":"running","Status":"Up"}]"#.to_string(),
                stderr: String::new(),
            },
            CommandOutput { status: 0, stdout: String::new(), stderr: String::new() },
        ]);
        let captured = runner.invocations.clone();
        let result = run_with_engine(
            RunUserCommandsArgs {
                container_id: None,
                workspace_folder: Some(temp.path().to_path_buf()),
                config: None,
            },
            DockerEngine::with_runner(runner),
        )
        .expect("run-user-commands should succeed");
        std::env::set_current_dir(previous_dir).expect("cwd should be restored");

        let invocations = captured.lock().expect("lock should work");
        assert_eq!(invocations[0].args[0], OsString::from("compose"));
        assert_eq!(invocations[1].args[0], OsString::from("exec"));
        assert_eq!(result.container_id, "compose-container-1");
    }
}
