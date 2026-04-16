# DEVELOP

Developer guide for building, testing, and releasing `deco`.

License: MIT  
Author: ilyar

## Prerequisites

- Rust stable with `cargo`
- Linux or macOS shell environment
- Docker if you want to validate runtime-oriented commands manually

Run all commands from `repo/deco`.

## Local Development

Use the root package entrypoint:

```sh
cargo run -p deco -- --help
cargo run -p deco -- --version
```

## Quality Gates

The standard local gates are:

```sh
make fmt
make lint
make test
make parity
```

Equivalent raw commands:

```sh
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test -q
cargo test -q -p deco-cli --test parity_harness
```

## Install and Release Build

Install from source:

```sh
cargo install --path . --locked
deco --version
```

Build a local optimized binary:

```sh
make build-release
ls -l target/release/deco
```

Create a local archive if needed:

```sh
tar -czf deco-$(cargo run -q -p deco -- --version | awk '{print $2}').tar.gz LICENSE -C target/release deco
```

## Release Process

The first public pre-release target is:

```text
v1.0.0-alpha.1
```

Release workflow model:

1. Push a semantic version tag such as `v1.0.0-alpha.1`.
2. GitHub Actions runs the release workflow.
3. The workflow:
   - builds the release binary
   - packages the binary and `LICENSE`
   - generates `SHA256SUMS`
   - creates GitHub artifact attestations for supply-chain provenance
   - creates a GitHub Release and uploads the assets

Create and push the tag:

```sh
git tag v1.0.0-alpha.1
git push origin v1.0.0-alpha.1
```

## Verifying Release Provenance

GitHub artifact attestations are enabled for release artifacts.

After downloading a published artifact, consumers can verify provenance with GitHub CLI:

```sh
gh attestation verify PATH/TO/ARTIFACT -R <owner>/<repo>
```

Official GitHub reference:
- https://docs.github.com/en/actions/how-tos/secure-your-work/use-artifact-attestations/use-artifact-attestations

## Notes About Runtime Validation

CI stays mostly Docker-independent today. That is intentional, but it means runtime-heavy changes should also be checked manually with Docker when relevant.

Useful manual checks:

```sh
deco read-configuration --workspace-folder /path/to/workspace
deco build --workspace-folder /path/to/workspace
deco up --workspace-folder /path/to/workspace
deco exec --workspace-folder /path/to/workspace -- pwd
deco run-user-commands --workspace-folder /path/to/workspace
```
