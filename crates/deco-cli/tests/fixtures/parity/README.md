# Compatibility Harness Fixtures

This directory is reserved for fixture-driven parity checks against upstream `devcontainer-cli`.

The harness is intentionally split from product logic:
- `tests/support/manifest.rs` defines the fixture schema.
- `tests/support/harness.rs` defines comparison and normalization hooks.
- `tests/parity_harness.rs` is an ignored integration test that will later invoke both binaries.

Integration contract:
- set `DECO_PARITY_UPSTREAM_BIN` to the upstream CLI binary when ready.
- set `DECO_PARITY_UPDATE_SNAPSHOTS=1` only when updating expected parity snapshots.
- keep fixture workspaces in this directory so scenarios stay self-contained.

Not every fixture in this directory is expected to compare with upstream.

Current upstream-backed subset:
- `read-configuration*`
- `features-resolve-dependencies-config-local`
- `outdated-feature-lockfile`
- `upgrade-feature-lockfile-dry-run`

Current local-only subset:
- local template fixtures
- local manifest-directory feature fixtures
- legacy `--lockfile` fixtures

Use the `compare_with_upstream` field in `manifest.example.json` to distinguish the two modes.
