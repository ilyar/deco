use std::collections::BTreeMap;
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

use crate::support::manifest::{ParityFixture, ParityManifest};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParityHarnessConfig {
    pub deco_binary: PathBuf,
    pub upstream_binary: Option<PathBuf>,
    pub upstream_prefix_args: Vec<String>,
    pub update_snapshots: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunnerSpec {
    pub binary: PathBuf,
    pub args: Vec<String>,
    pub env: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapturedRun {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParityDiff {
    pub fixture_id: String,
    pub expected: String,
    pub actual: String,
}

#[derive(Debug, Clone)]
pub struct ParityHarness {
    pub config: ParityHarnessConfig,
}

impl ParityHarness {
    pub fn new(config: ParityHarnessConfig) -> Self {
        Self { config }
    }

    pub fn load_manifest(&self, path: impl AsRef<Path>) -> Result<ParityManifest, String> {
        ParityManifest::from_path(path)
    }

    pub fn filter_fixtures<'a>(&self, fixtures: &'a [ParityFixture]) -> Vec<&'a ParityFixture> {
        let filter = env::var("DECO_PARITY_FILTER").ok();
        match filter {
            Some(filter) => {
                fixtures.iter().filter(|fixture| fixture.id.contains(&filter)).collect()
            }
            None => fixtures.iter().collect(),
        }
    }

    pub fn build_runner(&self, binary: PathBuf, args: Vec<String>) -> RunnerSpec {
        RunnerSpec { binary, args, env: BTreeMap::new() }
    }

    pub fn build_runner_for_fixture(&self, binary: PathBuf, fixture: &ParityFixture) -> RunnerSpec {
        self.build_runner_for_fixture_with_prefix(binary, Vec::new(), fixture)
    }

    pub fn build_runner_for_fixture_with_prefix(
        &self,
        binary: PathBuf,
        prefix_args: Vec<String>,
        fixture: &ParityFixture,
    ) -> RunnerSpec {
        let mut args = prefix_args;
        args.extend(fixture.command.clone());
        absolutize_fixture_args(&mut args);
        let should_inject_workspace = args
            .iter()
            .find(|arg| !arg.starts_with('-') && !arg.ends_with(".js"))
            .is_some_and(|command| {
                matches!(
                    command.as_str(),
                    "read-configuration"
                        | "build"
                        | "up"
                        | "exec"
                        | "run-user-commands"
                        | "set-up"
                        | "outdated"
                        | "upgrade"
                )
            });
        if should_inject_workspace && !args.iter().any(|arg| arg == "--workspace-folder") {
            args.push("--workspace-folder".to_string());
            args.push(absolutize_fixture_path(&fixture.workspace).display().to_string());
        }
        self.build_runner(binary, args)
    }

    pub fn run_fixture(
        &self,
        fixture: &ParityFixture,
        binary: PathBuf,
    ) -> Result<CapturedRun, String> {
        let runner = self.build_runner_for_fixture(binary, fixture);
        self.capture_run(&runner)
    }

    pub fn run_upstream_fixture(&self, fixture: &ParityFixture) -> Result<CapturedRun, String> {
        let binary = self
            .config
            .upstream_binary
            .clone()
            .ok_or_else(|| "upstream binary is not configured".to_string())?;
        let mut runner = self.build_runner_for_fixture_with_prefix(
            binary,
            self.config.upstream_prefix_args.clone(),
            fixture,
        );
        self.inject_upstream_runtime_shims(&mut runner);
        self.capture_run(&runner)
    }

    fn inject_upstream_runtime_shims(&self, runner: &mut RunnerSpec) {
        let has_docker_path = runner.args.iter().any(|arg| arg == "--docker-path");
        let supports_docker_path = runner
            .args
            .iter()
            .find(|arg| !arg.starts_with('-') && !arg.ends_with(".js"))
            .is_some_and(|command| {
                matches!(
                    command.as_str(),
                    "read-configuration"
                        | "build"
                        | "up"
                        | "exec"
                        | "run-user-commands"
                        | "set-up"
                        | "upgrade"
                )
            });
        if supports_docker_path
            && !has_docker_path
            && let Some(fake_docker) = self.resolve_fake_docker_path()
        {
            runner.args.push("--docker-path".to_string());
            runner.args.push(fake_docker.display().to_string());
        }
    }

    fn resolve_fake_docker_path(&self) -> Option<PathBuf> {
        if let Some(path) = env::var_os("DECO_PARITY_FAKE_DOCKER").map(PathBuf::from) {
            return Some(path);
        }

        let candidate = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("parity")
            .join("bin")
            .join("fake-docker");
        candidate.exists().then_some(candidate)
    }

    pub fn capture_run(&self, spec: &RunnerSpec) -> Result<CapturedRun, String> {
        let output = Command::new(&spec.binary)
            .args(&spec.args)
            .envs(&spec.env)
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .output()
            .map_err(|error| format!("failed to execute `{}`: {error}", spec.binary.display()))?;

        Ok(CapturedRun {
            exit_code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        })
    }

    pub fn normalize_run(&self, run: CapturedRun) -> CapturedRun {
        run
    }

    pub fn compare_fixture(
        &self,
        fixture: &ParityFixture,
        deco_run: CapturedRun,
        upstream_run: Option<CapturedRun>,
    ) -> Result<(), ParityDiff> {
        if deco_run.exit_code != fixture.expected.exit_code {
            return Err(ParityDiff {
                fixture_id: fixture.id.clone(),
                expected: format!("exit code {}", fixture.expected.exit_code),
                actual: format!("exit code {}", deco_run.exit_code),
            });
        }

        for needle in &fixture.expected.stdout_contains {
            if !deco_run.stdout.contains(needle) {
                return Err(ParityDiff {
                    fixture_id: fixture.id.clone(),
                    expected: format!("stdout to contain `{needle}`"),
                    actual: deco_run.stdout,
                });
            }
        }

        for needle in &fixture.expected.stderr_contains {
            if !deco_run.stderr.contains(needle) {
                return Err(ParityDiff {
                    fixture_id: fixture.id.clone(),
                    expected: format!("stderr to contain `{needle}`"),
                    actual: deco_run.stderr,
                });
            }
        }

        if let Some(upstream_run) = upstream_run {
            let upstream_run = self.normalize_run(upstream_run);
            let deco_run = self.normalize_run(deco_run);

            if !fixture.expected.allow_upstream_exit_code_difference
                && deco_run.exit_code != upstream_run.exit_code
            {
                return Err(ParityDiff {
                    fixture_id: fixture.id.clone(),
                    expected: format!("upstream exit code {}", upstream_run.exit_code),
                    actual: format!("deco exit code {}", deco_run.exit_code),
                });
            }

            if let (Ok(deco_json), Ok(upstream_json)) = (
                serde_json::from_str::<Value>(&deco_run.stdout),
                serde_json::from_str::<Value>(&upstream_run.stdout),
            ) {
                for key in ["command", "outcome"] {
                    let deco_value = deco_json.get(key);
                    let upstream_value = upstream_json.get(key);
                    if let (Some(deco_value), Some(upstream_value)) = (deco_value, upstream_value)
                        && deco_value != upstream_value
                    {
                        return Err(ParityDiff {
                            fixture_id: fixture.id.clone(),
                            expected: format!(
                                "JSON field `{key}` to match upstream `{upstream_value}`"
                            ),
                            actual: format!("deco `{key}` was `{deco_value}`"),
                        });
                    }
                }
            }

            for needle in &fixture.expected.stdout_contains {
                if upstream_run.stdout.contains(needle) && !deco_run.stdout.contains(needle) {
                    return Err(ParityDiff {
                        fixture_id: fixture.id.clone(),
                        expected: format!("deco stdout to preserve upstream marker `{needle}`"),
                        actual: deco_run.stdout,
                    });
                }
            }
        }

        Ok(())
    }
}

fn absolutize_fixture_args(args: &mut [String]) {
    let path_flags = [
        "--workspace-folder",
        "--config",
        "--manifest-dir",
        "--manifest-path",
        "--target-dir",
        "--lockfile",
        "--project-folder",
    ];

    let mut index = 0;
    while index + 1 < args.len() {
        if path_flags.contains(&args[index].as_str()) {
            let path = PathBuf::from(&args[index + 1]);
            args[index + 1] = absolutize_fixture_path(&path).display().to_string();
            index += 2;
            continue;
        }
        index += 1;
    }
}

fn absolutize_fixture_path(path: &Path) -> PathBuf {
    if path.is_absolute() {
        return path.to_path_buf();
    }

    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(path)
}
