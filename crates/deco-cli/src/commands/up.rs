use std::env;
use std::path::{Path, PathBuf};

use deco_config::{BuildSpec, ComposeSpec, DevcontainerConfigKind, resolve_read_configuration};
use deco_core_model::{DecoError, ErrorCategory};
use deco_engine::{
    BuildRequest, CommandRunner, ComposeProjectRequest, ComposeTargetResolutionRequest,
    ComposeUpRequest, ContainerBindMount, ContainerCreateRequest, DockerEngine, PrimitiveResult,
};
use serde::Serialize;

use crate::cli::TargetArgs;
use crate::commands::target::{
    find_existing_container, generated_container_name, generated_image_tag, resolve_named_target,
};

const KEEP_ALIVE_COMMAND: &[&str] = &["sleep", "infinity"];

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct UpResult {
    pub kind: DevcontainerConfigKind,
    pub execution_status: &'static str,
    pub container_id: String,
    pub image: String,
    pub remote_workspace_folder: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub engine_status: Option<i32>,
}

pub fn run(args: TargetArgs) -> Result<UpResult, DecoError> {
    run_with_engine(args, DockerEngine::new())
}

pub(crate) fn run_with_engine<R: CommandRunner>(
    args: TargetArgs,
    engine: DockerEngine<R>,
) -> Result<UpResult, DecoError> {
    let current_dir = env::current_dir().map_err(|error| {
        DecoError::new(ErrorCategory::Internal, "failed to determine current working directory")
            .with_details(error.to_string())
    })?;
    let target = resolve_named_target(args.clone())?;

    let resolved = resolve_read_configuration(
        &current_dir,
        args.workspace_folder.as_deref(),
        args.config.as_deref(),
        false,
    )?;
    let container_name = generated_container_name(&resolved.workspace_folder);
    let existing = find_existing_container(&target, &engine)?;

    match resolved.kind {
        DevcontainerConfigKind::Image => {
            let image = resolved.normalized.image.ok_or_else(|| {
                DecoError::new(
                    ErrorCategory::Compatibility,
                    "image config did not produce a normalized image reference",
                )
            })?;
            create_and_start(
                &engine,
                resolved.kind,
                image,
                CreateContainerSpec {
                    container_name,
                    workspace_folder: target.workspace_folder.clone(),
                    remote_workspace_folder: target.remote_workspace_folder.clone(),
                    labels: target.applied_id_labels.clone(),
                },
                existing,
                "container-started",
            )
        }
        DevcontainerConfigKind::Dockerfile => {
            let build_spec = resolved.normalized.build.ok_or_else(|| {
                DecoError::new(
                    ErrorCategory::Compatibility,
                    "dockerfile config did not produce a normalized build specification",
                )
            })?;
            let config_dir = Path::new(&resolved.config_file).parent().ok_or_else(|| {
                DecoError::new(ErrorCategory::Internal, "config file has no parent directory")
            })?;
            let image = generated_image_tag(&resolved.workspace_folder);
            if existing.is_none() {
                let build_request = build_request_from_spec(config_dir, build_spec, image.clone());
                engine.build(build_request).map_err(DecoError::from)?;
            }
            create_and_start(
                &engine,
                resolved.kind,
                image,
                CreateContainerSpec {
                    container_name,
                    workspace_folder: target.workspace_folder.clone(),
                    remote_workspace_folder: target.remote_workspace_folder.clone(),
                    labels: target.applied_id_labels.clone(),
                },
                existing,
                "image-built-and-container-started",
            )
        }
        DevcontainerConfigKind::Compose => {
            let compose_spec = resolved.normalized.compose.ok_or_else(|| {
                DecoError::new(
                    ErrorCategory::Compatibility,
                    "compose config did not produce a normalized compose specification",
                )
            })?;
            let config_dir = Path::new(&resolved.config_file).parent().ok_or_else(|| {
                DecoError::new(ErrorCategory::Internal, "config file has no parent directory")
            })?;
            compose_up(
                &engine,
                resolved.kind,
                compose_spec,
                config_dir,
                target.remote_workspace_folder.clone(),
            )
        }
        DevcontainerConfigKind::Unknown => Err(DecoError::new(
            ErrorCategory::Compatibility,
            "unable to infer devcontainer config kind for up",
        )),
    }
}

