use std::env;
use std::path::{Path, PathBuf};

use deco_config::{BuildSpec, ComposeSpec, DevcontainerConfigKind, resolve_read_configuration};
use deco_core_model::{DecoError, ErrorCategory};
use deco_engine::{
    BuildRequest, CommandRunner, ComposeBuildRequest, DockerEngine, PrimitiveResult,
};
use serde::Serialize;

use crate::cli::TargetArgs;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BuildResult {
    pub kind: DevcontainerConfigKind,
    pub execution_status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub engine_status: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dockerfile: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
}

pub fn run(args: TargetArgs) -> Result<BuildResult, DecoError> {
    run_with_engine(args, DockerEngine::new())
}

fn run_with_engine<R: CommandRunner>(
    args: TargetArgs,
    engine: DockerEngine<R>,
) -> Result<BuildResult, DecoError> {
    let current_dir = env::current_dir().map_err(|error| {
        DecoError::new(ErrorCategory::Internal, "failed to determine current working directory")
            .with_details(error.to_string())
    })?;

    let resolved = resolve_read_configuration(
        &current_dir,
        args.workspace_folder.as_deref(),
        args.config.as_deref(),
        false,
    )?;

    match resolved.kind {
        DevcontainerConfigKind::Image => Ok(BuildResult {
            kind: resolved.kind,
            execution_status: "skipped-existing-image",
            engine_status: None,
            image: resolved.normalized.image,
            dockerfile: None,
            context: None,
        }),
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
            let request = build_request_from_spec(config_dir, build_spec);
            let result = engine.build(request).map_err(DecoError::from)?;

            Ok(build_result_from_primitive(resolved.kind, "docker-build-completed", result))
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
            let request = compose_build_request_from_spec(config_dir, compose_spec);
            let result = engine.compose_build(request).map_err(DecoError::from)?;

            Ok(build_result_from_primitive(resolved.kind, "compose-build-completed", result))
        }
        DevcontainerConfigKind::Unknown => Err(DecoError::new(
            ErrorCategory::Compatibility,
            "unable to infer devcontainer config kind for build",
        )),
    }
}

fn build_request_from_spec(config_dir: &Path, build_spec: BuildSpec) -> BuildRequest {
    let context = build_spec
        .context
        .map(|value| absolutize(config_dir, value))
        .unwrap_or_else(|| config_dir.to_path_buf());
    let dockerfile = build_spec.dockerfile.map(|value| absolutize(config_dir, value));

    BuildRequest {
        context,
        dockerfile,
        tag: None,
        build_args: Vec::new(),
        labels: Vec::new(),
        no_cache: false,
    }
}

fn compose_build_request_from_spec(
    config_dir: &Path,
    compose_spec: ComposeSpec,
) -> ComposeBuildRequest {
    ComposeBuildRequest {
        files: compose_spec.files.into_iter().map(|value| absolutize(config_dir, value)).collect(),
        service: compose_spec.service,
    }
}

fn build_result_from_primitive(
    kind: DevcontainerConfigKind,
    execution_status: &'static str,
    primitive: PrimitiveResult,
) -> BuildResult {
    BuildResult {
        kind,
        execution_status,
        engine_status: Some(primitive.status),
        image: None,
        dockerfile: None,
        context: None,
    }
}

fn absolutize(base: &Path, value: String) -> PathBuf {
    let path = PathBuf::from(value);
    if path.is_absolute() { path } else { base.join(path) }
}

#[cfg(test)]
mod tests {
    use std::ffi::{OsStr, OsString};
    use std::sync::{Arc, Mutex};

    use deco_engine::{CommandInvocation, CommandOutput};

    use super::*;

    #[derive(Debug, Clone, Default)]
    struct RecordingRunner {
        invocations: Arc<Mutex<Vec<CommandInvocation>>>,
    }

    impl CommandRunner for RecordingRunner {
        fn run(
            &self,
            program: &OsStr,
            args: &[OsString],
        ) -> Result<CommandOutput, deco_engine::EngineError> {
            self.invocations
                .lock()
                .expect("lock should work")
                .push(CommandInvocation { program: program.to_os_string(), args: args.to_vec() });
            Ok(CommandOutput { status: 0, stdout: "build ok".to_string(), stderr: String::new() })
        }
    }

