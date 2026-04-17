use std::collections::hash_map::DefaultHasher;
use std::env;
use std::hash::{Hash, Hasher};
use std::path::Path;

use deco_config::resolve_read_configuration;
use deco_core_model::{DecoError, ErrorCategory};
use deco_engine::{
    CommandRunner, ComposeProjectRequest, ComposeTargetResolutionRequest, ContainerInspectResult,
    DockerEngine,
};

use crate::cli::TargetArgs;

pub const HOST_FOLDER_LABEL: &str = "devcontainer.local_folder";
pub const CONFIG_FILE_LABEL: &str = "devcontainer.config_file";

pub fn resolve_named_target(args: TargetArgs) -> Result<ResolvedTarget, DecoError> {
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

    let container_name = generated_container_name(&resolved.workspace_folder);
    let remote_workspace_folder = resolved
        .normalized
        .workspace_folder
        .clone()
        .unwrap_or_else(|| default_remote_workspace_folder(&resolved.workspace_folder));
    let workspace_folder = resolved.workspace_folder.clone();
    let config_file = resolved.config_file.clone();
    let compatibility_labels = compatibility_labels(&workspace_folder, &config_file);
    let requested_id_labels = parse_id_labels(&args.id_label)?;
    let applied_id_labels = merge_labels(&compatibility_labels, &requested_id_labels);
    let lookup_label_sets = if requested_id_labels.is_empty() {
        vec![
            compatibility_labels.clone(),
            vec![(
                HOST_FOLDER_LABEL.to_string(),
                normalize_devcontainer_label_path(&workspace_folder),
            )],
        ]
    } else {
        vec![requested_id_labels.clone()]
    };

    Ok(ResolvedTarget {
        workspace_folder,
        config_file,
        container_name,
        remote_workspace_folder,
        applied_id_labels,
        lookup_label_sets,
        allow_name_lookup: requested_id_labels.is_empty(),
        resolved,
    })
}

#[derive(Debug, Clone)]
pub struct ResolvedTarget {
    pub workspace_folder: String,
    pub config_file: String,
    pub container_name: String,
    pub remote_workspace_folder: String,
    pub applied_id_labels: Vec<(String, String)>,
    pub lookup_label_sets: Vec<Vec<(String, String)>>,
    pub allow_name_lookup: bool,
    pub resolved: deco_config::ResolvedReadConfiguration,
}

pub fn resolve_runtime_container_id<R: CommandRunner>(
    target: &ResolvedTarget,
    engine: &DockerEngine<R>,
) -> Result<String, DecoError> {
    if target.resolved.kind == deco_config::DevcontainerConfigKind::Compose {
        let compose = target.resolved.normalized.compose.clone().ok_or_else(|| {
            DecoError::new(
                ErrorCategory::Compatibility,
                "compose target did not produce a normalized compose specification",
            )
        })?;
        let config_dir = Path::new(&target.config_file).parent().ok_or_else(|| {
            DecoError::new(ErrorCategory::Internal, "config file has no parent directory")
        })?;
        let files = compose
            .files
            .into_iter()
            .map(|value| {
                let path = std::path::PathBuf::from(value);
                if path.is_absolute() { path } else { config_dir.join(path) }
            })
            .collect();
        let service = compose.service.ok_or_else(|| {
            DecoError::new(ErrorCategory::Compatibility, "compose target is missing a service")
        })?;
        let result = engine
            .resolve_compose_target(ComposeTargetResolutionRequest {
                project: ComposeProjectRequest {
                    files,
                    project_directory: Some(config_dir.to_path_buf()),
                    project_name: None,
                },
                service,
                prefer_running: true,
            })
            .map_err(DecoError::from)?;
        return result.container_id.ok_or_else(|| {
            DecoError::new(
                ErrorCategory::Engine,
                "docker compose target resolution did not return a container id",
            )
        });
    }

    if let Some(container) = find_existing_container(target, engine)? {
        return container_id_from_inspect(&container.raw);
    }

    Ok(target.container_name.clone())
}

pub fn find_existing_container<R: CommandRunner>(
    target: &ResolvedTarget,
    engine: &DockerEngine<R>,
) -> Result<Option<ContainerInspectResult>, DecoError> {
    if target.resolved.kind == deco_config::DevcontainerConfigKind::Compose {
        return Ok(None);
    }

    for label_set in &target.lookup_label_sets {
        if label_set.is_empty() {
            continue;
        }
        if let Some(container) =
            engine.find_container_by_labels(label_set).map_err(DecoError::from)?
        {
            return Ok(Some(container));
        }
    }

    if target.allow_name_lookup {
        match engine.inspect(&target.container_name) {
            Ok(result) => return Ok(Some(result)),
            Err(deco_engine::EngineError::Exit { .. }) => {}
            Err(error) => return Err(DecoError::from(error)),
        }
    }

    Ok(None)
}

pub fn generated_image_tag(workspace_folder: &str) -> String {
    format!("{}:dev", generated_name_prefix(workspace_folder))
}

pub fn generated_container_name(workspace_folder: &str) -> String {
    let hash = stable_hash_hex(workspace_folder);
    format!("{}-{hash}", generated_name_prefix(workspace_folder))
}

fn generated_name_prefix(workspace_folder: &str) -> String {
    let candidate = Path::new(workspace_folder)
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("workspace")
        .replace(|ch: char| !ch.is_ascii_alphanumeric() && ch != '-' && ch != '_', "-");
    format!("deco-{candidate}")
}

