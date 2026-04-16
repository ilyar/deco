use assert_cmd::Command;
use std::env;
use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::Command as StdCommand;
use tempfile::tempdir;

fn deco_command() -> Command {
    if let Ok(command) = Command::cargo_bin("deco") {
        return command;
    }

    Command::new(resolve_local_deco_binary())
}

fn resolve_local_deco_binary() -> PathBuf {
    if let Some(binary) = env::var_os("CARGO_BIN_EXE_deco").map(PathBuf::from) {
        return binary;
    }

    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("deco-cli should live under <workspace>/crates/deco-cli")
        .to_path_buf();
    let candidate = workspace_root.join("target").join("debug").join("deco");
    if candidate.exists() {
        return candidate;
    }

    let status = StdCommand::new("cargo")
        .arg("build")
        .arg("-q")
        .arg("-p")
        .arg("deco")
        .current_dir(&workspace_root)
        .status()
        .expect("building root deco binary should succeed");
    assert!(status.success(), "building root deco binary should succeed");
    candidate
}

#[test]
fn read_configuration_returns_success_envelope() {
    let temp = tempdir().expect("tempdir should be created");
    fs::create_dir_all(temp.path().join(".devcontainer")).expect("config directory should exist");
    fs::write(
        temp.path().join(".devcontainer").join("devcontainer.json"),
        r#"
        {
          // comment to prove jsonc parsing works
          "name": "sample",
          "image": "mcr.microsoft.com/devcontainers/rust:1"
        }
        "#,
    )
    .expect("config file should be written");

    let mut command = deco_command();
    command.arg("read-configuration").arg("--workspace-folder").arg(temp.path());

    command
        .assert()
        .success()
        .stdout(predicates::str::contains("\"outcome\": \"success\""))
        .stdout(predicates::str::contains("\"command\": \"read-configuration\""))
        .stdout(predicates::str::contains("\"kind\": \"image\""))
        .stdout(predicates::str::contains("\"normalized\""))
        .stdout(predicates::str::contains("\"image\": \"mcr.microsoft.com/devcontainers/rust:1\""))
        .stdout(predicates::str::contains("\"configuration\""));
}

#[test]
fn build_returns_success_for_image_based_config() {
    let temp = tempdir().expect("tempdir should be created");
    fs::create_dir_all(temp.path().join(".devcontainer")).expect("config directory should exist");
    fs::write(
        temp.path().join(".devcontainer").join("devcontainer.json"),
        r#"{ "image": "alpine:3.20" }"#,
    )
    .expect("config file should be written");

    let mut command = deco_command();
    command.arg("build").arg("--workspace-folder").arg(temp.path());

    command
        .assert()
        .success()
        .stdout(predicates::str::contains("\"command\": \"build\""))
        .stdout(predicates::str::contains("\"execution_status\": \"skipped-existing-image\""))
        .stdout(predicates::str::contains("\"image\": \"alpine:3.20\""));
}

#[test]
fn read_configuration_detects_compose_configs() {
    let temp = tempdir().expect("tempdir should be created");
    fs::create_dir_all(temp.path().join(".devcontainer")).expect("config directory should exist");
    fs::write(
        temp.path().join(".devcontainer").join("devcontainer.json"),
        r#"{ "dockerComposeFile": "compose.yml", "service": "app" }"#,
    )
    .expect("config file should be written");

    let mut command = deco_command();
    command.arg("read-configuration").arg("--workspace-folder").arg(temp.path());

    command
        .assert()
        .success()
        .stdout(predicates::str::contains("\"kind\": \"compose\""))
        .stdout(predicates::str::contains("\"service\": \"app\""));
}

#[test]
fn read_configuration_returns_structured_config_error_when_missing() {
    let temp = tempdir().expect("tempdir should be created");

    let mut command = deco_command();
    command.arg("read-configuration").arg("--workspace-folder").arg(temp.path());

    command
        .assert()
        .code(3)
        .stdout(predicates::str::contains("\"category\": \"config\""))
        .stderr(predicates::str::contains("dev container config not found"));
}

