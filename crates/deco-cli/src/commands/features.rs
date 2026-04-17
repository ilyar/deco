use std::env;
use std::path::{Path, PathBuf};

use deco_core_model::{DecoError, ErrorCategory};
use deco_features::{
    FeatureDependencyResolutionResult, FeatureTestResult, FeaturesResult, FeaturesSource,
    discover_feature_manifests, features_from_read_configuration, resolve_feature_dependencies,
    test_feature_manifests,
};
use serde::Serialize;

use crate::cli::{FeaturesArgs, FeaturesCommand, FeaturesInspectArgs, FeaturesTestArgs};

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "mode", rename_all = "kebab-case")]
pub enum FeaturesCommandResult {
    Inspect(FeaturesResult),
    ResolveDependencies(FeatureDependencyResolutionResult),
    Test(FeatureTestResult),
}

pub fn run(args: FeaturesArgs) -> Result<FeaturesCommandResult, DecoError> {
    match args.command {
        Some(FeaturesCommand::ResolveDependencies(args)) => {
            run_resolve_dependencies(args).map(FeaturesCommandResult::ResolveDependencies)
        }
        Some(FeaturesCommand::Test(args)) => run_test(args).map(FeaturesCommandResult::Test),
        None => run_inspect(args.inspect).map(FeaturesCommandResult::Inspect),
    }
}

pub fn run_inspect(args: FeaturesInspectArgs) -> Result<FeaturesResult, DecoError> {
    let current_dir = base_dir()?;

    if let Some(manifest_dir) = args.manifest_dir {
        let manifest_dir = absolutize(&current_dir, manifest_dir);
        let manifests = discover_feature_manifests(&manifest_dir)?;
        return Ok(FeaturesResult {
            source: FeaturesSource::ManifestDirectory,
            manifest_dir: Some(manifest_dir.display().to_string()),
            workspace_folder: None,
            config_file: None,
            manifests,
            references: Vec::new(),
        });
    }

    let resolved = deco_config::resolve_read_configuration(
        &current_dir,
        args.workspace_folder.as_deref(),
        args.config.as_deref(),
        false,
    )?;
    features_from_read_configuration(&resolved)
}

pub fn run_resolve_dependencies(
    args: FeaturesInspectArgs,
) -> Result<FeatureDependencyResolutionResult, DecoError> {
    let current_dir = base_dir()?;

    if let Some(manifest_dir) = args.manifest_dir {
        let manifest_dir = absolutize(&current_dir, manifest_dir);
        return resolve_feature_dependencies(Some(&manifest_dir), None);
    }

    let resolved = deco_config::resolve_read_configuration(
        &current_dir,
        args.workspace_folder.as_deref(),
        args.config.as_deref(),
        false,
    )?;
    resolve_feature_dependencies(None, Some(&resolved))
}

pub fn run_test(args: FeaturesTestArgs) -> Result<FeatureTestResult, DecoError> {
    let current_dir = base_dir()?;
    let manifest_dir = resolve_test_manifest_dir(&current_dir, &args)?;
    test_feature_manifests(manifest_dir)
}

fn base_dir() -> Result<PathBuf, DecoError> {
    match env::current_dir() {
        Ok(path) => Ok(path),
        Err(_) => Ok(PathBuf::from(".")),
    }
}

fn absolutize(base: &Path, value: PathBuf) -> PathBuf {
    if value.is_absolute() { value } else { base.join(value) }
}

