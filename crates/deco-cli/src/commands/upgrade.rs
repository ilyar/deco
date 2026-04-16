use std::fs;
use std::path::Path;

use deco_config::DevcontainerConfigKind;
use deco_features::{FeatureDependencyResolutionResult, resolve_feature_dependencies};
use deco_lockfile::{
    CURRENT_LOCKFILE_SCHEMA_VERSION, FeatureLockfileDocument, LockfileDocument, LockfileParseError,
    LockfileSource, LockfileTarget, parse_feature_lockfile_json, parse_lockfile_json,
    serialize_feature_lockfile_json, serialize_lockfile_json,
};
use serde::Serialize;

use crate::cli::UpgradeArgs;
use crate::commands::outdated::{
    LockfileFormat, resolve_lockfile_context, synthesized_feature_lockfile,
    validate_lockfile_summary,
};

use deco_core_model::{DecoError, ErrorCategory};

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct UpgradeResult {
    pub lockfile: String,
    pub format: LockfileFormat,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<LockfileSource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config_kind: Option<DevcontainerConfigKind>,
    pub before_schema_version: u32,
    pub schema_version: u32,
    pub current_schema_version: u32,
    pub target_count: usize,
    pub dry_run: bool,
    pub written: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub targets: Vec<LockfileTarget>,
    pub feature_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub feature_lockfile: Option<FeatureLockfileDocument>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config_feature_graph: Option<FeatureDependencyResolutionResult>,
}

pub fn run(args: UpgradeArgs) -> Result<UpgradeResult, DecoError> {
    let context = resolve_lockfile_context(&args.lockfile)?;
    let lockfile_path = &context.lockfile_path;
    let resolved_config = context.resolved_config.as_ref();
    let existing_content = fs::read_to_string(lockfile_path).ok();

    if let Some(feature_lockfile) = synthesized_feature_lockfile(&context)? {
        let serialized = serialize_feature_lockfile_json(&feature_lockfile)
            .map_err(|error| map_lockfile_error(lockfile_path, error))?;
        let written = if args.dry_run {
            false
        } else {
            fs::write(lockfile_path, serialized).map_err(|error| {
                DecoError::new(ErrorCategory::Config, "failed to write lockfile")
                    .with_details(format!("{}: {}", lockfile_path.display(), error))
            })?;
            true
        };

        return Ok(UpgradeResult {
            lockfile: lockfile_path.display().to_string(),
            format: LockfileFormat::DevcontainerFeature,
            source: resolved_config.map(|resolved| LockfileSource {
                workspace_folder: resolved.workspace_folder.clone(),
                config_file: resolved.config_file.clone(),
            }),
            config_kind: resolved_config.map(|resolved| resolved.kind),
            before_schema_version: parse_feature_schema_version(existing_content.as_deref()),
            schema_version: CURRENT_LOCKFILE_SCHEMA_VERSION,
            current_schema_version: CURRENT_LOCKFILE_SCHEMA_VERSION,
            target_count: 0,
            dry_run: args.dry_run,
            written,
            targets: Vec::new(),
            feature_count: feature_lockfile.features.len(),
            feature_lockfile: Some(feature_lockfile),
            config_feature_graph: resolved_config
                .and_then(|resolved| resolve_feature_dependencies(None, Some(resolved)).ok()),
        });
    }

    let document = read_lockfile(lockfile_path)?;
    let before_schema_version = document.schema_version;
    let normalized = normalize_lockfile(document);
    let serialized = serialize_lockfile_json(&normalized)
        .map_err(|error| map_lockfile_error(lockfile_path, error))?;

    let written =
        if args.dry_run {
            false
        } else {
            fs::write(lockfile_path, serialized).map_err(|error| {
                DecoError::new(ErrorCategory::Config, "failed to write lockfile")
                    .with_details(format!("{}: {}", lockfile_path.display(), error))
            })?;
            true
        };

    Ok(UpgradeResult {
        lockfile: lockfile_path.display().to_string(),
        format: LockfileFormat::DecoLegacy,
        source: Some(normalized.source.clone()),
        config_kind: resolved_config.map(|resolved| resolved.kind),
        before_schema_version,
        schema_version: normalized.schema_version,
        current_schema_version: CURRENT_LOCKFILE_SCHEMA_VERSION,
        target_count: normalized.targets.len(),
        dry_run: args.dry_run,
        written,
        targets: normalized.targets.clone(),
        feature_count: 0,
        feature_lockfile: None,
        config_feature_graph: context
            .resolved_config
            .as_ref()
            .and_then(|resolved| resolve_feature_dependencies(None, Some(resolved)).ok()),
    })
}

fn read_lockfile(path: &Path) -> Result<LockfileDocument, DecoError> {
    let content = fs::read_to_string(path).map_err(|error| {
        DecoError::new(ErrorCategory::Config, "failed to read lockfile").with_details(format!(
            "{}: {}",
            path.display(),
            error
        ))
    })?;

    match parse_lockfile_json(&content) {
        Ok(document) => Ok(document),
        Err(deco_lockfile::LockfileParseError::UnsupportedSchemaVersion { .. }) => {
            let document: LockfileDocument = serde_json::from_str(&content).map_err(|error| {
                DecoError::new(ErrorCategory::Config, "failed to parse lockfile")
                    .with_details(format!("{}: {}", path.display(), error))
            })?;
            validate_lockfile_summary(&document).map_err(|message| {
                DecoError::new(ErrorCategory::Config, "invalid lockfile").with_details(format!(
                    "{}: {}",
                    path.display(),
                    message
                ))
            })?;
            Ok(document)
        }
        Err(
            deco_lockfile::LockfileParseError::Json(_)
            | deco_lockfile::LockfileParseError::Invalid(_),
        ) => {
            let feature_lockfile = parse_feature_lockfile_json(&content).map_err(|error| {
                DecoError::new(ErrorCategory::Config, "failed to parse lockfile")
                    .with_details(format!("{}: {}", path.display(), error))
            })?;
            Err(DecoError::new(
                ErrorCategory::Config,
                "feature lockfile upgrade requires workspace-based config context",
            )
            .with_details(format!(
                "{}: parsed {} feature entries; rerun with --workspace-folder or --config",
                path.display(),
                feature_lockfile.features.len()
            )))
        }
        Err(error) => Err(map_lockfile_error(path, error)),
    }
}

fn normalize_lockfile(mut document: LockfileDocument) -> LockfileDocument {
    document.schema_version = CURRENT_LOCKFILE_SCHEMA_VERSION;
    document
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

fn parse_feature_schema_version(content: Option<&str>) -> u32 {
    content
        .and_then(|content| parse_feature_lockfile_json(content).ok())
        .map(|_| CURRENT_LOCKFILE_SCHEMA_VERSION)
        .unwrap_or(0)
}