    #[test]
    fn dockerfile_build_resolves_relative_paths_against_config_dir() {
        let request = build_request_from_spec(
            Path::new("/tmp/workspace/.devcontainer"),
            BuildSpec {
                dockerfile: Some("Dockerfile".to_string()),
                context: Some("..".to_string()),
            },
        );

        assert_eq!(
            request.dockerfile,
            Some(PathBuf::from("/tmp/workspace/.devcontainer/Dockerfile"))
        );
        assert_eq!(request.context, PathBuf::from("/tmp/workspace/.devcontainer/.."));
    }

    #[test]
    fn build_invokes_docker_engine_for_dockerfile_configs() {
        let temp = tempfile::tempdir().expect("tempdir should be created");
        let config_dir = temp.path().join(".devcontainer");
        std::fs::create_dir_all(&config_dir).expect("config dir should be created");
        std::fs::write(
            config_dir.join("devcontainer.json"),
            r#"{ "dockerFile": "Dockerfile", "build": { "context": ".." } }"#,
        )
        .expect("config should be written");

        let previous_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        std::env::set_current_dir(temp.path()).expect("cwd should be changed");

        let runner = RecordingRunner::default();
        let captured = runner.invocations.clone();
        let engine = DockerEngine::with_runner(runner);
        let result = run_with_engine(
            TargetArgs { workspace_folder: Some(temp.path().to_path_buf()), config: None },
            engine,
        )
        .expect("build should succeed");

        std::env::set_current_dir(previous_dir).expect("cwd should be restored");

        assert_eq!(result.kind, DevcontainerConfigKind::Dockerfile);
        assert_eq!(result.execution_status, "docker-build-completed");
        assert_eq!(result.engine_status, Some(0));

        let invocations = captured.lock().expect("lock should work");
        assert_eq!(invocations.len(), 1);
        assert_eq!(invocations[0].program, OsString::from("docker"));
        assert_eq!(invocations[0].args[0], OsString::from("build"));
        assert_eq!(invocations[0].args[1], OsString::from("-f"));
        assert_eq!(invocations[0].args[2], config_dir.join("Dockerfile").into_os_string());
        assert_eq!(
            invocations[0].args.last().expect("context arg should exist"),
            &temp.path().join(".devcontainer").join("..").into_os_string()
        );
    }

    #[test]
    fn build_invokes_compose_build_for_compose_configs() {
        let temp = tempfile::tempdir().expect("tempdir should be created");
        let config_dir = temp.path().join(".devcontainer");
        std::fs::create_dir_all(&config_dir).expect("config dir should be created");
        std::fs::write(
            config_dir.join("devcontainer.json"),
            r#"{ "dockerComposeFile": ["compose.yml"], "service": "app" }"#,
        )
        .expect("config should be written");

        let previous_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        std::env::set_current_dir(temp.path()).expect("cwd should be changed");

        let runner = RecordingRunner::default();
        let captured = runner.invocations.clone();
        let result = run_with_engine(
            TargetArgs { workspace_folder: Some(temp.path().to_path_buf()), config: None },
            DockerEngine::with_runner(runner),
        )
        .expect("compose build should succeed");

        std::env::set_current_dir(previous_dir).expect("cwd should be restored");

        assert_eq!(result.kind, DevcontainerConfigKind::Compose);
        assert_eq!(result.execution_status, "compose-build-completed");
        let invocations = captured.lock().expect("lock should work");
        assert_eq!(invocations.len(), 1);
        assert_eq!(invocations[0].args[0], OsString::from("compose"));
        assert_eq!(invocations[0].args[1], OsString::from("-f"));
        assert_eq!(invocations[0].args[2], config_dir.join("compose.yml").into_os_string());
        assert_eq!(invocations[0].args[3], OsString::from("build"));
        assert_eq!(invocations[0].args[4], OsString::from("app"));
    }
}
