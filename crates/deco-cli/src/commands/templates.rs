use std::env;
use std::path::{Path, PathBuf};

use deco_core_model::{DecoError, ErrorCategory};
use deco_templates::{
    TemplateApplyResult, TemplatesMetadataResult, apply_template, inspect_template_manifest_path,
    resolve_template_manifest_by_id, select_single_manifest_path,
};

use crate::cli::{TemplatesApplyArgs, TemplatesArgs, TemplatesCommand, TemplatesMetadataArgs};

pub fn run(args: TemplatesArgs) -> Result<TemplatesCommandResult, DecoError> {
    match args.command {
        TemplatesCommand::Metadata(args) => {
            run_metadata(args).map(TemplatesCommandResult::Metadata)
        }
        TemplatesCommand::Apply(args) => run_apply(args).map(TemplatesCommandResult::Apply),
    }
}

pub fn run_metadata(args: TemplatesMetadataArgs) -> Result<TemplatesMetadataResult, DecoError> {
    let current_dir = env::current_dir().map_err(|error| {
        DecoError::new(ErrorCategory::Internal, "failed to determine current working directory")
            .with_details(error.to_string())
    })?;
    let manifest_path = resolve_template_input_path(
        &current_dir,
        args.manifest_path,
        args.template_id,
        "template metadata",
    )?;
    inspect_template_manifest_path(&manifest_path)
}

pub fn run_apply(args: TemplatesApplyArgs) -> Result<TemplateApplyResult, DecoError> {
    let current_dir = env::current_dir().map_err(|error| {
        DecoError::new(ErrorCategory::Internal, "failed to determine current working directory")
            .with_details(error.to_string())
    })?;
    let manifest_path = resolve_template_input_path(
        &current_dir,
        args.manifest_path,
        args.template_id,
        "template apply",
    )?;
    let manifest_path = select_single_manifest_path(&manifest_path)?;
    let target_dir = resolve_target_dir(&current_dir, args.workspace_folder, args.target_dir)?;
    apply_template(&manifest_path, &target_dir)
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(tag = "mode", rename_all = "kebab-case")]
pub enum TemplatesCommandResult {
    Metadata(TemplatesMetadataResult),
    Apply(TemplateApplyResult),
}

fn absolutize(base: &Path, value: PathBuf) -> PathBuf {
    if value.is_absolute() { value } else { base.join(value) }
}

fn resolve_template_input_path(
    current_dir: &Path,
    manifest_path: Option<PathBuf>,
    template_id: Option<PathBuf>,
    command_name: &str,
) -> Result<PathBuf, DecoError> {
    let manifest_path = manifest_path.map(|path| absolutize(current_dir, path));
    if let Some(template_id) = template_id {
        let template_path = absolutize(current_dir, template_id.clone());
        if template_path.exists() {
            return Ok(template_path);
        }

        let collection_dir = manifest_path.ok_or_else(|| {
            DecoError::new(
                ErrorCategory::Config,
                format!(
                    "{command_name} requires `--manifest-path <collection-dir>` when `--template-id` is a logical id"
                ),
            )
        })?;
        return resolve_template_manifest_by_id(&collection_dir, &template_id.to_string_lossy());
    }

    manifest_path.ok_or_else(|| {
        DecoError::new(
            ErrorCategory::Config,
            format!("{command_name} requires `--template-id` or `--manifest-path`"),
        )
    })
}

