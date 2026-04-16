use std::fs;

use deco_lockfile::{
    LockfileDocument, LockfileSource, LockfileTarget, LockfileTargetKind, parse_lockfile_json,
    parse_lockfile_path, serialize_lockfile_json, validate_lockfile_document,
};

#[test]
fn roundtrip_lockfile_document() {
    let document = LockfileDocument::new(LockfileSource::new(
        "/workspace/app",
        "/workspace/app/.devcontainer/devcontainer.json",
    ))
    .with_target(
        LockfileTarget::new(
            "base-image",
            LockfileTargetKind::Image,
            "mcr.microsoft.com/devcontainers/base:ubuntu",
        )
        .with_resolved_reference("mcr.microsoft.com/devcontainers/base@sha256:deadbeef")
        .with_digest("sha256:deadbeef"),
    );

    let json = serialize_lockfile_json(&document).expect("serialize");
    let parsed = parse_lockfile_json(&json).expect("parse");

    assert_eq!(parsed, document);
}

#[test]
fn rejects_duplicate_target_names() {
    let mut document = LockfileDocument::new(LockfileSource::new(
        "/workspace/app",
        "/workspace/app/.devcontainer/devcontainer.json",
    ));
    document.push_target(LockfileTarget::new("base", LockfileTargetKind::Image, "image-a"));
    document.push_target(LockfileTarget::new("base", LockfileTargetKind::Dockerfile, "Dockerfile"));

    let error = validate_lockfile_document(&document).expect_err("should reject duplicates");
    assert!(error.to_string().contains("duplicate target name"));
}

#[test]
fn loads_lockfile_from_path() {
    let tmpdir = tempfile::tempdir().expect("tempdir");
    let path = tmpdir.path().join("deco.lock.json");
    let content = r#"{
        "schema_version": 1,
        "source": {
            "workspace_folder": "/workspace/app",
            "config_file": "/workspace/app/.devcontainer/devcontainer.json"
        },
        "targets": [
            {
                "name": "image",
                "kind": "image",
                "reference": "mcr.microsoft.com/devcontainers/base:ubuntu"
            }
        ]
    }"#;
    fs::write(&path, content).expect("write lockfile");

    let document = parse_lockfile_path(&path).expect("parse path");
    assert_eq!(document.targets.len(), 1);
    assert_eq!(document.targets[0].kind, LockfileTargetKind::Image);
}

#[test]
fn rejects_unsupported_schema_version() {
    let json = r#"{
        "schema_version": 2,
        "source": {
            "workspace_folder": "/workspace/app",
            "config_file": "/workspace/app/.devcontainer/devcontainer.json"
        }
    }"#;

    let error = parse_lockfile_json(json).expect_err("schema version should be rejected");
    assert!(error.to_string().contains("unsupported lockfile schema version"));
}