#[test]
fn features_from_manifest_directory_returns_summary() {
    let temp = tempdir().expect("tempdir should be created");
    fs::write(
        temp.path().join("feature-a.json"),
        r#"{
          "id": "feature-a",
          "version": "1.0.0",
          "name": "Feature A",
          "options": { "username": {} }
        }"#,
    )
    .expect("manifest should be written");

    let mut command = deco_command();
    command.arg("features").arg("--manifest-dir").arg(temp.path());

    command
        .assert()
        .success()
        .stdout(predicates::str::contains("\"command\": \"features\""))
        .stdout(predicates::str::contains("\"source\": \"manifest-directory\""))
        .stdout(predicates::str::contains("\"id\": \"feature-a\""))
        .stdout(predicates::str::contains("\"option_names\": ["))
        .stdout(predicates::str::contains("\"username\""));
}

#[test]
fn features_from_config_returns_references_summary() {
    let temp = tempdir().expect("tempdir should be created");
    fs::create_dir_all(temp.path().join(".devcontainer")).expect("config directory should exist");
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
    .expect("config file should be written");

    let mut command = deco_command();
    command.arg("features").arg("--workspace-folder").arg(temp.path());

    command
        .assert()
        .success()
        .stdout(predicates::str::contains("\"command\": \"features\""))
        .stdout(predicates::str::contains("\"source\": \"devcontainer-config\""))
        .stdout(predicates::str::contains("\"references\""))
        .stdout(predicates::str::contains("\"ghcr.io/devcontainers/features/common-utils:2\""))
        .stdout(predicates::str::contains("\"installZsh\""));
}

#[test]
fn features_resolve_dependencies_returns_graph_summary() {
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

    let mut command = deco_command();
    command.arg("features").arg("resolve-dependencies").arg("--manifest-dir").arg(temp.path());

    command
        .assert()
        .success()
        .stdout(predicates::str::contains("\"mode\": \"resolve-dependencies\""))
        .stdout(predicates::str::contains("\"roots\": ["))
        .stdout(predicates::str::contains("\"feature-b\""))
        .stdout(predicates::str::contains("\"depends_on\": ["))
        .stdout(predicates::str::contains("\"installs_after\": ["))
        .stdout(predicates::str::contains("\"feature-c\""));
}

#[test]
fn features_resolve_dependencies_reads_config_metadata() {
    let temp = tempdir().expect("tempdir should be created");
    fs::create_dir_all(temp.path().join(".devcontainer")).expect("config directory should exist");
    fs::write(
        temp.path().join(".devcontainer").join("devcontainer.json"),
        r#"{
          "image": "alpine:3.20",
          "features": {
            "feature-a": {
              "dependsOn": ["feature-b"],
              "installsAfter": ["feature-c"],
              "installZsh": true
            },
            "feature-b": {}
          }
        }"#,
    )
    .expect("config should be written");

    let mut command = deco_command();
    command.arg("features").arg("resolve-dependencies").arg("--workspace-folder").arg(temp.path());

    command
        .assert()
        .success()
        .stdout(predicates::str::contains("\"mode\": \"resolve-dependencies\""))
        .stdout(predicates::str::contains("\"source\": \"devcontainer-config\""))
        .stdout(predicates::str::contains("\"feature-a\""))
        .stdout(predicates::str::contains("\"feature-b\""))
        .stdout(predicates::str::contains("\"depends_on\": ["))
        .stdout(predicates::str::contains("\"installs_after\": ["))
        .stdout(predicates::str::contains("\"roots\": ["));
}

#[test]
fn features_test_reports_pass_fail_summary() {
    let temp = tempdir().expect("tempdir should be created");
    fs::write(
        temp.path().join("good.json"),
        r#"{
          "id": "good",
          "dependsOn": []
        }"#,
    )
    .expect("manifest should be written");
    fs::write(temp.path().join("broken.json"), r#"{ "name": "Broken" }"#)
        .expect("manifest should be written");

    let mut command = deco_command();
    command.arg("features").arg("test").arg("--manifest-dir").arg(temp.path());

    command
        .assert()
        .success()
        .stdout(predicates::str::contains("\"mode\": \"test\""))
        .stdout(predicates::str::contains("\"total\": 2"))
        .stdout(predicates::str::contains("\"failed\": 1"))
        .stdout(predicates::str::contains("\"passed\": 1"))
        .stdout(predicates::str::contains("broken.json"))
        .stdout(predicates::str::contains("missing `id`"));
}