fn create_and_start<R: CommandRunner>(
    engine: &DockerEngine<R>,
    kind: DevcontainerConfigKind,
    image: String,
    create: CreateContainerSpec,
    existing: Option<deco_engine::ContainerInspectResult>,
    execution_status: &'static str,
) -> Result<UpResult, DecoError> {
    if let Some(existing) = existing {
        let container_id = parse_container_id_from_inspect(&existing.raw)?;
        let start_result = engine.start(&container_id).map_err(DecoError::from)?;
        let started_inspect = engine.inspect(&container_id).map_err(DecoError::from)?;
        if container_is_running(&started_inspect.raw) {
            return Ok(UpResult {
                kind,
                execution_status: "reused-existing-container",
                container_id,
                image,
                remote_workspace_folder: create.remote_workspace_folder,
                engine_status: Some(start_result.status),
            });
        }

        engine.remove(&container_id, true).map_err(DecoError::from)?;
        return create_fresh_runtime(engine, kind, image, create, execution_status);
    }

    create_fresh_runtime(engine, kind, image, create, execution_status)
}

fn create_fresh_runtime<R: CommandRunner>(
    engine: &DockerEngine<R>,
    kind: DevcontainerConfigKind,
    image: String,
    create: CreateContainerSpec,
    execution_status: &'static str,
) -> Result<UpResult, DecoError> {
    let create_result = engine
        .create(ContainerCreateRequest {
            image: image.clone(),
            name: Some(create.container_name),
            mounts: vec![ContainerBindMount::new(
                PathBuf::from(create.workspace_folder),
                PathBuf::from(&create.remote_workspace_folder),
            )],
            env: Vec::new(),
            labels: {
                let mut labels = vec![("deco.managed".to_string(), "true".to_string())];
                labels.extend(create.labels);
                labels
            },
            workdir: Some(create.remote_workspace_folder.clone()),
            user: None,
            entrypoint: None,
            command: Some(KEEP_ALIVE_COMMAND.iter().map(|part| (*part).to_string()).collect()),
            tty: false,
            interactive: false,
            detach: false,
            remove: false,
        })
        .map_err(DecoError::from)?;
    let container_id = parse_container_id(&create_result)?;
    let start_result = engine.start(&container_id).map_err(DecoError::from)?;

    Ok(UpResult {
        kind,
        execution_status,
        container_id,
        image,
        remote_workspace_folder: create.remote_workspace_folder,
        engine_status: Some(start_result.status),
    })
}

fn container_is_running(raw: &serde_json::Value) -> bool {
    raw.get("State")
        .and_then(|state| state.get("Running"))
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
}

struct CreateContainerSpec {
    container_name: String,
    workspace_folder: String,
    remote_workspace_folder: String,
    labels: Vec<(String, String)>,
}

fn parse_container_id(result: &PrimitiveResult) -> Result<String, DecoError> {
    let container_id = result.stdout.trim();
    if container_id.is_empty() {
        return Err(DecoError::new(
            ErrorCategory::Engine,
            "docker create returned an empty container id",
        ));
    }

    Ok(container_id.to_string())
}

fn parse_container_id_from_inspect(raw: &serde_json::Value) -> Result<String, DecoError> {
    raw.get("Id").and_then(|value| value.as_str()).map(ToOwned::to_owned).ok_or_else(|| {
        DecoError::new(ErrorCategory::Engine, "docker inspect output did not contain container Id")
    })
}

fn build_request_from_spec(config_dir: &Path, build_spec: BuildSpec, tag: String) -> BuildRequest {
    let context = build_spec
        .context
        .map(|value| absolutize(config_dir, value))
        .unwrap_or_else(|| config_dir.to_path_buf());
    let dockerfile = build_spec.dockerfile.map(|value| absolutize(config_dir, value));

    BuildRequest {
        context,
        dockerfile,
        tag: Some(tag),
        build_args: Vec::new(),
        labels: Vec::new(),
        no_cache: false,
    }
}

