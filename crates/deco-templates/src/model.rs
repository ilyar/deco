use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use deco_core_model::{DecoError, ErrorCategory};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TemplatesScanMode {
    File,
    Directory,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TemplateManifestDocument {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_dir: Option<PathBuf>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TemplateManifestSummary {
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_dir: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TemplatesMetadataResult {
    pub scan_mode: TemplatesScanMode,
    pub manifest_path: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub manifests: Vec<TemplateManifestSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TemplateCopyEntry {
    pub source: String,
    pub target: String,
    pub bytes_copied: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TemplateApplyResult {
    pub manifest_path: String,
    pub source_dir: String,
    pub target_dir: String,
    pub files_copied: usize,
    pub bytes_copied: u64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub copied_entries: Vec<TemplateCopyEntry>,
}

impl TemplateManifestDocument {
    pub fn resolve_source_dir(
        &self,
        manifest_path: impl AsRef<Path>,
    ) -> Result<PathBuf, DecoError> {
        let manifest_path = manifest_path.as_ref();
        let source_dir = self.source_dir.as_ref().ok_or_else(|| {
            DecoError::new(
                ErrorCategory::Config,
                format!("template manifest `{}` is missing `source_dir`", manifest_path.display()),
            )
        })?;

        let resolved = if source_dir.is_absolute() {
            source_dir.clone()
        } else {
            manifest_path
                .parent()
                .map(|parent| parent.join(source_dir))
                .unwrap_or_else(|| source_dir.clone())
        };

        Ok(fs::canonicalize(&resolved).unwrap_or(resolved))
    }
}