#[test]
fn features_resolve_dependencies_returns_local_graph() {
    let temp = tempdir().expect("tempdir should be created");
    fs::write(
        temp.path().join("feature-a.json"),
        r#"{ "id": "feature-a", "dependsOn": ["feature-b"] }"#,
    )
    .expect("manifest should be written");
    fs::write(temp.path().join("feature-b.json"), r#"{ "id": "feature-b" }"#)
        .expect("manifest should be written");

    let mut command = deco_command();
    command.arg("features").arg("resolve-dependencies").arg("--manifest-dir").arg(temp.path());

    command
        .assert()
        .success()
        .stdout(predicates::str::contains("\"mode\": \"resolve-dependencies\""))
        .stdout(predicates::str::contains("\"roots\""))
        .stdout(predicates::str::contains("\"feature-a\""))
        .stdout(predicates::str::contains("\"feature-b\""));
}

#[test]
fn features_test_reports_failures_for_invalid_manifests() {
    let temp = tempdir().expect("tempdir should be created");
    fs::write(temp.path().join("broken.json"), r#"{ "name": "Broken" }"#)
        .expect("manifest should be written");

    let mut command = deco_command();
    command.arg("features").arg("test").arg("--manifest-dir").arg(temp.path());

    command
        .assert()
        .success()
        .stdout(predicates::str::contains("\"mode\": \"test\""))
        .stdout(predicates::str::contains("\"failed\": 1"))
        .stdout(predicates::str::contains("missing `id`"));
}

#[test]
fn features_test_accepts_project_folder_layout() {
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

    let mut command = deco_command();
    command.arg("features").arg("test").arg("--project-folder").arg(temp.path());

    command
        .assert()
        .success()
        .stdout(predicates::str::contains("\"mode\": \"test\""))
        .stdout(predicates::str::contains("\"total\": 1"))
        .stdout(predicates::str::contains("\"failed\": 0"));
}

#[test]
fn outdated_supports_upstream_feature_lockfiles_via_workspace_folder() {
    let temp = tempdir().expect("tempdir should be created");
    fs::create_dir_all(temp.path().join(".devcontainer")).expect("config dir should exist");
    fs::write(
        temp.path().join(".devcontainer").join("devcontainer.json"),
        r#"{
          "image": "alpine:3.20",
          "features": {
            "./features/feature-a": {}
          }
        }"#,
    )
    .expect("config should be written");
    fs::create_dir_all(temp.path().join(".devcontainer").join("features").join("feature-a"))
        .expect("feature dir should exist");
    fs::write(
        temp.path()
            .join(".devcontainer")
            .join("features")
            .join("feature-a")
            .join("devcontainer-feature.json"),
        r#"{ "id": "feature-a", "version": "1.2.3" }"#,
    )
    .expect("feature manifest should be written");
    fs::write(
        temp.path().join(".devcontainer").join("devcontainer-lock.json"),
        r#"{
          "features": {
            "feature-a": {
              "version": "1.2.3",
              "resolved": "./features/feature-a",
              "integrity": "unresolved"
            }
          }
        }"#,
    )
    .expect("lockfile should be written");

    let mut command = deco_command();
    command.arg("outdated").arg("--workspace-folder").arg(temp.path());

    command
        .assert()
        .success()
        .stdout(predicates::str::contains("\"command\": \"outdated\""))
        .stdout(predicates::str::contains("\"format\": \"devcontainer-feature\""))
        .stdout(predicates::str::contains("\"feature_count\": 1"))
        .stdout(predicates::str::contains("\"feature_lockfile\""))
        .stdout(predicates::str::contains("\"feature-a\""));
}

