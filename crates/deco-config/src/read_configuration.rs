use std::fs;
use std::path::{Path, PathBuf};

use deco_core_model::{DecoError, ErrorCategory};
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum DevcontainerConfigKind {
    Image,
    Dockerfile,
    Compose,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ResolvedReadConfiguration {
    pub workspace_folder: String,
    pub config_file: String,
    pub kind: DevcontainerConfigKind,
    pub normalized: NormalizedDevcontainerConfig,
    pub configuration: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub merged_configuration: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct NormalizedDevcontainerConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub build: Option<BuildSpec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compose: Option<ComposeSpec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_folder: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_user: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_env: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BuildSpec {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dockerfile: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ComposeSpec {
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub files: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service: Option<String>,
}

pub fn resolve_read_configuration(
    current_dir: &Path,
    workspace_folder: Option<&Path>,
    config: Option<&Path>,
    include_merged_configuration: bool,
) -> Result<ResolvedReadConfiguration, DecoError> {
    let workspace_folder = resolve_workspace_folder(current_dir, workspace_folder)?;
    let config_file = resolve_config_path(current_dir, &workspace_folder, config)?;
    let content = fs::read_to_string(&config_file).map_err(|error| {
        DecoError::new(
            ErrorCategory::Config,
            format!("failed to read config file `{}`", config_file.display()),
        )
        .with_details(error.to_string())
    })?;

    let configuration = parse_jsonc(&config_file, &content)?;
    let kind = infer_kind(&configuration);
    let normalized = normalize_config(&configuration, kind);
    let merged_configuration = include_merged_configuration.then(|| configuration.clone());

    Ok(ResolvedReadConfiguration {
        workspace_folder: workspace_folder.display().to_string(),
        config_file: config_file.display().to_string(),
        kind,
        normalized,
        configuration,
        merged_configuration,
    })
}

fn resolve_workspace_folder(
    current_dir: &Path,
    workspace_folder: Option<&Path>,
) -> Result<PathBuf, DecoError> {
    let candidate = workspace_folder
        .map(|path| absolutize(current_dir, path))
        .unwrap_or_else(|| current_dir.to_path_buf());

    if !candidate.exists() {
        return Err(DecoError::new(
            ErrorCategory::Config,
            format!("workspace folder `{}` does not exist", candidate.display()),
        ));
    }

    if !candidate.is_dir() {
        return Err(DecoError::new(
            ErrorCategory::Config,
            format!("workspace folder `{}` is not a directory", candidate.display()),
        ));
    }

    Ok(candidate)
}

fn resolve_config_path(
    current_dir: &Path,
    workspace_folder: &Path,
    config: Option<&Path>,
) -> Result<PathBuf, DecoError> {
    if let Some(config) = config {
        let explicit = absolutize(current_dir, config);
        if explicit.exists() {
            return Ok(explicit);
        }

        return Err(DecoError::new(
            ErrorCategory::Config,
            format!("config file `{}` does not exist", explicit.display()),
        ));
    }

    let preferred = workspace_folder.join(".devcontainer").join("devcontainer.json");
    if preferred.exists() {
        return Ok(preferred);
    }

    let fallback = workspace_folder.join(".devcontainer.json");
    if fallback.exists() {
        return Ok(fallback);
    }

    Err(DecoError::new(
        ErrorCategory::Config,
        format!(
            "dev container config not found under `{}`; expected `.devcontainer/devcontainer.json` or `.devcontainer.json`",
            workspace_folder.display()
        ),
    ))
}

fn parse_jsonc(config_file: &Path, content: &str) -> Result<Value, DecoError> {
    if content.trim().is_empty() {
        return Err(DecoError::new(
            ErrorCategory::Config,
            format!("config file `{}` is empty", config_file.display()),
        ));
    }

    jsonc_parser::parse_to_serde_value::<Value>(content, &Default::default()).map_err(|error| {
        DecoError::new(
            ErrorCategory::Config,
            format!("failed to parse config file `{}`", config_file.display()),
        )
        .with_details(error.to_string())
    })
}

fn infer_kind(configuration: &Value) -> DevcontainerConfigKind {
    if has_key(configuration, "dockerComposeFile") {
        return DevcontainerConfigKind::Compose;
    }

    if has_key(configuration, "dockerFile") || has_key(configuration, "build") {
        return DevcontainerConfigKind::Dockerfile;
    }

    if has_key(configuration, "image") {
        return DevcontainerConfigKind::Image;
    }

    DevcontainerConfigKind::Unknown
}

fn normalize_config(
    configuration: &Value,
    kind: DevcontainerConfigKind,
) -> NormalizedDevcontainerConfig {
    NormalizedDevcontainerConfig {
        name: string_field(configuration, "name"),
        image: string_field(configuration, "image"),
        build: normalize_build_spec(configuration, kind),
        compose: normalize_compose_spec(configuration, kind),
        workspace_folder: string_field(configuration, "workspaceFolder"),
        remote_user: string_field(configuration, "remoteUser"),
        remote_env: configuration.get("remoteEnv").cloned(),
    }
}

fn normalize_build_spec(configuration: &Value, kind: DevcontainerConfigKind) -> Option<BuildSpec> {
    if kind != DevcontainerConfigKind::Dockerfile {
        return None;
    }

    let build_value = configuration.get("build");
    let dockerfile = string_field(configuration, "dockerFile")
        .or_else(|| build_value.and_then(|value| value.get("dockerfile")).and_then(as_string));
    let context = build_value
        .and_then(|value| value.get("context"))
        .and_then(as_string)
        .or_else(|| string_field(configuration, "context"));

    Some(BuildSpec { dockerfile, context })
}

fn normalize_compose_spec(
    configuration: &Value,
    kind: DevcontainerConfigKind,
) -> Option<ComposeSpec> {
    if kind != DevcontainerConfigKind::Compose {
        return None;
    }

    let files = match configuration.get("dockerComposeFile") {
        Some(Value::String(value)) => vec![value.clone()],
        Some(Value::Array(values)) => values.iter().filter_map(as_string).collect(),
        _ => Vec::new(),
    };

    Some(ComposeSpec { files, service: string_field(configuration, "service") })
}

fn has_key(value: &Value, key: &str) -> bool {
    value.as_object().is_some_and(|object| object.contains_key(key))
}

fn string_field(value: &Value, key: &str) -> Option<String> {
    value.get(key).and_then(as_string)
}

fn as_string(value: &Value) -> Option<String> {
    value.as_str().map(ToOwned::to_owned)
}

fn absolutize(base: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() { path.to_path_buf() } else { base.join(path) }
}
