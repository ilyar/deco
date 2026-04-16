use std::fs;
use std::path::Path;

use deco_core_model::{DecoError, ErrorCategory};

use crate::model::{TemplateApplyResult, TemplateCopyEntry, TemplateManifestDocument};

pub fn apply_template(
    manifest_path: impl AsRef<Path>,
    target_dir: impl AsRef<Path>,
) -> Result<TemplateApplyResult, DecoError> {
    let manifest_path = manifest_path.as_ref();
    let target_dir = target_dir.as_ref();
    if manifest_path.is_dir() {
        return Err(DecoError::new(
            ErrorCategory::Config,
            format!(
                "template apply requires a manifest file, got directory `{}`",
                manifest_path.display()
            ),
        ));
    }

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
    let source_dir = document.resolve_source_dir(manifest_path)?;

    copy_directory_tree(&source_dir, target_dir).map(|(entries, bytes_copied)| {
        TemplateApplyResult {
            manifest_path: manifest_path.display().to_string(),
            source_dir: source_dir.display().to_string(),
            target_dir: target_dir.display().to_string(),
            files_copied: entries.len(),
            bytes_copied,
            copied_entries: entries,
        }
    })
}

pub fn copy_directory_tree(
    source_dir: impl AsRef<Path>,
    target_dir: impl AsRef<Path>,
) -> Result<(Vec<TemplateCopyEntry>, u64), DecoError> {
    let source_dir = source_dir.as_ref();
    let target_dir = target_dir.as_ref();

    if !source_dir.exists() {
        return Err(DecoError::new(
            ErrorCategory::Config,
            format!("template source directory `{}` does not exist", source_dir.display()),
        ));
    }
    if !source_dir.is_dir() {
        return Err(DecoError::new(
            ErrorCategory::Config,
            format!("template source `{}` is not a directory", source_dir.display()),
        ));
    }

    fs::create_dir_all(target_dir).map_err(|error| {
        DecoError::new(
            ErrorCategory::Config,
            format!("failed to create target directory `{}`", target_dir.display()),
        )
        .with_details(error.to_string())
    })?;

    let mut entries = Vec::new();
    let mut bytes_copied = 0u64;
    copy_directory_recursive(source_dir, source_dir, target_dir, &mut entries, &mut bytes_copied)?;
    Ok((entries, bytes_copied))
}

fn copy_directory_recursive(
    root: &Path,
    source_dir: &Path,
    target_dir: &Path,
    entries: &mut Vec<TemplateCopyEntry>,
    bytes_copied: &mut u64,
) -> Result<(), DecoError> {
    for entry in fs::read_dir(source_dir).map_err(|error| {
        DecoError::new(
            ErrorCategory::Config,
            format!("failed to read template source `{}`", source_dir.display()),
        )
        .with_details(error.to_string())
    })? {
        let entry = entry.map_err(|error| {
            DecoError::new(
                ErrorCategory::Config,
                format!("failed to read template source `{}`", source_dir.display()),
            )
            .with_details(error.to_string())
        })?;
        let source_path = entry.path();
        let relative = source_path.strip_prefix(root).unwrap_or(&source_path);
        let target_path = target_dir.join(relative);

        if source_path.is_dir() {
            fs::create_dir_all(&target_path).map_err(|error| {
                DecoError::new(
                    ErrorCategory::Config,
                    format!("failed to create directory `{}`", target_path.display()),
                )
                .with_details(error.to_string())
            })?;
            copy_directory_recursive(root, &source_path, target_dir, entries, bytes_copied)?;
            continue;
        }

        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent).map_err(|error| {
                DecoError::new(
                    ErrorCategory::Config,
                    format!("failed to create directory `{}`", parent.display()),
                )
                .with_details(error.to_string())
            })?;
        }

        let copied = fs::copy(&source_path, &target_path).map_err(|error| {
            DecoError::new(
                ErrorCategory::Config,
                format!(
                    "failed to copy template file `{}` to `{}`",
                    source_path.display(),
                    target_path.display()
                ),
            )
            .with_details(error.to_string())
        })?;

        entries.push(TemplateCopyEntry {
            source: source_path.display().to_string(),
            target: target_path.display().to_string(),
            bytes_copied: copied,
        });
        *bytes_copied += copied;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::*;

    #[test]
    fn copies_template_tree_into_target_directory() {
        let temp = tempdir().expect("tempdir should be created");
        let template_dir = temp.path().join("template");
        let target_dir = temp.path().join("output");
        fs::create_dir_all(template_dir.join("nested")).expect("template tree should exist");
        fs::write(template_dir.join("hello.txt"), "hello").expect("file should be written");
        fs::write(template_dir.join("nested").join("world.txt"), "world")
            .expect("file should be written");

        let manifest = temp.path().join("template.json");
        fs::write(&manifest, r#"{"id":"sample","source_dir":"./template"}"#)
            .expect("manifest should be written");

        let result = apply_template(&manifest, &target_dir).expect("apply should succeed");

        assert_eq!(result.files_copied, 2);
        assert!(target_dir.join("hello.txt").exists());
        assert!(target_dir.join("nested").join("world.txt").exists());
        assert_eq!(
            fs::read_to_string(target_dir.join("hello.txt")).expect("file should be readable"),
            "hello"
        );
        assert_eq!(result.copied_entries.len(), 2);
    }

    #[test]
    fn rejects_manifest_directories() {
        let temp = tempdir().expect("tempdir should be created");
        let result = apply_template(temp.path(), temp.path().join("out"));
        assert!(result.is_err());
    }
}