#[test]
fn upgrade_can_generate_feature_lockfile_from_workspace_config_in_dry_run_mode() {
    let temp = tempdir().expect("tempdir should be created");
    fs::create_dir_all(temp.path().join(".devcontainer").join("features").join("feature-a"))
        .expect("feature dir should exist");
    fs::write(
        temp.path().join(".devcontainer").join("devcontainer.json"),
        r#"{
          "image": "alpine:3.20",
          "features": {
            "./features/feature-a": {
              "dependsOn": ["feature-b"]
            }
          }
        }"#,
    )
    .expect("config should be written");
    fs::write(
        temp.path()
            .join(".devcontainer")
            .join("features")
            .join("feature-a")
            .join("devcontainer-feature.json"),
        r#"{ "id": "feature-a", "version": "2.0.0", "dependsOn": ["feature-b"] }"#,
    )
    .expect("feature manifest should be written");

    let mut command = deco_command();
    command.arg("upgrade").arg("--workspace-folder").arg(temp.path()).arg("--dry-run");

    command
        .assert()
        .success()
        .stdout(predicates::str::contains("\"command\": \"upgrade\""))
        .stdout(predicates::str::contains("\"format\": \"devcontainer-feature\""))
        .stdout(predicates::str::contains("\"written\": false"))
        .stdout(predicates::str::contains("\"feature_count\": 1"))
        .stdout(predicates::str::contains("\"version\": \"2.0.0\""))
        .stdout(predicates::str::contains("\"dependsOn\": ["));
}

#[test]
fn templates_metadata_returns_manifest_summary() {
    let temp = tempdir().expect("tempdir should be created");
    let template_dir = temp.path().join("template");
    fs::create_dir_all(&template_dir).expect("template dir should exist");
    fs::write(
        temp.path().join("template.json"),
        r#"{
          "id": "sample-template",
          "name": "Sample Template",
          "version": "1.0.0",
          "source_dir": "./template"
        }"#,
    )
    .expect("manifest should be written");

    let mut command = deco_command();
    command
        .arg("templates")
        .arg("metadata")
        .arg("--manifest-path")
        .arg(temp.path().join("template.json"));

    command
        .assert()
        .success()
        .stdout(predicates::str::contains("\"command\": \"templates\""))
        .stdout(predicates::str::contains("\"scan_mode\": \"file\""))
        .stdout(predicates::str::contains("\"sample-template\""))
        .stdout(predicates::str::contains("\"source_dir\""));
}

#[test]
fn templates_metadata_accepts_template_id_path() {
    let temp = tempdir().expect("tempdir should be created");
    fs::create_dir_all(temp.path().join("template")).expect("template dir should exist");
    fs::write(
        temp.path().join("template.json"),
        r#"{
          "id": "sample-template",
          "source_dir": "./template"
        }"#,
    )
    .expect("manifest should be written");

    let mut command = deco_command();
    command
        .arg("templates")
        .arg("metadata")
        .arg("--template-id")
        .arg(temp.path().join("template.json"));

    command
        .assert()
        .success()
        .stdout(predicates::str::contains("\"scan_mode\": \"file\""))
        .stdout(predicates::str::contains("\"sample-template\""));
}

#[test]
fn templates_metadata_scans_manifest_directory() {
    let temp = tempdir().expect("tempdir should be created");
    fs::create_dir_all(temp.path().join("nested")).expect("nested dir should exist");
    fs::write(
        temp.path().join("alpha.json"),
        r#"{
          "id": "alpha",
          "source_dir": "./alpha"
        }"#,
    )
    .expect("manifest should be written");
    fs::write(
        temp.path().join("nested").join("beta.json"),
        r#"{
          "id": "beta",
          "source_dir": "./beta"
        }"#,
    )
    .expect("manifest should be written");

    let mut command = deco_command();
    command.arg("templates").arg("metadata").arg("--manifest-path").arg(temp.path());

    command
        .assert()
        .success()
        .stdout(predicates::str::contains("\"scan_mode\": \"directory\""))
        .stdout(predicates::str::contains("\"alpha\""))
        .stdout(predicates::str::contains("\"beta\""));
}

#[test]
fn templates_metadata_resolves_logical_template_id_from_collection() {
    let temp = tempdir().expect("tempdir should be created");
    fs::create_dir_all(temp.path().join("alpha")).expect("alpha dir should exist");
    fs::create_dir_all(temp.path().join("beta")).expect("beta dir should exist");
    fs::write(
        temp.path().join("alpha.json"),
        r#"{
          "id": "alpha",
          "source_dir": "./alpha"
        }"#,
    )
    .expect("manifest should be written");
    fs::write(
        temp.path().join("beta.json"),
        r#"{
          "id": "beta",
          "name": "Beta Template",
          "source_dir": "./beta"
        }"#,
    )
    .expect("manifest should be written");

    let mut command = deco_command();
    command
        .arg("templates")
        .arg("metadata")
        .arg("--manifest-path")
        .arg(temp.path())
        .arg("--template-id")
        .arg("beta");

    command
        .assert()
        .success()
        .stdout(predicates::str::contains("\"scan_mode\": \"file\""))
        .stdout(predicates::str::contains("\"beta\""))
        .stdout(predicates::str::contains("\"Beta Template\""))
        .stdout(predicates::str::contains("\"manifests\""));
}

