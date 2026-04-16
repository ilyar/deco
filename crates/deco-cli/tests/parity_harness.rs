mod support;

use std::env;
use std::path::PathBuf;
use std::process::Command;

use support::harness::{ParityHarness, ParityHarnessConfig};

#[test]
fn parity_manifest_is_loadable() {
    let harness = ParityHarness::new(ParityHarnessConfig {
        deco_binary: PathBuf::from("target/debug/deco"),
        upstream_binary: None,
        upstream_prefix_args: Vec::new(),
        update_snapshots: false,
    });

    let manifest = harness
        .load_manifest("tests/fixtures/parity/manifest.example.json")
        .expect("fixture manifest should parse");

    assert!(!manifest.fixtures.is_empty(), "fixture manifest should declare scenarios");
}

#[test]
fn parity_fixture_runs_against_local_deco() {
    let deco_binary = resolve_local_deco_binary();
    let harness = ParityHarness::new(ParityHarnessConfig {
        deco_binary,
        upstream_binary: None,
        upstream_prefix_args: Vec::new(),
        update_snapshots: false,
    });

    let manifest = harness
        .load_manifest("tests/fixtures/parity/manifest.example.json")
        .expect("fixture manifest should parse");
    let fixtures = harness.filter_fixtures(&manifest.fixtures);
    assert!(!fixtures.is_empty(), "at least one parity fixture should be selected");

    for fixture in fixtures {
        let run = harness
            .run_fixture(fixture, harness.config.deco_binary.clone())
            .expect("fixture should run");

        harness.compare_fixture(fixture, run, None).expect("fixture should satisfy expectations");
    }
}

#[test]
#[ignore = "configure upstream binary or node entrypoint to compare against upstream"]
fn parity_fixture_can_compare_with_upstream_when_configured() {
    let (upstream_binary, upstream_prefix_args) =
        resolve_upstream_runner().expect("upstream runner must be configured");
    let deco_binary = resolve_local_deco_binary();
    let harness = ParityHarness::new(ParityHarnessConfig {
        deco_binary,
        upstream_binary: Some(upstream_binary),
        upstream_prefix_args,
        update_snapshots: env::var_os("DECO_PARITY_UPDATE_SNAPSHOTS").is_some(),
    });

    let manifest = harness
        .load_manifest("tests/fixtures/parity/manifest.example.json")
        .expect("fixture manifest should parse");

    for fixture in harness
        .filter_fixtures(&manifest.fixtures)
        .into_iter()
        .filter(|fixture| fixture.compare_with_upstream)
    {
        let deco_run = harness
            .run_fixture(fixture, harness.config.deco_binary.clone())
            .expect("deco fixture should run");
        let upstream_run =
            harness.run_upstream_fixture(fixture).expect("upstream fixture should run");
        harness
            .compare_fixture(fixture, deco_run, Some(upstream_run))
            .expect("fixture should match upstream");
    }
}

fn resolve_upstream_runner() -> Option<(PathBuf, Vec<String>)> {
    if let Some(binary) = env::var_os("DECO_PARITY_UPSTREAM_BIN").map(PathBuf::from) {
        return Some((binary, Vec::new()));
    }

    if let Some(entrypoint) = env::var_os("DECO_PARITY_UPSTREAM_NODE_ENTRYPOINT").map(PathBuf::from)
    {
        let node_binary = env::var_os("DECO_PARITY_UPSTREAM_NODE_BIN")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("node"));
        return Some((node_binary, vec![entrypoint.display().to_string()]));
    }

    let local_entrypoint =
        PathBuf::from("/home/ilyar/startup/deco/knowledge/devcontainer-cli/devcontainer.js");
    let local_dist = PathBuf::from(
        "/home/ilyar/startup/deco/knowledge/devcontainer-cli/dist/spec-node/devContainersSpecCLI.js",
    );
    if local_entrypoint.exists() && local_dist.exists() {
        return Some((PathBuf::from("node"), vec![local_entrypoint.display().to_string()]));
    }

    None
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

    let status = Command::new("cargo")
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
