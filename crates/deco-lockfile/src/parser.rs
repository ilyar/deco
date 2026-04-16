use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use thiserror::Error;

use crate::{FeatureLockfileDocument, LockfileDocument};
use crate::model::CURRENT_LOCKFILE_SCHEMA_VERSION;

#[derive(Debug, Error)]
pub enum LockfileParseError {
    #[error("failed to read lockfile `{path}`")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse lockfile JSON")]
    Json(#[from] serde_json::Error),
    #[error("unsupported lockfile schema version {found}, expected {expected}")]
    UnsupportedSchemaVersion { expected: u32, found: u32 },
    #[error("invalid lockfile: {0}")]
    Invalid(String),
}

pub fn parse_lockfile_json(content: &str) -> Result<LockfileDocument, LockfileParseError> {
    let document: LockfileDocument = serde_json::from_str(content)?;
    validate_lockfile_document(&document)?;
    Ok(document)
}

pub fn parse_feature_lockfile_json(
    content: &str,
) -> Result<FeatureLockfileDocument, LockfileParseError> {
    let document: FeatureLockfileDocument = serde_json::from_str(content)?;
    validate_feature_lockfile_document(&document)?;
    Ok(document)
}

pub fn parse_lockfile_path(path: impl AsRef<Path>) -> Result<LockfileDocument, LockfileParseError> {
    let path = path.as_ref();
    let content = fs::read_to_string(path)
        .map_err(|source| LockfileParseError::Io { path: path.display().to_string(), source })?;
    parse_lockfile_json(&content)
}

pub fn serialize_lockfile_json(document: &LockfileDocument) -> Result<String, LockfileParseError> {
    validate_lockfile_document(document)?;
    Ok(serde_json::to_string_pretty(document)?)
}

pub fn serialize_feature_lockfile_json(
    document: &FeatureLockfileDocument,
) -> Result<String, LockfileParseError> {
    validate_feature_lockfile_document(document)?;
    Ok(serde_json::to_string_pretty(document)?)
}

pub fn validate_lockfile_document(document: &LockfileDocument) -> Result<(), LockfileParseError> {
    if document.schema_version != CURRENT_LOCKFILE_SCHEMA_VERSION {
        return Err(LockfileParseError::UnsupportedSchemaVersion {
            expected: CURRENT_LOCKFILE_SCHEMA_VERSION,
            found: document.schema_version,
        });
    }

    if document.source.workspace_folder.trim().is_empty() {
        return Err(LockfileParseError::Invalid(
            "source.workspace_folder must not be empty".to_string(),
        ));
    }

    if document.source.config_file.trim().is_empty() {
        return Err(LockfileParseError::Invalid(
            "source.config_file must not be empty".to_string(),
        ));
    }

    let mut names = BTreeSet::new();
    for target in &document.targets {
        if target.name.trim().is_empty() {
            return Err(LockfileParseError::Invalid("target.name must not be empty".to_string()));
        }

        if target.reference.trim().is_empty() {
            return Err(LockfileParseError::Invalid(format!(
                "target `{}` reference must not be empty",
                target.name
            )));
        }

        if !names.insert(target.name.as_str()) {
            return Err(LockfileParseError::Invalid(format!(
                "duplicate target name `{}`",
                target.name
            )));
        }
    }

    Ok(())
}

pub fn validate_feature_lockfile_document(
    document: &FeatureLockfileDocument,
) -> Result<(), LockfileParseError> {
    for (id, feature) in &document.features {
        if id.trim().is_empty() {
            return Err(LockfileParseError::Invalid(
                "feature id must not be empty".to_string(),
            ));
        }
        if feature.version.trim().is_empty() {
            return Err(LockfileParseError::Invalid(format!(
                "feature `{id}` version must not be empty"
            )));
        }
        if feature.resolved.trim().is_empty() {
            return Err(LockfileParseError::Invalid(format!(
                "feature `{id}` resolved must not be empty"
            )));
        }
        if feature.integrity.trim().is_empty() {
            return Err(LockfileParseError::Invalid(format!(
                "feature `{id}` integrity must not be empty"
            )));
        }
    }

    Ok(())
}