#[test]
fn templates_metadata_reports_missing_logical_template_id() {
    let temp = tempdir().expect("tempdir should be created");
    fs::create_dir_all(temp.path().join("alpha")).expect("alpha dir should exist");
    fs::write(
        temp.path().join("alpha.json"),
        r#"{
          "id": "alpha",
          "source_dir": "./alpha"
        }"#,
    )
    .expect("manifest should be written");

    let mut command = deco_command();
    command
        .arg("templates")
        .arg("metadata")
        .arg("--manifest-path")
        .arg(temp.path())
        .arg("--template-id")
        .arg("missing");

    command
        .assert()
        .code(3)
        .stdout(predicates::str::contains("\"category\": \"config\""))
        .stderr(predicates::str::contains("template id `missing` was not found"));
}

#[test]
fn templates_metadata_reports_duplicate_logical_template_id() {
    let temp = tempdir().expect("tempdir should be created");
    fs::create_dir_all(temp.path().join("one")).expect("dir should exist");
    fs::create_dir_all(temp.path().join("two")).expect("dir should exist");
    fs::write(
        temp.path().join("one.json"),
        r#"{
          "id": "sample",
          "source_dir": "./one"
        }"#,
    )
    .expect("manifest should be written");
    fs::write(
        temp.path().join("two.json"),
        r#"{
          "id": "sample",
          "source_dir": "./two"
        }"#,
    )
    .expect("manifest should be written");

    let mut command = deco_command();
    command
        .arg("templates")
        .arg("metadata")
        .arg("--manifest-path")
        .arg(temp.path())
        .arg("--template-id")
        .arg("sample");

    command
        .assert()
        .code(3)
        .stdout(predicates::str::contains("\"category\": \"config\""))
        .stderr(predicates::str::contains("template id `sample` is duplicated"));
}

#[test]
fn templates_apply_copies_files() {
    let temp = tempdir().expect("tempdir should be created");
    let template_dir = temp.path().join("template");
    let target_dir = temp.path().join("target");
    fs::create_dir_all(template_dir.join("nested")).expect("template tree should exist");
    fs::write(template_dir.join("hello.txt"), "hello").expect("template file should be written");
    fs::write(template_dir.join("nested").join("world.txt"), "world")
        .expect("template file should be written");
    fs::write(
        temp.path().join("template.json"),
        r#"{
          "id": "sample-template",
          "source_dir": "./template"
        }"#,
    )
    .expect("manifest should be written");

    let mut command = deco_command();
    command
        .arg("templates")
        .arg("apply")
        .arg("--manifest-path")
        .arg(temp.path().join("template.json"))
        .arg("--target-dir")
        .arg(&target_dir);

    command
        .assert()
        .success()
        .stdout(predicates::str::contains("\"command\": \"templates\""))
        .stdout(predicates::str::contains("\"mode\": \"apply\""))
        .stdout(predicates::str::contains("\"files_copied\": 2"));

    assert_eq!(
        fs::read_to_string(target_dir.join("hello.txt")).expect("target file should exist"),
        "hello"
    );
    assert_eq!(
        fs::read_to_string(target_dir.join("nested").join("world.txt"))
            .expect("target file should exist"),
        "world"
    );
}