fn stable_hash_hex(value: &str) -> String {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

fn default_remote_workspace_folder(workspace_folder: &str) -> String {
    let leaf = Path::new(workspace_folder)
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("workspace");
    format!("/workspaces/{leaf}")
}

fn compatibility_labels(workspace_folder: &str, config_file: &str) -> Vec<(String, String)> {
    vec![
        (HOST_FOLDER_LABEL.to_string(), normalize_devcontainer_label_path(workspace_folder)),
        (CONFIG_FILE_LABEL.to_string(), normalize_devcontainer_label_path(config_file)),
    ]
}

fn parse_id_labels(values: &[String]) -> Result<Vec<(String, String)>, DecoError> {
    values
        .iter()
        .map(|value| {
            let Some((key, label_value)) = value.split_once('=') else {
                return Err(DecoError::new(
                    ErrorCategory::User,
                    format!("invalid id-label `{value}`; expected <key>=<value>"),
                ));
            };
            if key.is_empty() || label_value.is_empty() {
                return Err(DecoError::new(
                    ErrorCategory::User,
                    format!("invalid id-label `{value}`; expected <key>=<value>"),
                ));
            }
            Ok((key.to_string(), label_value.to_string()))
        })
        .collect()
}

fn merge_labels(
    primary: &[(String, String)],
    secondary: &[(String, String)],
) -> Vec<(String, String)> {
    let mut merged = primary.to_vec();
    for label in secondary {
        if !merged.iter().any(|existing| existing == label) {
            merged.push(label.clone());
        }
    }
    merged
}

fn normalize_devcontainer_label_path(value: &str) -> String {
    if cfg!(windows) {
        let normalized = value.replace('/', "\\");
        let bytes = normalized.as_bytes();
        if bytes.len() >= 2 && bytes[1] == b':' {
            let mut chars = normalized.chars();
            if let Some(first) = chars.next() {
                return first.to_ascii_lowercase().to_string() + chars.as_str();
            }
        }
        return normalized;
    }

    value.to_string()
}

fn container_id_from_inspect(raw: &serde_json::Value) -> Result<String, DecoError> {
    raw.get("Id").and_then(|value| value.as_str()).map(ToOwned::to_owned).ok_or_else(|| {
        DecoError::new(ErrorCategory::Engine, "docker inspect output did not contain container Id")
    })
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
    fn resolve_named_target_applies_upstream_compatibility_labels() {
        let temp = tempfile::tempdir().expect("tempdir should be created");
        let config_dir = temp.path().join(".devcontainer");
        std::fs::create_dir_all(&config_dir).expect("config dir should be created");
        std::fs::write(config_dir.join("devcontainer.json"), r#"{ "image": "alpine:3.20" }"#)
            .expect("config should be written");

        let _cwd_guard = crate::test_support::cwd_lock();
        let previous_dir = std::env::current_dir().expect("cwd should resolve");
        std::env::set_current_dir(temp.path()).expect("cwd should be changed");

        let resolved = resolve_named_target(TargetArgs {
            workspace_folder: Some(temp.path().to_path_buf()),
            config: None,
            id_label: vec!["foo=bar".to_string()],
        })
        .expect("target should resolve");

        std::env::set_current_dir(previous_dir).expect("cwd should be restored");

        assert!(
            resolved
                .applied_id_labels
                .contains(&(HOST_FOLDER_LABEL.to_string(), temp.path().display().to_string()))
        );
        assert!(resolved.applied_id_labels.contains(&(
            CONFIG_FILE_LABEL.to_string(),
            temp.path().join(".devcontainer").join("devcontainer.json").display().to_string(),
        )));
        assert!(resolved.applied_id_labels.contains(&("foo".to_string(), "bar".to_string())));
    }

    #[test]
    fn find_existing_container_prefers_label_lookup_before_name_lookup() {
        let temp = tempfile::tempdir().expect("tempdir should be created");
        let config_dir = temp.path().join(".devcontainer");
        std::fs::create_dir_all(&config_dir).expect("config dir should be created");
        std::fs::write(config_dir.join("devcontainer.json"), r#"{ "image": "alpine:3.20" }"#)
            .expect("config should be written");

        let _cwd_guard = crate::test_support::cwd_lock();
        let previous_dir = std::env::current_dir().expect("cwd should resolve");
        std::env::set_current_dir(temp.path()).expect("cwd should be changed");

        let target = resolve_named_target(TargetArgs {
            workspace_folder: Some(temp.path().to_path_buf()),
            config: None,
            id_label: Vec::new(),
        })
        .expect("target should resolve");

        let runner = SequencedRunner::new(vec![
            CommandOutput {
                status: 0,
                stdout: "container-from-labels\n".to_string(),
                stderr: String::new(),
            },
            CommandOutput {
                status: 0,
                stdout: r#"[{"Id":"container-from-labels","State":{"Running":true}}]"#.to_string(),
                stderr: String::new(),
            },
        ]);
        let captured = runner.invocations.clone();
        let engine = DockerEngine::with_runner(runner);

        let container = find_existing_container(&target, &engine)
            .expect("lookup should succeed")
            .expect("container should be found");

        std::env::set_current_dir(previous_dir).expect("cwd should be restored");

        assert_eq!(
            container.raw.get("Id").and_then(serde_json::Value::as_str),
            Some("container-from-labels")
        );

        let invocations = captured.lock().expect("lock should work");
        assert_eq!(invocations.len(), 2);
        assert_eq!(invocations[0].args[0], OsString::from("ps"));
        assert_eq!(invocations[1].args[0], OsString::from("inspect"));
        assert!(invocations[0].args.iter().any(|arg| arg
            == &OsString::from(format!("label={HOST_FOLDER_LABEL}={}", temp.path().display()))));
    }
}
