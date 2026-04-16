use std::fs;
use std::path::{Path, PathBuf};

use crate::model::{
    TemplateManifestDocument, TemplateManifestSummary, TemplatesMetadataResult, TemplatesScanMode,
};
use deco_core_model::{DecoError, ErrorCategory};

pub fn inspect_template_manifest_path(
    manifest_path: impl AsRef<Path>,
) -> Result<TemplatesMetadataResult, DecoError> {
    let manifest_path = manifest_path.as_ref();
    if !manifest_path.exists() {
        return Err(DecoError::new(
            ErrorCategory::Config,
            format!("template manifest path `{}` does not exist", manifest_path.display()),
        ));
    }

    if manifest_path.is_dir() {
        let mut manifests = Vec::new();
        for path in collect_manifest_files(manifest_path)? {
            manifests.push(inspect_manifest_file(&path)?);
        }
        manifests.sort_by(|left, right| left.path.cmp(&right.path));
        return Ok(TemplatesMetadataResult {
            scan_mode: TemplatesScanMode::Directory,
            manifest_path: manifest_path.display().to_string(),
            manifests,
        });
    }

    Ok(TemplatesMetadataResult {
        scan_mode: TemplatesScanMode::File,
        manifest_path: manifest_path.display().to_string(),
        manifests: vec![inspect_manifest_file(manifest_path)?],
    })
}

pub fn inspect_template_metadata(
    manifest_path: impl AsRef<Path>,
) -> Result<TemplatesMetadataResult, DecoError> {
    inspect_template_manifest_path(manifest_path)
}

fn inspect_manifest_file(manifest_path: &Path) -> Result<TemplateManifestSummary, DecoError> {
    let raw = fs::read_to_string(manifest_path).map_err(|error| {
        DecoError::new(
            ErrorCategory::Config,
            format!("failed to read template manifest `{}`", manifest_path.display()),
        )
        .with_details(error.to_string())
    })?;

    let document: TemplateManifestDocument = serde_json::from_str(&raw).map_err(|error| {
        DecoError::new(
            ErrorCategory::Config,
            format!("failed to parse template manifest `{}`", manifest_path.display()),
        )
        .with_details(error.to_string())
    })?;

    let source_dir = document
        .source_dir
        .as_ref()
        .map(|_| document.resolve_source_dir(manifest_path))
        .and_then(|result| result.ok())
        .map(|path| path.display().to_string());

    Ok(TemplateManifestSummary {
        path: manifest_path.display().to_string(),
        id: document.id,
        name: document.name,
        description: document.description,
        version: document.version,
        source_dir,
    })
}

fn collect_manifest_files(manifest_dir: &Path) -> Result<Vec<PathBuf>, DecoError> {
    let mut manifests = Vec::new();
    let mut stack = vec![manifest_dir.to_path_buf()];

    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir).map_err(|error| {
            DecoError::new(
                ErrorCategory::Config,
                format!("failed to read template manifest directory `{}`", dir.display()),
            )
            .with_details(error.to_string())
        })? {
            let entry = entry.map_err(|error| {
                DecoError::new(
                    ErrorCategory::Config,
                    format!("failed to read template manifest directory `{}`", dir.display()),
                )
                .with_details(error.to_string())
            })?;
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if path
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| extension.eq_ignore_ascii_case("json"))
            {
                manifests.push(path);
            }
        }
    }

    manifests.sort();
    Ok(manifests)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::*;

    #[test]
    fn metadata_scans_directory_recursively() {
        let temp = tempdir().expect("tempdir should be created");
        fs::create_dir_all(temp.path().join("nested")).expect("nested dir should exist");
        fs::write(temp.path().join("a.json"), r#"{"id":"a","name":"A","source_dir":"./a"}"#)
            .expect("manifest should be written");
        fs::write(
            temp.path().join("nested").join("b.json"),
            r#"{"id":"b","name":"B","source_dir":"./b"}"#,
        )
        .expect("manifest should be written");

        let result = inspect_template_manifest_path(temp.path()).expect("scan should succeed");
        assert_eq!(result.scan_mode, TemplatesScanMode::Directory);
        assert_eq!(result.manifests.len(), 2);
        assert_eq!(result.manifests[0].id.as_deref(), Some("a"));
        assert_eq!(result.manifests[1].id.as_deref(), Some("b"));
    }

    #[test]
    fn metadata_reads_single_manifest_file() {
        let temp = tempdir().expect("tempdir should be created");
        fs::create_dir_all(temp.path().join("template")).expect("template dir should exist");
        fs::write(
            temp.path().join("template.json"),
            r#"{"id":"template","source_dir":"./template"}"#,
        )
        .expect("manifest should be written");

        let result = inspect_template_manifest_path(temp.path().join("template.json"))
            .expect("scan should succeed");
        assert_eq!(result.scan_mode, TemplatesScanMode::File);
        assert_eq!(result.manifests.len(), 1);
        assert!(result.manifests[0].source_dir.is_some());
    }
}
