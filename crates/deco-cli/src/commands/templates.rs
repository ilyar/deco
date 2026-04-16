use std::env;
use std::path::{Path, PathBuf};

use deco_core_model::{DecoError, ErrorCategory};
use deco_templates::{
    TemplateApplyResult, TemplatesMetadataResult, apply_template, inspect_template_manifest_path,
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
    let manifest_path = resolve_template_manifest_path(&manifest_path)?;
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
    let raw_path = template_id.or(manifest_path).ok_or_else(|| {
        DecoError::new(
            ErrorCategory::Config,
            format!("{command_name} requires `--template-id` or `--manifest-path`"),
        )
    })?;
    Ok(absolutize(current_dir, raw_path))
}

fn resolve_template_manifest_path(path: &Path) -> Result<PathBuf, DecoError> {
    if path.is_file() {
        return Ok(path.to_path_buf());
    }

    if !path.is_dir() {
        return Err(DecoError::new(
            ErrorCategory::Config,
            format!("template path `{}` does not exist", path.display()),
        ));
    }

    let summary = inspect_template_manifest_path(path)?;
    match summary.manifests.as_slice() {
        [single] => Ok(PathBuf::from(&single.path)),
        [] => Err(DecoError::new(
            ErrorCategory::Config,
            format!("template path `{}` does not contain a manifest", path.display()),
        )),
        _ => Err(DecoError::new(
            ErrorCategory::Config,
            format!(
                "template path `{}` contains multiple manifests; pass a manifest file with `--manifest-path`",
                path.display()
            ),
        )),
    }
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
}
