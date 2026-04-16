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

The current skeleton only validates manifest loading and fixture shape.
