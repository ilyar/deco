use deco_core_model::DecoError;
use deco_engine::{CommandRunner, DockerEngine, ExecRequest};
use serde::Serialize;

use crate::cli::{ExecArgs, TargetArgs};
use crate::commands::target::{resolve_named_target, resolve_runtime_container_id};
use crate::commands::up;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ExecResult {
    pub container_id: String,
    pub execution_status: &'static str,
    pub exit_status: i32,
}

pub fn run(args: ExecArgs) -> Result<ExecResult, DecoError> {
    run_with_engine(args, DockerEngine::new())
}

fn run_with_engine<R: CommandRunner + Clone>(
    args: ExecArgs,
    engine: DockerEngine<R>,
) -> Result<ExecResult, DecoError> {
    let target = if args.container_id.is_none() {
        Some(resolve_named_target(TargetArgs {
            workspace_folder: args.workspace_folder.clone(),
            config: args.config.clone(),
        })?)
    } else {
        None
    };
    let ensured_runtime = if args.container_id.is_none() {
        Some(up::run_with_engine(
            TargetArgs {
                workspace_folder: args.workspace_folder.clone(),
                config: args.config.clone(),
            },
            engine.clone(),
        )?)
    } else {
        None
    };
    let container_id = match args.container_id {
        Some(container_id) => container_id,
        None => {
            if let Some(result) = ensured_runtime {
                result.container_id
            } else {
                resolve_runtime_container_id(
                    target.as_ref().expect("target should be present"),
                    &engine,
                )?
            }
        }
    };
    let workdir = args
        .workdir
        .or_else(|| target.as_ref().map(|target| target.remote_workspace_folder.clone()));
    let result = engine
        .exec(ExecRequest {
            container: container_id.clone(),
            command: args.args,
            env: Vec::new(),
            labels: Vec::new(),
            workdir,
            user: args.user,
            tty: false,
            interactive: false,
            detach: false,
            privileged: false,
            remove: false,
        })
        .map_err(DecoError::from)?;

    Ok(ExecResult {
        container_id,
        execution_status: "command-executed",
        exit_status: result.status,
    })
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
            Ok(CommandOutput { status: 0, stdout: "hello\n".to_string(), stderr: String::new() })
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

    #[derive(Debug, Clone)]
    struct MissingThenReadyRunner {
        invocations: Arc<Mutex<Vec<CommandInvocation>>>,
        outputs: Arc<Mutex<Vec<CommandOutput>>>,
    }

    impl MissingThenReadyRunner {
        fn new(outputs: Vec<CommandOutput>) -> Self {
            Self {
                invocations: Arc::new(Mutex::new(Vec::new())),
                outputs: Arc::new(Mutex::new(outputs)),
            }
        }
    }

    impl CommandRunner for MissingThenReadyRunner {
        fn run(&self, program: &OsStr, args: &[OsString]) -> Result<CommandOutput, EngineError> {
            self.invocations
                .lock()
                .expect("lock should work")
                .push(CommandInvocation { program: program.to_os_string(), args: args.to_vec() });

            if args.first() == Some(&OsString::from("inspect")) {
                return Err(EngineError::Exit {
                    program: "docker".to_string(),
                    status: 1,
                    stdout: String::new(),
                    stderr: "No such container".to_string(),
                });
            }

            Ok(self.outputs.lock().expect("lock should work").remove(0))
        }
    }

    #[test]
    fn exec_invokes_docker_exec_with_requested_context() {
        let runner = RecordingRunner::new();
        let captured = runner.invocations.clone();

        let result = run_with_engine(
            ExecArgs {
                container_id: Some("container-123".to_string()),
                workspace_folder: None,
                config: None,
                user: Some("vscode".to_string()),
                workdir: Some("/workspaces/project".to_string()),
                args: vec!["cargo".to_string(), "test".to_string()],
            },
            DockerEngine::with_runner(runner),
        )
        .expect("exec should succeed");

        assert_eq!(result.execution_status, "command-executed");
        assert_eq!(result.exit_status, 0);

        let invocations = captured.lock().expect("lock should work");
        assert_eq!(invocations.len(), 1);
        assert_eq!(invocations[0].program, OsString::from("docker"));
        assert_eq!(invocations[0].args[0], OsString::from("exec"));
        assert_eq!(invocations[0].args[1], OsString::from("--workdir"));
        assert_eq!(invocations[0].args[2], OsString::from("/workspaces/project"));
        assert_eq!(invocations[0].args[3], OsString::from("--user"));
        assert_eq!(invocations[0].args[4], OsString::from("vscode"));
        assert_eq!(invocations[0].args[5], OsString::from("container-123"));
        assert_eq!(invocations[0].args[6], OsString::from("cargo"));
        assert_eq!(invocations[0].args[7], OsString::from("test"));
    }

    #[test]
    fn exec_can_resolve_container_name_from_workspace() {
        let temp = tempfile::tempdir().expect("tempdir should be created");
        let config_dir = temp.path().join(".devcontainer");
        std::fs::create_dir_all(&config_dir).expect("config dir should be created");
        std::fs::write(config_dir.join("devcontainer.json"), r#"{ "image": "alpine:3.20" }"#)
            .expect("config should be written");
        let previous_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        std::env::set_current_dir(temp.path()).expect("cwd should be changed");

        let runner = SequencedRunner::new(vec![
            CommandOutput {
                status: 0,
                stdout: r#"[{"Id":"existing-container-1"}]"#.to_string(),
                stderr: String::new(),
            },
            CommandOutput {
                status: 0,
                stdout: "existing-container-1\n".to_string(),
                stderr: String::new(),
            },
            CommandOutput { status: 0, stdout: "ok\n".to_string(), stderr: String::new() },
        ]);
        let captured = runner.invocations.clone();
        let result = run_with_engine(
            ExecArgs {
                container_id: None,
                workspace_folder: Some(temp.path().to_path_buf()),
                config: None,
                user: None,
                workdir: None,
                args: vec!["pwd".to_string()],
            },
            DockerEngine::with_runner(runner),
        )
        .expect("exec should succeed");
        std::env::set_current_dir(previous_dir).expect("cwd should be restored");

        let invocations = captured.lock().expect("lock should work");
        let args = &invocations[2].args;
        let container_index = args
            .iter()
            .position(|arg| arg == &OsString::from("pwd"))
            .expect("pwd command should exist")
            - 1;
        assert_eq!(result.container_id, args[container_index].to_string_lossy());
        assert_eq!(invocations[0].args[0], OsString::from("inspect"));
        assert_eq!(invocations[1].args[0], OsString::from("start"));
        assert_eq!(invocations[2].args[0], OsString::from("exec"));
        assert!(args.iter().any(|arg| arg == &OsString::from("--workdir")));
        assert!(args.iter().any(|arg| arg.to_string_lossy().starts_with("/workspaces/")));
    }

    #[test]
    fn exec_auto_creates_runtime_when_workspace_container_is_missing() {
        let temp = tempfile::tempdir().expect("tempdir should be created");
        let config_dir = temp.path().join(".devcontainer");
        std::fs::create_dir_all(&config_dir).expect("config dir should be created");
        std::fs::write(config_dir.join("devcontainer.json"), r#"{ "image": "alpine:3.20" }"#)
            .expect("config should be written");
        let previous_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        std::env::set_current_dir(temp.path()).expect("cwd should be changed");

        let runner = MissingThenReadyRunner::new(vec![
            CommandOutput {
                status: 0,
                stdout: "created-container-1\n".to_string(),
                stderr: String::new(),
            },
            CommandOutput {
                status: 0,
                stdout: "created-container-1\n".to_string(),
                stderr: String::new(),
            },
            CommandOutput { status: 0, stdout: "ok\n".to_string(), stderr: String::new() },
        ]);
        let captured = runner.invocations.clone();
        let result = run_with_engine(
            ExecArgs {
                container_id: None,
                workspace_folder: Some(temp.path().to_path_buf()),
                config: None,
                user: None,
                workdir: None,
                args: vec!["pwd".to_string()],
            },
            DockerEngine::with_runner(runner),
        )
        .expect("exec should succeed");
        std::env::set_current_dir(previous_dir).expect("cwd should be restored");

        let invocations = captured.lock().expect("lock should work");
        assert_eq!(invocations[0].args[0], OsString::from("inspect"));
        assert_eq!(invocations[1].args[0], OsString::from("create"));
        assert_eq!(invocations[2].args[0], OsString::from("start"));
        assert_eq!(invocations[3].args[0], OsString::from("exec"));
        assert_eq!(result.container_id, "created-container-1");
    }

    #[test]
    fn exec_can_resolve_compose_container_from_workspace() {
        let temp = tempfile::tempdir().expect("tempdir should be created");
        let config_dir = temp.path().join(".devcontainer");
        std::fs::create_dir_all(&config_dir).expect("config dir should be created");
        std::fs::write(
            config_dir.join("devcontainer.json"),
            r#"{ "dockerComposeFile": "compose.yml", "service": "app" }"#,
        )
        .expect("config should be written");
        let previous_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        std::env::set_current_dir(temp.path()).expect("cwd should be changed");

        let runner = MissingThenReadyRunner::new(vec![
            CommandOutput { status: 0, stdout: String::new(), stderr: String::new() },
            CommandOutput {
                status: 0,
                stdout: r#"[{"ID":"compose-container-1","Name":"project-app-1","Service":"app","State":"running","Status":"Up"}]"#.to_string(),
                stderr: String::new(),
            },
            CommandOutput { status: 0, stdout: "ok\n".to_string(), stderr: String::new() },
        ]);
        let captured = runner.invocations.clone();
        let result = run_with_engine(
            ExecArgs {
                container_id: None,
                workspace_folder: Some(temp.path().to_path_buf()),
                config: None,
                user: None,
                workdir: None,
                args: vec!["pwd".to_string()],
            },
            DockerEngine::with_runner(runner),
        )
        .expect("exec should succeed");
        std::env::set_current_dir(previous_dir).expect("cwd should be restored");

        let invocations = captured.lock().expect("lock should work");
        assert_eq!(invocations[0].args[0], OsString::from("inspect"));
        assert_eq!(invocations[1].args[0], OsString::from("compose"));
        assert_eq!(invocations[2].args[0], OsString::from("compose"));
        assert_eq!(invocations[3].args[0], OsString::from("exec"));
        assert_eq!(result.container_id, "compose-container-1");
    }
}