#[test]
fn templates_apply_resolves_logical_template_id_from_collection() {
    let temp = tempdir().expect("tempdir should be created");
    let alpha_dir = temp.path().join("alpha");
    let beta_source = temp.path().join("beta").join("src");
    let target_dir = temp.path().join("target");
    fs::create_dir_all(&alpha_dir).expect("alpha dir should exist");
    fs::create_dir_all(beta_source.join("nested")).expect("beta dir should exist");
    fs::write(beta_source.join("hello.txt"), "hello").expect("template file should be written");
    fs::write(beta_source.join("nested").join("world.txt"), "world")
        .expect("template file should be written");
    fs::write(
        temp.path().join("alpha.json"),
        r#"{
          "id": "alpha",
          "source_dir": "./alpha"
        }"#,
    )
    .expect("manifest should be written");
    fs::write(
        temp.path().join("beta.json"),
        r#"{
          "id": "beta",
          "source_dir": "./beta/src"
        }"#,
    )
    .expect("manifest should be written");

    let mut command = deco_command();
    command
        .arg("templates")
        .arg("apply")
        .arg("--manifest-path")
        .arg(temp.path())
        .arg("--template-id")
        .arg("beta")
        .arg("--target-dir")
        .arg(&target_dir);

    command
        .assert()
        .success()
        .stdout(predicates::str::contains("\"mode\": \"apply\""))
        .stdout(predicates::str::contains("\"files_copied\": 2"));

    assert_eq!(
        fs::read_to_string(target_dir.join("hello.txt")).expect("target file should exist"),
        "hello"
    );
    assert_eq!(
        fs::read_to_string(target_dir.join("nested").join("world.txt"))
            .expect("target file should exist"),
        "world"
    );
}

#[test]
fn templates_apply_accepts_template_id_directory_and_workspace_folder() {
    let temp = tempdir().expect("tempdir should be created");
    let template_dir = temp.path().join("template");
    let source_dir = template_dir.join("src");
    let workspace_dir = temp.path().join("workspace");
    fs::create_dir_all(source_dir.join("nested")).expect("template tree should exist");
    fs::write(source_dir.join("hello.txt"), "hello").expect("template file should be written");
    fs::write(source_dir.join("nested").join("world.txt"), "world")
        .expect("template file should be written");
    fs::write(
        template_dir.join("template.json"),
        r#"{
          "id": "sample-template",
          "source_dir": "./src"
        }"#,
    )
    .expect("manifest should be written");

    let mut command = deco_command();
    command
        .arg("templates")
        .arg("apply")
        .arg("--template-id")
        .arg(&template_dir)
        .arg("--workspace-folder")
        .arg(&workspace_dir);

    command
        .assert()
        .success()
        .stdout(predicates::str::contains("\"command\": \"templates\""))
        .stdout(predicates::str::contains("\"mode\": \"apply\""))
        .stdout(predicates::str::contains("\"files_copied\": 2"))
        .stdout(predicates::str::contains(workspace_dir.display().to_string()));

    assert_eq!(
        fs::read_to_string(workspace_dir.join("hello.txt")).expect("target file should exist"),
        "hello"
    );
    assert_eq!(
        fs::read_to_string(workspace_dir.join("nested").join("world.txt"))
            .expect("target file should exist"),
        "world"
    );
}

#[test]
fn outdated_reports_lockfile_summary() {
    let temp = tempdir().expect("tempdir should be created");
    let lockfile = temp.path().join("deco-lock.json");
    fs::write(
        &lockfile,
        r#"{
          "schema_version": 1,
          "source": {
            "workspace_folder": "/workspace",
            "config_file": "/workspace/.devcontainer/devcontainer.json"
          },
          "targets": [
            {
              "name": "base",
              "kind": "image",
              "reference": "alpine:3.20"
            }
          ]
        }"#,
    )
    .expect("lockfile should be written");

    let mut command = deco_command();
    command.arg("outdated").arg("--lockfile").arg(&lockfile);

    command
        .assert()
        .success()
        .stdout(predicates::str::contains("\"command\": \"outdated\""))
        .stdout(predicates::str::contains("\"source\""))
        .stdout(predicates::str::contains("\"targets\""))
        .stdout(predicates::str::contains("\"reference\": \"alpine:3.20\""))
        .stdout(predicates::str::contains("\"schema_version\": 1"))
        .stdout(predicates::str::contains("\"current_schema_version\": 1"))
        .stdout(predicates::str::contains("\"target_count\": 1"))
        .stdout(predicates::str::contains("\"upgrade_needed\": false"))
        .stdout(predicates::str::contains("\"valid\": true"));
}

