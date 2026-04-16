# Compatibility Harness

This document defines the intended fixture-driven parity workflow for `deco`.

## Purpose

The harness compares `deco` against representative upstream `devcontainer-cli` behaviors without coupling the product code to the comparison machinery.

## Layout

- `crates/deco-cli/tests/support/manifest.rs` owns the fixture schema.
- `crates/deco-cli/tests/support/harness.rs` owns runner configuration and comparison helpers.
- `crates/deco-cli/tests/parity_harness.rs` owns the ignored parity entrypoint.
- `crates/deco-cli/tests/fixtures/parity/` owns manifests and workspace fixtures.

## Current Integration

The harness now supports:

1. loading a manifest that describes fixture workspaces and expected outcomes,
2. executing fixture commands against the local `deco` test binary,
3. optionally executing the same fixture against an upstream binary when `DECO_PARITY_UPSTREAM_BIN` is set,
4. checking expected exit code and required stdout/stderr substrings,
5. comparing exit-code parity with upstream in opt-in mode,
6. comparing basic JSON envelope fields such as `command` and `outcome` when both binaries emit JSON stdout.

Upstream-backed parity is intentionally narrower than local parity. In the current phase the
required upstream-backed fixture subset is:

- `read-configuration*`
- `features-resolve-dependencies-config-local`
- `outdated-feature-lockfile`
- `upgrade-feature-lockfile-dry-run`

Local-only fixtures remain valuable, but they do not currently map to a truthful upstream contract.
That includes the local template flows and the legacy `--lockfile`-only fixtures.

It does not yet normalize full JSON payloads field-by-field and does not regenerate snapshots.

## Environment Contract

- `DECO_PARITY_UPSTREAM_BIN` points at the upstream binary.
- `DECO_PARITY_UPSTREAM_NODE_ENTRYPOINT` points at an upstream `devcontainer.js` entrypoint when you want to run upstream through Node.
- `DECO_PARITY_UPSTREAM_NODE_BIN` optionally overrides the Node executable for the entrypoint mode. Default: `node`.
- `DECO_PARITY_UPDATE_SNAPSHOTS=1` enables regeneration of expected snapshots.
- `DECO_PARITY_FILTER=<substring>` runs only fixtures whose `id` contains that value.

If neither env var is set, the ignored upstream test also knows how to auto-detect a locally built
`knowledge/devcontainer-cli/devcontainer.js` when its compiled `dist/spec-node/devContainersSpecCLI.js`
is present.

For non-Docker parity fixtures the harness also auto-detects
`crates/deco-cli/tests/fixtures/parity/bin/fake-docker` and passes it through `--docker-path`
to the upstream CLI. You can override that with `DECO_PARITY_FAKE_DOCKER`.

## Scope

The always-on test runs the sample fixture against the local `deco` binary.
The upstream comparison test remains ignored and becomes active only when an upstream binary path is provided.

## Recommended Commands

Run local parity only:

```bash
cargo test -q -p deco-cli --test parity_harness
```

Run the current upstream-backed subset:

```bash
DECO_PARITY_UPSTREAM_NODE_ENTRYPOINT=/tmp/devcontainer-cli-build.qHh47l/devcontainer.js \
DECO_PARITY_FILTER=read-configuration \
cargo test -q -p deco-cli --test parity_harness parity_fixture_can_compare_with_upstream_when_configured -- --ignored

DECO_PARITY_UPSTREAM_NODE_ENTRYPOINT=/tmp/devcontainer-cli-build.qHh47l/devcontainer.js \
DECO_PARITY_FILTER=features-resolve-dependencies-config-local \
cargo test -q -p deco-cli --test parity_harness parity_fixture_can_compare_with_upstream_when_configured -- --ignored

DECO_PARITY_UPSTREAM_NODE_ENTRYPOINT=/tmp/devcontainer-cli-build.qHh47l/devcontainer.js \
DECO_PARITY_FILTER=outdated-feature-lockfile \
cargo test -q -p deco-cli --test parity_harness parity_fixture_can_compare_with_upstream_when_configured -- --ignored

DECO_PARITY_UPSTREAM_NODE_ENTRYPOINT=/tmp/devcontainer-cli-build.qHh47l/devcontainer.js \
DECO_PARITY_FILTER=upgrade-feature-lockfile-dry-run \
cargo test -q -p deco-cli --test parity_harness parity_fixture_can_compare_with_upstream_when_configured -- --ignored
```