fn resolve_target_dir(
    current_dir: &Path,
    workspace_folder: Option<PathBuf>,
    target_dir: Option<PathBuf>,
) -> Result<PathBuf, DecoError> {
    if let Some(target_dir) = target_dir {
        return Ok(absolutize(current_dir, target_dir));
    }

    if let Some(workspace_folder) = workspace_folder {
        return Ok(absolutize(current_dir, workspace_folder));
    }

    Err(DecoError::new(
        ErrorCategory::Config,
        "template apply requires `--target-dir` or `--workspace-folder`",
    ))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::*;

    #[test]
    fn metadata_scans_single_manifest_file() {
        let temp = tempdir().expect("tempdir should be created");
        let template_dir = temp.path().join("template");
        fs::create_dir_all(&template_dir).expect("template dir should exist");
        fs::write(
            temp.path().join("template.json"),
            r#"{
              "id": "sample",
              "name": "Sample",
              "source_dir": "./template"
            }"#,
        )
        .expect("manifest should be written");

        let result = run_metadata(TemplatesMetadataArgs {
            manifest_path: Some(temp.path().join("template.json")),
            template_id: None,
        })
        .expect("metadata should succeed");

        assert_eq!(result.scan_mode, deco_templates::TemplatesScanMode::File);
        assert_eq!(result.manifests.len(), 1);
        assert_eq!(result.manifests[0].id.as_deref(), Some("sample"));
    }

    #[test]
    fn apply_copies_template_contents() {
        let temp = tempdir().expect("tempdir should be created");
        let template_dir = temp.path().join("template");
        let target_dir = temp.path().join("target");
        fs::create_dir_all(&template_dir).expect("template dir should exist");
        fs::write(template_dir.join("hello.txt"), "hello").expect("file should be written");
        fs::write(
            temp.path().join("template.json"),
            r#"{"id":"sample","source_dir":"./template"}"#,
        )
        .expect("manifest should be written");

        let result = run_apply(TemplatesApplyArgs {
            manifest_path: Some(temp.path().join("template.json")),
            template_id: None,
            workspace_folder: None,
            target_dir: Some(target_dir.clone()),
        })
        .expect("apply should succeed");

        assert_eq!(result.files_copied, 1);
        assert!(target_dir.join("hello.txt").exists());
    }

    #[test]
    fn apply_accepts_template_id_directory_and_workspace_folder() {
        let temp = tempdir().expect("tempdir should be created");
        let template_root = temp.path().join("template");
        let source_dir = template_root.join("src");
        let workspace_folder = temp.path().join("workspace");
        fs::create_dir_all(&source_dir).expect("source dir should exist");
        fs::write(source_dir.join("hello.txt"), "hello").expect("file should be written");
        fs::write(
            template_root.join("template.json"),
            r#"{
              "id": "sample",
              "source_dir": "./src"
            }"#,
        )
        .expect("manifest should be written");

        let result = run_apply(TemplatesApplyArgs {
            manifest_path: None,
            template_id: Some(template_root.clone()),
            workspace_folder: Some(workspace_folder.clone()),
            target_dir: None,
        })
        .expect("apply should succeed");

        assert_eq!(result.files_copied, 1);
        assert_eq!(result.target_dir, workspace_folder.display().to_string());
        assert!(workspace_folder.join("hello.txt").exists());
    }

    #[test]
    fn metadata_resolves_template_id_from_collection() {
        let temp = tempdir().expect("tempdir should be created");
        let collection_dir = temp.path().join("collection");
        let selected_source = collection_dir.join("selected").join("src");
        let other_source = collection_dir.join("other").join("src");
        fs::create_dir_all(&selected_source).expect("selected source should exist");
        fs::create_dir_all(&other_source).expect("other source should exist");
        fs::write(
            collection_dir.join("selected.json"),
            r#"{"id":"selected","name":"Selected","source_dir":"./selected/src"}"#,
        )
        .expect("selected manifest should be written");
        fs::write(
            collection_dir.join("other.json"),
            r#"{"id":"other","name":"Other","source_dir":"./other/src"}"#,
        )
        .expect("other manifest should be written");

        let result = run_metadata(TemplatesMetadataArgs {
            manifest_path: Some(collection_dir),
            template_id: Some(PathBuf::from("selected")),
        })
        .expect("metadata should resolve logical id");

        assert_eq!(result.scan_mode, deco_templates::TemplatesScanMode::File);
        assert_eq!(result.manifests.len(), 1);
        assert_eq!(result.manifests[0].id.as_deref(), Some("selected"));
    }

    #[test]
    fn apply_resolves_template_id_from_collection() {
        let temp = tempdir().expect("tempdir should be created");
        let collection_dir = temp.path().join("collection");
        let selected_source = collection_dir.join("selected").join("src");
        let workspace_folder = temp.path().join("workspace");
        fs::create_dir_all(selected_source.join("nested")).expect("selected source should exist");
        fs::write(selected_source.join("hello.txt"), "hello").expect("file should be written");
        fs::write(selected_source.join("nested").join("world.txt"), "world")
            .expect("file should be written");
        fs::write(
            collection_dir.join("selected.json"),
            r#"{"id":"selected","source_dir":"./selected/src"}"#,
        )
        .expect("selected manifest should be written");

        let result = run_apply(TemplatesApplyArgs {
            manifest_path: Some(collection_dir),
            template_id: Some(PathBuf::from("selected")),
            workspace_folder: Some(workspace_folder.clone()),
            target_dir: None,
        })
        .expect("apply should resolve logical id");

        assert_eq!(result.files_copied, 2);
        assert!(workspace_folder.join("hello.txt").exists());
        assert!(workspace_folder.join("nested").join("world.txt").exists());
    }
}