fn compose_up<R: CommandRunner>(
    engine: &DockerEngine<R>,
    kind: DevcontainerConfigKind,
    compose_spec: ComposeSpec,
    config_dir: &Path,
    remote_workspace_folder: String,
) -> Result<UpResult, DecoError> {
    let files: Vec<PathBuf> =
        compose_spec.files.iter().cloned().map(|value| absolutize(config_dir, value)).collect();
    let service = compose_spec.service.ok_or_else(|| {
        DecoError::new(ErrorCategory::Compatibility, "compose config is missing a target service")
    })?;
    let project = ComposeProjectRequest {
        files,
        project_directory: Some(config_dir.to_path_buf()),
        project_name: None,
    };

    engine
        .compose_up(ComposeUpRequest {
            project: project.clone(),
            services: vec![service.clone()],
            detach: true,
            build: false,
            no_build: false,
            force_recreate: false,
            no_recreate: false,
            remove_orphans: false,
            wait: false,
        })
        .map_err(DecoError::from)?;
    let target = engine
        .resolve_compose_target(ComposeTargetResolutionRequest {
            project,
            service: service.clone(),
            prefer_running: true,
        })
        .map_err(DecoError::from)?;
    let container_id = target.container_id.ok_or_else(|| {
        DecoError::new(
            ErrorCategory::Engine,
            "docker compose target resolution did not return a container id",
        )
    })?;

    Ok(UpResult {
        kind,
        execution_status: "compose-service-started",
        container_id,
        image: service,
        remote_workspace_folder,
        engine_status: Some(target.transport.status),
    })
}

fn absolutize(base: &Path, value: String) -> PathBuf {
    let path = PathBuf::from(value);
    if path.is_absolute() { path } else { base.join(path) }
}

