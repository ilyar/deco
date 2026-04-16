use std::collections::hash_map::DefaultHasher;
use std::env;
use std::hash::{Hash, Hasher};
use std::path::Path;

use deco_config::resolve_read_configuration;
use deco_core_model::{DecoError, ErrorCategory};
use deco_engine::{
    CommandRunner, ComposeProjectRequest, ComposeTargetResolutionRequest, DockerEngine,
};

use crate::cli::TargetArgs;

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
    let image_tag = generated_image_tag(&resolved.workspace_folder);
    let remote_workspace_folder = resolved
        .normalized
        .workspace_folder
        .clone()
        .unwrap_or_else(|| default_remote_workspace_folder(&resolved.workspace_folder));
    let workspace_folder = resolved.workspace_folder.clone();
    let config_file = resolved.config_file.clone();

    Ok(ResolvedTarget {
        workspace_folder,
        config_file,
        container_name,
        image_tag,
        remote_workspace_folder,
        resolved,
    })
}

#[derive(Debug, Clone)]
pub struct ResolvedTarget {
    pub workspace_folder: String,
    pub config_file: String,
    pub container_name: String,
    pub image_tag: String,
    pub remote_workspace_folder: String,
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

    Ok(target.container_name.clone())
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