#[test]
fn upgrade_writes_normalized_lockfile_unless_dry_run() {
    let temp = tempdir().expect("tempdir should be created");
    let lockfile = temp.path().join("deco-lock.json");
    fs::write(
        &lockfile,
        r#"{
          "schema_version": 0,
          "source": {
            "workspace_folder": "/workspace",
            "config_file": "/workspace/.devcontainer/devcontainer.json"
          },
          "targets": [
            {
              "name": "base",
              "kind": "image",
              "reference": "alpine:3.20"
            }
          ]
        }"#,
    )
    .expect("lockfile should be written");

    let mut command = deco_command();
    command.arg("upgrade").arg("--lockfile").arg(&lockfile);

    command
        .assert()
        .success()
        .stdout(predicates::str::contains("\"command\": \"upgrade\""))
        .stdout(predicates::str::contains("\"before_schema_version\": 0"))
        .stdout(predicates::str::contains("\"source\""))
        .stdout(predicates::str::contains("\"targets\""))
        .stdout(predicates::str::contains("\"schema_version\": 1"))
        .stdout(predicates::str::contains("\"current_schema_version\": 1"))
        .stdout(predicates::str::contains("\"target_count\": 1"))
        .stdout(predicates::str::contains("\"dry_run\": false"))
        .stdout(predicates::str::contains("\"written\": true"));

    let rewritten = fs::read_to_string(&lockfile).expect("lockfile should be readable");
    assert!(rewritten.contains("\"schema_version\": 1"));
}

#[test]
fn upgrade_dry_run_does_not_write_back() {
    let temp = tempdir().expect("tempdir should be created");
    let lockfile = temp.path().join("deco-lock.json");
    let original = r#"{
          "schema_version": 0,
          "source": {
            "workspace_folder": "/workspace",
            "config_file": "/workspace/.devcontainer/devcontainer.json"
          },
          "targets": []
        }"#;
    fs::write(&lockfile, original).expect("lockfile should be written");

    let mut command = deco_command();
    command.arg("upgrade").arg("--lockfile").arg(&lockfile).arg("--dry-run");

    command
        .assert()
        .success()
        .stdout(predicates::str::contains("\"dry_run\": true"))
        .stdout(predicates::str::contains("\"written\": false"));

    let after = fs::read_to_string(&lockfile).expect("lockfile should be readable");
    assert_eq!(after, original);
}

#[test]
fn outdated_can_resolve_lockfile_from_workspace_folder() {
    let temp = tempdir().expect("tempdir should be created");
    let config_dir = temp.path().join(".devcontainer");
    fs::create_dir_all(&config_dir).expect("config dir should exist");
    fs::write(config_dir.join("devcontainer.json"), r#"{ "image": "alpine:3.20" }"#)
        .expect("config should be written");
    fs::write(
        config_dir.join("devcontainer-lock.json"),
        r#"{
          "schema_version": 1,
          "source": {
            "workspace_folder": "/workspace",
            "config_file": ".devcontainer/devcontainer.json"
          },
          "targets": []
        }"#,
    )
    .expect("lockfile should be written");

    let mut command = deco_command();
    command.arg("outdated").arg("--workspace-folder").arg(temp.path());

    command
        .assert()
        .success()
        .stdout(predicates::str::contains("\"command\": \"outdated\""))
        .stdout(predicates::str::contains("\"lockfile\""))
        .stdout(predicates::str::contains("devcontainer-lock.json"));
}

#[test]
fn upgrade_can_resolve_lockfile_from_workspace_folder_in_dry_run_mode() {
    let temp = tempdir().expect("tempdir should be created");
    let config_dir = temp.path().join(".devcontainer");
    fs::create_dir_all(&config_dir).expect("config dir should exist");
    fs::write(config_dir.join("devcontainer.json"), r#"{ "image": "alpine:3.20" }"#)
        .expect("config should be written");
    let lockfile_path = config_dir.join("devcontainer-lock.json");
    let original = r#"{
          "schema_version": 0,
          "source": {
            "workspace_folder": "/workspace",
            "config_file": ".devcontainer/devcontainer.json"
          },
          "targets": []
        }"#;
    fs::write(&lockfile_path, original).expect("lockfile should be written");

    let mut command = deco_command();
    command.arg("upgrade").arg("--workspace-folder").arg(temp.path()).arg("--dry-run");

    command
        .assert()
        .success()
        .stdout(predicates::str::contains("\"command\": \"upgrade\""))
        .stdout(predicates::str::contains("\"dry_run\": true"))
        .stdout(predicates::str::contains("devcontainer-lock.json"));

    let after = fs::read_to_string(&lockfile_path).expect("lockfile should be readable");
    assert_eq!(after, original);
}