#[cfg(test)]
mod tests {
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
    fn image_up_creates_and_starts_container() {
        let temp = tempfile::tempdir().expect("tempdir should be created");
        let config_dir = temp.path().join(".devcontainer");
        std::fs::create_dir_all(&config_dir).expect("config dir should be created");
        std::fs::write(config_dir.join("devcontainer.json"), r#"{ "image": "alpine:3.20" }"#)
            .expect("config should be written");

        let _cwd_guard = crate::test_support::cwd_lock();
        let previous_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        std::env::set_current_dir(temp.path()).expect("cwd should be changed");

        let runner = SequencedRunner::new(vec![
            CommandOutput { status: 0, stdout: String::new(), stderr: String::new() },
            CommandOutput { status: 0, stdout: String::new(), stderr: String::new() },
            CommandOutput {
                status: 1,
                stdout: String::new(),
                stderr: "no such object".to_string(),
            },
            CommandOutput {
                status: 0,
                stdout: "container-123\n".to_string(),
                stderr: String::new(),
            },
            CommandOutput {
                status: 0,
                stdout: "container-123\n".to_string(),
                stderr: String::new(),
            },
        ]);
        let captured = runner.invocations.clone();
        let result = run_with_engine(
            TargetArgs {
                workspace_folder: Some(temp.path().to_path_buf()),
                config: None,
                id_label: Vec::new(),
            },
            DockerEngine::with_runner(runner),
        )
        .expect("up should succeed");

        std::env::set_current_dir(previous_dir).expect("cwd should be restored");

        assert_eq!(result.kind, DevcontainerConfigKind::Image);
        assert_eq!(result.execution_status, "container-started");
        assert_eq!(result.container_id, "container-123");
        assert_eq!(result.image, "alpine:3.20");
        assert!(result.remote_workspace_folder.starts_with("/workspaces/"));

        let invocations = captured.lock().expect("lock should work");
        assert_eq!(invocations.len(), 5);
        assert_eq!(invocations[0].args[0], OsString::from("ps"));
        assert_eq!(invocations[1].args[0], OsString::from("ps"));
        assert_eq!(invocations[2].args[0], OsString::from("inspect"));
        assert_eq!(invocations[3].args[0], OsString::from("create"));
        assert_eq!(invocations[4].args[0], OsString::from("start"));
        assert!(invocations[3].args.iter().any(|arg| arg == &OsString::from("--mount")));
        assert!(
            invocations[3]
                .args
                .iter()
                .any(|arg| arg.to_string_lossy().contains("target=/workspaces/"))
        );
        assert!(invocations[3].args.iter().any(|arg| arg == &OsString::from("--workdir")));
        assert!(invocations[3].args.iter().any(|arg| arg == &OsString::from("sleep")));
        assert!(invocations[3].args.iter().any(|arg| arg == &OsString::from("infinity")));
        assert!(invocations[3].args.iter().any(|arg| {
            arg == &OsString::from(format!(
                "{}={}",
                crate::commands::target::HOST_FOLDER_LABEL,
                temp.path().display()
            ))
        }));
        assert!(invocations[3].args.iter().any(|arg| {
            arg == &OsString::from(format!(
                "{}={}",
                crate::commands::target::CONFIG_FILE_LABEL,
                temp.path().join(".devcontainer").join("devcontainer.json").display()
            ))
        }));
    }

    #[test]
    fn dockerfile_up_builds_before_create_and_start() {
        let temp = tempfile::tempdir().expect("tempdir should be created");
        let config_dir = temp.path().join(".devcontainer");
        std::fs::create_dir_all(&config_dir).expect("config dir should be created");
        std::fs::write(
            config_dir.join("devcontainer.json"),
            r#"{ "dockerFile": "Dockerfile", "build": { "context": ".." } }"#,
        )
        .expect("config should be written");

        let _cwd_guard = crate::test_support::cwd_lock();
        let previous_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        std::env::set_current_dir(temp.path()).expect("cwd should be changed");

        let runner = SequencedRunner::new(vec![
            CommandOutput { status: 0, stdout: String::new(), stderr: String::new() },
            CommandOutput { status: 0, stdout: String::new(), stderr: String::new() },
            CommandOutput {
                status: 1,
                stdout: String::new(),
                stderr: "no such object".to_string(),
            },
            CommandOutput { status: 0, stdout: "build ok".to_string(), stderr: String::new() },
            CommandOutput {
                status: 0,
                stdout: "container-456\n".to_string(),
                stderr: String::new(),
            },
            CommandOutput {
                status: 0,
                stdout: "container-456\n".to_string(),
                stderr: String::new(),
            },
        ]);
        let captured = runner.invocations.clone();
        let result = run_with_engine(
            TargetArgs {
                workspace_folder: Some(temp.path().to_path_buf()),
                config: None,
                id_label: Vec::new(),
            },
            DockerEngine::with_runner(runner),
        )
        .expect("up should succeed");

        std::env::set_current_dir(previous_dir).expect("cwd should be restored");

        assert_eq!(result.kind, DevcontainerConfigKind::Dockerfile);
        assert_eq!(result.execution_status, "image-built-and-container-started");
        assert_eq!(result.container_id, "container-456");
        assert_eq!(result.image, generated_image_tag(temp.path().to_string_lossy().as_ref()));
        assert!(result.remote_workspace_folder.starts_with("/workspaces/"));

        let invocations = captured.lock().expect("lock should work");
        assert_eq!(invocations.len(), 6);
        assert_eq!(invocations[0].args[0], OsString::from("ps"));
        assert_eq!(invocations[1].args[0], OsString::from("ps"));
        assert_eq!(invocations[2].args[0], OsString::from("inspect"));
        assert_eq!(invocations[3].args[0], OsString::from("build"));
        assert_eq!(invocations[4].args[0], OsString::from("create"));
        assert_eq!(invocations[5].args[0], OsString::from("start"));
        assert!(invocations[4].args.iter().any(|arg| arg == &OsString::from("sleep")));
        assert!(invocations[4].args.iter().any(|arg| arg == &OsString::from("infinity")));
    }

    #[test]
    fn up_reuses_existing_named_container() {
        let temp = tempfile::tempdir().expect("tempdir should be created");
        let config_dir = temp.path().join(".devcontainer");
        std::fs::create_dir_all(&config_dir).expect("config dir should be created");
        std::fs::write(config_dir.join("devcontainer.json"), r#"{ "image": "alpine:3.20" }"#)
            .expect("config should be written");

        let _cwd_guard = crate::test_support::cwd_lock();
        let previous_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        std::env::set_current_dir(temp.path()).expect("cwd should be changed");

        let runner = SequencedRunner::new(vec![
            CommandOutput {
                status: 0,
                stdout: "container-existing\n".to_string(),
                stderr: String::new(),
            },
            CommandOutput {
                status: 0,
                stdout: r#"[{"Id":"container-existing","State":{"Running":false}}]"#.to_string(),
                stderr: String::new(),
            },
            CommandOutput {
                status: 0,
                stdout: "container-existing\n".to_string(),
                stderr: String::new(),
            },
            CommandOutput {
                status: 0,
                stdout: r#"[{"Id":"container-existing","State":{"Running":true}}]"#.to_string(),
                stderr: String::new(),
            },
        ]);
        let captured = runner.invocations.clone();
        let result = run_with_engine(
            TargetArgs {
                workspace_folder: Some(temp.path().to_path_buf()),
                config: None,
                id_label: Vec::new(),
            },
            DockerEngine::with_runner(runner),
        )
        .expect("up should reuse");

        std::env::set_current_dir(previous_dir).expect("cwd should be restored");

        assert_eq!(result.execution_status, "reused-existing-container");
        assert_eq!(result.container_id, "container-existing");
        assert!(result.remote_workspace_folder.starts_with("/workspaces/"));
        let invocations = captured.lock().expect("lock should work");
        assert_eq!(invocations.len(), 4);
        assert_eq!(invocations[0].args[0], OsString::from("ps"));
        assert_eq!(invocations[1].args[0], OsString::from("inspect"));
        assert_eq!(invocations[2].args[0], OsString::from("start"));
        assert_eq!(invocations[3].args[0], OsString::from("inspect"));
    }

    #[test]
    fn up_recreates_existing_container_that_exits_immediately_after_start() {
        let temp = tempfile::tempdir().expect("tempdir should be created");
        let config_dir = temp.path().join(".devcontainer");
        std::fs::create_dir_all(&config_dir).expect("config dir should be created");
        std::fs::write(config_dir.join("devcontainer.json"), r#"{ "image": "alpine:3.20" }"#)
            .expect("config should be written");

        let _cwd_guard = crate::test_support::cwd_lock();
        let previous_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        std::env::set_current_dir(temp.path()).expect("cwd should be changed");

        let runner = SequencedRunner::new(vec![
            CommandOutput {
                status: 0,
                stdout: "container-old\n".to_string(),
                stderr: String::new(),
            },
            CommandOutput {
                status: 0,
                stdout: r#"[{"Id":"container-old","State":{"Running":false},"Config":{"Cmd":["node"]}}]"#.to_string(),
                stderr: String::new(),
            },
            CommandOutput {
                status: 0,
                stdout: "container-old\n".to_string(),
                stderr: String::new(),
            },
            CommandOutput {
                status: 0,
                stdout: r#"[{"Id":"container-old","State":{"Running":false},"Config":{"Cmd":["node"]}}]"#.to_string(),
                stderr: String::new(),
            },
            CommandOutput {
                status: 0,
                stdout: "container-old\n".to_string(),
                stderr: String::new(),
            },
            CommandOutput {
                status: 0,
                stdout: "container-new\n".to_string(),
                stderr: String::new(),
            },
            CommandOutput {
                status: 0,
                stdout: "container-new\n".to_string(),
                stderr: String::new(),
            },
        ]);
        let captured = runner.invocations.clone();
        let result = run_with_engine(
            TargetArgs {
                workspace_folder: Some(temp.path().to_path_buf()),
                config: None,
                id_label: Vec::new(),
            },
            DockerEngine::with_runner(runner),
        )
        .expect("up should self-heal");

        std::env::set_current_dir(previous_dir).expect("cwd should be restored");

        assert_eq!(result.execution_status, "container-started");
        assert_eq!(result.container_id, "container-new");
        let invocations = captured.lock().expect("lock should work");
        assert_eq!(invocations[0].args[0], OsString::from("ps"));
        assert_eq!(invocations[1].args[0], OsString::from("inspect"));
        assert_eq!(invocations[2].args[0], OsString::from("start"));
        assert_eq!(invocations[3].args[0], OsString::from("inspect"));
        assert_eq!(invocations[4].args[0], OsString::from("rm"));
        assert_eq!(invocations[5].args[0], OsString::from("create"));
        assert_eq!(invocations[6].args[0], OsString::from("start"));
        assert!(invocations[5].args.iter().any(|arg| arg == &OsString::from("sleep")));
        assert!(invocations[5].args.iter().any(|arg| arg == &OsString::from("infinity")));
    }
}