fn resolve_test_manifest_dir(base: &Path, args: &FeaturesTestArgs) -> Result<PathBuf, DecoError> {
    if let Some(manifest_dir) = &args.manifest_dir {
        return Ok(absolutize(base, manifest_dir.clone()));
    }

    if let Some(project_folder) = args.project_folder.clone().or_else(|| args.target.clone()) {
        let project_folder = absolutize(base, project_folder);
        let src_dir = project_folder.join("src");
        if src_dir.is_dir() {
            return Ok(src_dir);
        }
        return Ok(project_folder);
    }

    Err(DecoError::new(
        ErrorCategory::User,
        "features test requires --manifest-dir, --project-folder, or a target path",
    ))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use tempfile::tempdir;

    #[test]
    fn features_from_manifest_directory_discovers_json_files() {
        let temp = tempdir().expect("tempdir should be created");
        fs::write(
            temp.path().join("feature-a.json"),
            r#"{
              "id": "feature-a",
              "version": "1.0.0",
              "name": "Feature A"
            }"#,
        )
        .expect("manifest should be written");

        let result = run(FeaturesArgs {
            command: None,
            inspect: FeaturesInspectArgs {
                manifest_dir: Some(temp.path().to_path_buf()),
                workspace_folder: None,
                config: None,
            },
        })
        .expect("features command should succeed");

        match result {
            FeaturesCommandResult::Inspect(result) => {
                assert_eq!(result.source, FeaturesSource::ManifestDirectory);
                assert_eq!(result.manifests.len(), 1);
                assert_eq!(result.manifests[0].id.as_deref(), Some("feature-a"));
            }
            other => panic!("unexpected result: {other:?}"),
        }
    }

    #[test]
    fn features_from_config_reads_feature_references() {
        let temp = tempdir().expect("tempdir should be created");
        fs::create_dir_all(temp.path().join(".devcontainer")).expect("config dir should exist");
        fs::write(
            temp.path().join(".devcontainer").join("devcontainer.json"),
            r#"{
              "image": "alpine:3.20",
              "features": {
                "ghcr.io/devcontainers/features/common-utils:2": {
                  "installZsh": true
                }
              }
            }"#,
        )
        .expect("config should be written");

        let _cwd_guard = crate::test_support::cwd_lock();
        let previous_dir = env::current_dir().expect("cwd should be available");
        env::set_current_dir(temp.path()).expect("cwd should be changed");

        let result = run(FeaturesArgs {
            command: None,
            inspect: FeaturesInspectArgs {
                manifest_dir: None,
                workspace_folder: Some(temp.path().to_path_buf()),
                config: None,
            },
        })
        .expect("features command should succeed");

        env::set_current_dir(previous_dir).expect("cwd should be restored");

        match result {
            FeaturesCommandResult::Inspect(result) => {
                assert_eq!(result.source, FeaturesSource::DevcontainerConfig);
                assert_eq!(result.references.len(), 1);
                assert_eq!(
                    result.references[0].reference,
                    "ghcr.io/devcontainers/features/common-utils:2"
                );
            }
            other => panic!("unexpected result: {other:?}"),
        }
    }

    #[test]
    fn resolve_dependencies_reads_local_manifest_links() {
        let temp = tempdir().expect("tempdir should be created");
        fs::write(
            temp.path().join("feature-a.json"),
            r#"{
              "id": "feature-a",
              "dependsOn": ["feature-b"],
              "installsAfter": ["feature-c"]
            }"#,
        )
        .expect("manifest should be written");
        fs::write(temp.path().join("feature-b.json"), r#"{ "id": "feature-b" }"#)
            .expect("manifest should be written");

        let result = run_resolve_dependencies(FeaturesInspectArgs {
            manifest_dir: Some(temp.path().to_path_buf()),
            workspace_folder: None,
            config: None,
        })
        .expect("dependency resolution should succeed");

        assert_eq!(result.nodes.len(), 2);
        assert_eq!(result.nodes[0].id, "feature-a");
        assert_eq!(result.nodes[0].depends_on, vec!["feature-b".to_string()]);
        assert_eq!(result.nodes[0].installs_after, vec!["feature-c".to_string()]);
    }

    #[test]
    fn test_reports_invalid_feature_manifest() {
        let temp = tempdir().expect("tempdir should be created");
        fs::write(temp.path().join("broken.json"), r#"{ "name": "Broken" }"#)
            .expect("manifest should be written");

        let result = run_test(FeaturesTestArgs {
            manifest_dir: Some(temp.path().to_path_buf()),
            project_folder: None,
            target: None,
        })
        .expect("feature test should complete");

        assert_eq!(result.total, 1);
        assert_eq!(result.failed, 1);
    }

    #[test]
    fn test_can_use_project_folder_src_layout() {
        let temp = tempdir().expect("tempdir should be created");
        let src_dir = temp.path().join("src").join("feature-a");
        fs::create_dir_all(&src_dir).expect("src dir should exist");
        fs::write(
            src_dir.join("devcontainer-feature.json"),
            r#"{
              "id": "feature-a"
            }"#,
        )
        .expect("manifest should be written");

        let result = run_test(FeaturesTestArgs {
            manifest_dir: None,
            project_folder: Some(temp.path().to_path_buf()),
            target: None,
        })
        .expect("feature test should complete");

        assert_eq!(result.total, 1);
        assert_eq!(result.failed, 0);
    }
}