#[test]
fn outdated_exposes_feature_graph_from_resolved_config() {
    let temp = tempdir().expect("tempdir should be created");
    let config_dir = temp.path().join(".devcontainer");
    fs::create_dir_all(&config_dir).expect("config dir should exist");
    fs::create_dir_all(config_dir.join("features").join("feature-a"))
        .expect("feature dir should exist");
    fs::write(
        config_dir.join("features").join("feature-a").join("devcontainer-feature.json"),
        r#"{
          "id": "feature-a",
          "dependsOn": ["feature-b"]
        }"#,
    )
    .expect("feature manifest should be written");
    fs::write(
        config_dir.join("devcontainer.json"),
        r#"{
          "image": "alpine:3.20",
          "features": {
            "./features/feature-a": {
              "dependsOn": ["feature-b"]
            }
          }
        }"#,
    )
    .expect("config should be written");
    fs::write(
        config_dir.join("devcontainer-lock.json"),
        r#"{
          "schema_version": 1,
          "source": {
            "workspace_folder": "/workspace",
            "config_file": ".devcontainer/devcontainer.json"
          },
          "targets": [
            {
              "name": "feature-a",
              "kind": "feature",
              "reference": "./features/feature-a"
            }
          ]
        }"#,
    )
    .expect("lockfile should be written");

    let mut command = deco_command();
    command.arg("outdated").arg("--workspace-folder").arg(temp.path());

    command
        .assert()
        .success()
        .stdout(predicates::str::contains("\"config_kind\": \"image\""))
        .stdout(predicates::str::contains("\"config_feature_graph\""))
        .stdout(predicates::str::contains("\"nodes\""))
        .stdout(predicates::str::contains("\"feature-a\""))
        .stdout(predicates::str::contains("\"feature-b\""));
}

#[test]
fn set_up_runs_via_cli_and_returns_nested_summary() {
    let temp = tempdir().expect("tempdir should be created");
    let workspace = temp.path().join("workspace");
    let config_dir = workspace.join(".devcontainer");
    let fake_bin = temp.path().join("bin");
    fs::create_dir_all(&config_dir).expect("config directory should exist");
    fs::create_dir_all(&fake_bin).expect("fake bin directory should exist");
    fs::write(
        config_dir.join("devcontainer.json"),
        r#"{
          "image": "alpine:3.20",
          "initializeCommand": "echo init"
        }"#,
    )
    .expect("config file should be written");

    let docker_script = fake_bin.join("docker");
    let mut file = fs::File::create(&docker_script).expect("docker script should be created");
    writeln!(
        file,
        r#"#!/bin/sh
case "$1" in
  inspect)
    exit 1
    ;;
  create)
    printf '%s\n' container-setup
    ;;
  start)
    printf '%s\n' container-setup
    ;;
  exec)
    exit 0
    ;;
  *)
    exit 64
    ;;
esac
"#
    )
    .expect("docker script should be written");
    drop(file);
    let mut permissions =
        fs::metadata(&docker_script).expect("docker script metadata should exist").permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&docker_script, permissions).expect("docker script should be executable");

    let old_path = std::env::var_os("PATH").expect("PATH should exist");

    let mut command = deco_command();
    command.arg("set-up").arg("--workspace-folder").arg(&workspace);
    command.current_dir(&workspace);
    command.env("PATH", format!("{}:{}", fake_bin.display(), old_path.to_string_lossy()));

    command
        .assert()
        .success()
        .stdout(predicates::str::contains("\"command\": \"set-up\""))
        .stdout(predicates::str::contains("\"execution_status\": \"completed\""))
        .stdout(predicates::str::contains("\"container_id\": \"container-setup\""))
        .stdout(predicates::str::contains("\"planned_steps\": 1"));
}
