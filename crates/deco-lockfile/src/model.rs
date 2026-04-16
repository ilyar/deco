use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

pub const CURRENT_LOCKFILE_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LockfileDocument {
    pub schema_version: u32,
    pub source: LockfileSource,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub targets: Vec<LockfileTarget>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<LockfileMetadata>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeatureLockfileDocument {
    pub features: BTreeMap<String, FeatureLockfileEntry>,
}

impl FeatureLockfileDocument {
    pub fn is_empty(&self) -> bool {
        self.features.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeatureLockfileEntry {
    pub version: String,
    pub resolved: String,
    pub integrity: String,
    #[serde(rename = "dependsOn", skip_serializing_if = "Option::is_none")]
    pub depends_on: Option<Vec<String>>,
}

impl LockfileDocument {
    pub fn new(source: LockfileSource) -> Self {
        Self {
            schema_version: CURRENT_LOCKFILE_SCHEMA_VERSION,
            source,
            targets: Vec::new(),
            metadata: None,
        }
    }

    pub fn with_target(mut self, target: LockfileTarget) -> Self {
        self.targets.push(target);
        self
    }

    pub fn push_target(&mut self, target: LockfileTarget) {
        self.targets.push(target);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LockfileSource {
    pub workspace_folder: String,
    pub config_file: String,
}

impl LockfileSource {
    pub fn new(workspace_folder: impl Into<String>, config_file: impl Into<String>) -> Self {
        Self { workspace_folder: workspace_folder.into(), config_file: config_file.into() }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LockfileMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generated_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generated_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LockfileTargetKind {
    Image,
    Dockerfile,
    Compose,
    Feature,
    Template,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LockfileTarget {
    pub name: String,
    pub kind: LockfileTargetKind,
    pub reference: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolved_reference: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub digest: Option<String>,
}

impl LockfileTarget {
    pub fn new(
        name: impl Into<String>,
        kind: LockfileTargetKind,
        reference: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            kind,
            reference: reference.into(),
            resolved_reference: None,
            digest: None,
        }
    }

    pub fn with_resolved_reference(mut self, resolved_reference: impl Into<String>) -> Self {
        self.resolved_reference = Some(resolved_reference.into());
        self
    }

    pub fn with_digest(mut self, digest: impl Into<String>) -> Self {
        self.digest = Some(digest.into());
        self
    }
}
