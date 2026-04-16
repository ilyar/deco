use deco_core_model::DecoError;
use deco_engine::{CommandRunner, DockerEngine};
use serde::Serialize;

use crate::cli::{RunUserCommandsArgs, SetUpArgs};
use crate::commands::run_user_commands::{self, RunUserCommandsResult};
use crate::commands::up::{self, UpResult};

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SetUpResult {
    pub container_id: String,
    pub remote_workspace_folder: String,
    pub up: UpResult,
    pub lifecycle: RunUserCommandsResult,
}

pub fn run(args: SetUpArgs) -> Result<SetUpResult, DecoError> {
    run_with_engine(args, DockerEngine::new())
}

pub(crate) fn run_with_engine<R: CommandRunner + Clone>(
    args: SetUpArgs,
    engine: DockerEngine<R>,
) -> Result<SetUpResult, DecoError> {
    let up_result = up::run_with_engine(args.target.clone(), engine.clone())?;
    let lifecycle_result = run_user_commands::run_with_engine(
        RunUserCommandsArgs {
            container_id: Some(up_result.container_id.clone()),
            workspace_folder: args.target.workspace_folder.clone(),
            config: args.target.config.clone(),
        },
        engine,
    )?;

    Ok(SetUpResult {
        container_id: up_result.container_id.clone(),
        remote_workspace_folder: up_result.remote_workspace_folder.clone(),
        up: up_result,
        lifecycle: lifecycle_result,
    })
}

#[cfg(test)]
mod tests {
    use std::env;
    use std::ffi::{OsStr, OsString};
    use std::sync::{Arc, Mutex};

    use deco_engine::{CommandInvocation, CommandOutput, EngineError};

    use super::*;

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
    fn set_up_runs_up_then_lifecycle_hooks_against_created_container() {
        let temp = tempfile::tempdir().expect("tempdir should be created");
        let config_dir = temp.path().join(".devcontainer");
        std::fs::create_dir_all(&config_dir).expect("config dir should be created");
        std::fs::write(
            config_dir.join("devcontainer.json"),
            r#"{
              "image": "alpine:3.20",
              "initializeCommand": "echo init"
            }"#,
        )
        .expect("config should be written");

        let previous_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        env::set_current_dir(temp.path()).expect("cwd should be changed");

        let runner = SequencedRunner::new(vec![
            CommandOutput { status: 1, stdout: String::new(), stderr: "missing".to_string() },
            CommandOutput {
                status: 0,
                stdout: "container-123\n".to_string(),
                stderr: String::new(),
            },
            CommandOutput { status: 0, stdout: String::new(), stderr: String::new() },
            CommandOutput { status: 0, stdout: String::new(), stderr: String::new() },
        ]);
        let captured = runner.invocations.clone();

        let result = run_with_engine(
            SetUpArgs {
                target: crate::cli::TargetArgs {
                    workspace_folder: Some(temp.path().to_path_buf()),
                    config: None,
                },
            },
            DockerEngine::with_runner(runner),
        )
        .expect("set-up should succeed");

        env::set_current_dir(previous_dir).expect("cwd should be restored");

        assert_eq!(result.container_id, "container-123");
        assert_eq!(result.up.execution_status, "container-started");
        assert_eq!(result.lifecycle.execution_status, "completed");
        assert_eq!(result.lifecycle.planned_steps, 1);

        let invocations = captured.lock().expect("lock should work");
        assert_eq!(invocations.len(), 4);
        assert_eq!(invocations[0].args[0], OsString::from("inspect"));
        assert_eq!(invocations[1].args[0], OsString::from("create"));
        assert_eq!(invocations[2].args[0], OsString::from("start"));
        assert_eq!(invocations[3].args[0], OsString::from("exec"));
        assert_eq!(invocations[3].args[3], OsString::from("container-123"));
    }
}
