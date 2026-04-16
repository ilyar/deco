use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use deco_config::{DevcontainerConfigKind, resolve_read_configuration};
use deco_features::{
    FeatureDependencyResolutionResult, generate_feature_lockfile, resolve_feature_dependencies,
};
use deco_lockfile::{
    CURRENT_LOCKFILE_SCHEMA_VERSION, FeatureLockfileDocument, LockfileDocument,
    LockfileParseError, LockfileSource, LockfileTarget, parse_feature_lockfile_json,
    parse_lockfile_json,
};
use serde::Serialize;

use crate::cli::LockfileArgs;

use deco_core_model::{DecoError, ErrorCategory};

#[derive(Debug, Clone)]
pub(crate) struct ResolvedLockfileContext {
    pub lockfile_path: PathBuf,
    pub resolved_config: Option<deco_config::ResolvedReadConfiguration>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum LockfileFormat {
    DecoLegacy,
    DevcontainerFeature,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct OutdatedResult {
    pub lockfile: String,
    pub format: LockfileFormat,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<LockfileSource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config_kind: Option<DevcontainerConfigKind>,
    pub schema_version: u32,
    pub current_schema_version: u32,
    pub target_count: usize,
    pub upgrade_needed: bool,
    pub valid: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub targets: Vec<LockfileTarget>,
    pub feature_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub feature_lockfile: Option<FeatureLockfileDocument>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config_feature_graph: Option<FeatureDependencyResolutionResult>,
}

pub fn run(args: LockfileArgs) -> Result<OutdatedResult, DecoError> {
    let context = resolve_lockfile_context(&args)?;
    let lockfile_path = &context.lockfile_path;
    let content = fs::read_to_string(&lockfile_path).map_err(|error| {
        DecoError::new(ErrorCategory::Config, "failed to read lockfile").with_details(format!(
            "{}: {}",
            lockfile_path.display(),
            error
        ))
    })?;

    match parse_lockfile_json(&content) {
        Ok(document) => Ok(outdated_result(
            lockfile_path.as_path(),
            &document,
            true,
            context.resolved_config.as_ref(),
        )),
        Err(LockfileParseError::Json(_) | LockfileParseError::Invalid(_)) => {
            let document = parse_feature_lockfile_json(&content)
                .map_err(|error| map_lockfile_error(lockfile_path, error))?;
            Ok(feature_outdated_result(
                lockfile_path.as_path(),
                &document,
                context.resolved_config.as_ref(),
            ))
        }
        Err(LockfileParseError::UnsupportedSchemaVersion { .. }) => {
            let document = deserialize_lockfile_document(&content, lockfile_path.as_path())?;
            validate_lockfile_summary(&document).map_err(|message| {
                DecoError::new(ErrorCategory::Config, "invalid lockfile").with_details(format!(
                    "{}: {}",
                    lockfile_path.display(),
                    message
                ))
            })?;
            Ok(outdated_result(
                lockfile_path.as_path(),
                &document,
                false,
                context.resolved_config.as_ref(),
            ))
        }
        Err(error) => Err(map_lockfile_error(&lockfile_path, error)),
    }
}

pub(crate) fn resolve_lockfile_context(
    args: &LockfileArgs,
) -> Result<ResolvedLockfileContext, DecoError> {
    if let Some(lockfile) = &args.lockfile {
        return Ok(ResolvedLockfileContext {
            lockfile_path: lockfile.clone(),
            resolved_config: None,
        });
    }

    let current_dir = env::current_dir().map_err(|error| {
        DecoError::new(ErrorCategory::Internal, "failed to determine current working directory")
            .with_details(error.to_string())
    })?;
    let resolved_config = resolve_read_configuration(
        &current_dir,
        args.workspace_folder.as_deref(),
        args.config.as_deref(),
        false,
    )?;
    let lockfile_path = default_lockfile_path(Path::new(&resolved_config.config_file));
    Ok(ResolvedLockfileContext { lockfile_path, resolved_config: Some(resolved_config) })
}

fn default_lockfile_path(config_path: &Path) -> PathBuf {
    let config_dir = config_path.parent().unwrap_or_else(|| Path::new("."));
    let file_name = config_path.file_name().and_then(|value| value.to_str()).unwrap_or_default();
    let lockfile_name = if file_name.starts_with('.') {
        ".devcontainer-lock.json"
    } else {
        "devcontainer-lock.json"
    };
    config_dir.join(lockfile_name)
}

fn deserialize_lockfile_document(
    content: &str,
    path: &Path,
) -> Result<LockfileDocument, DecoError> {
    serde_json::from_str(content).map_err(|error| {
        DecoError::new(ErrorCategory::Config, "failed to parse lockfile").with_details(format!(
            "{}: {}",
            path.display(),
            error
        ))
    })
}

pub(crate) fn validate_lockfile_summary(document: &LockfileDocument) -> Result<(), String> {
    if document.source.workspace_folder.trim().is_empty() {
        return Err("source.workspace_folder must not be empty".to_string());
    }

    if document.source.config_file.trim().is_empty() {
        return Err("source.config_file must not be empty".to_string());
    }

    let mut seen = std::collections::BTreeSet::new();
    for target in &document.targets {
        if target.name.trim().is_empty() {
            return Err("target.name must not be empty".to_string());
        }
        if target.reference.trim().is_empty() {
            return Err(format!("target `{}` reference must not be empty", target.name));
        }
        if !seen.insert(target.name.as_str()) {
            return Err(format!("duplicate target name `{}`", target.name));
        }
    }

    Ok(())
}

fn outdated_result(
    path: &Path,
    document: &LockfileDocument,
    valid: bool,
    resolved_config: Option<&deco_config::ResolvedReadConfiguration>,
) -> OutdatedResult {
    OutdatedResult {
        lockfile: path.display().to_string(),
        format: LockfileFormat::DecoLegacy,
        source: Some(document.source.clone()),
        config_kind: resolved_config.map(|resolved| resolved.kind),
        schema_version: document.schema_version,
        current_schema_version: CURRENT_LOCKFILE_SCHEMA_VERSION,
        target_count: document.targets.len(),
        upgrade_needed: document.schema_version != CURRENT_LOCKFILE_SCHEMA_VERSION,
        valid,
        targets: document.targets.clone(),
        feature_count: 0,
        feature_lockfile: None,
        config_feature_graph: resolved_config
            .and_then(|resolved| resolve_feature_dependencies(None, Some(resolved)).ok()),
    }
}

fn feature_outdated_result(
    path: &Path,
    document: &FeatureLockfileDocument,
    resolved_config: Option<&deco_config::ResolvedReadConfiguration>,
) -> OutdatedResult {
    OutdatedResult {
        lockfile: path.display().to_string(),
        format: LockfileFormat::DevcontainerFeature,
        source: resolved_config.map(|resolved| LockfileSource {
            workspace_folder: resolved.workspace_folder.clone(),
            config_file: resolved.config_file.clone(),
        }),
        config_kind: resolved_config.map(|resolved| resolved.kind),
        schema_version: CURRENT_LOCKFILE_SCHEMA_VERSION,
        current_schema_version: CURRENT_LOCKFILE_SCHEMA_VERSION,
        target_count: 0,
        upgrade_needed: false,
        valid: true,
        targets: Vec::new(),
        feature_count: document.features.len(),
        feature_lockfile: Some(document.clone()),
        config_feature_graph: resolved_config
            .and_then(|resolved| resolve_feature_dependencies(None, Some(resolved)).ok()),
    }
}

pub(crate) fn synthesized_feature_lockfile(
    context: &ResolvedLockfileContext,
) -> Result<Option<FeatureLockfileDocument>, DecoError> {
    let Some(resolved_config) = context.resolved_config.as_ref() else {
        return Ok(None);
    };
    let feature_lockfile = generate_feature_lockfile(resolved_config)?;
    if feature_lockfile.is_empty() {
        return Ok(None);
    }
    Ok(Some(feature_lockfile))
}

fn map_lockfile_error(path: &Path, error: LockfileParseError) -> DecoError {
    match error {
        LockfileParseError::Io { source, .. } => DecoError::new(
            ErrorCategory::Config,
            "failed to read lockfile",
        )
        .with_details(format!("{}: {}", path.display(), source)),
        LockfileParseError::Json(error) => DecoError::new(
            ErrorCategory::Config,
            "failed to parse lockfile",
        )
        .with_details(format!("{}: {}", path.display(), error)),
        LockfileParseError::UnsupportedSchemaVersion { expected, found } => {
            DecoError::new(ErrorCategory::Config, "unsupported lockfile schema version")
                .with_details(format!("{}: expected {}, found {}", path.display(), expected, found))
        }
        LockfileParseError::Invalid(message) => DecoError::new(
            ErrorCategory::Config,
            "invalid lockfile",
        )
        .with_details(format!("{}: {}", path.display(), message)),
    }
}
