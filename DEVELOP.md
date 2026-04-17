# DEVELOP

Developer guide for building, testing, packaging, and releasing `deco`.

## Prerequisites

- Rust stable with `cargo`
- Linux or macOS shell environment
- Docker if you want to validate runtime-oriented commands manually

## Dev Container

The repository includes [.devcontainer/devcontainer.json](./.devcontainer/devcontainer.json) and [.devcontainer/Dockerfile](./.devcontainer/Dockerfile).

It is the intended reproducible environment for:

- day-to-day Rust development
- running `fmt`, `clippy`, `test`, and parity checks
- preparing Linux-side release builds and GitHub release operations

Run all commands from `repo/deco`.

## Local Development

Use the root package entrypoint:

```sh
cargo run -p deco -- --help
cargo run -p deco -- --version
```

The preferred task runner is [Justfile](./Justfile).

Typical commands:

```sh
just ci
just build-release
just verify-self-devcontainer
just deco-read
just deco-build
just deco-up
just deco-exec pwd
```

`Makefile` is kept only as a thin compatibility wrapper around `just`.

## Quality Gates

The standard local gates are:

```sh
just ci
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

Install the published Unix binary with:

```sh
curl -fsSL https://raw.githubusercontent.com/ilyar/deco/v1.0.0-alpha.2/scripts/install.sh | bash
```

Build a local optimized binary:

```sh
just build-release
ls -l target/release/deco
```

Create a local archive if needed:

```sh
scripts/package-unix.sh \
  "$(cargo run -q -p deco -- --version | awk '{print $2}')" \
  "$(rustc -vV | awk '/host:/ {print $2}')" \
  target/release/deco
```

## Release Process

The current public pre-release target is:

```text
v1.0.0-alpha.2
```

Release workflow model:

1. Push a semantic version tag such as `v1.0.0-alpha.2`.
2. GitHub Actions runs the release workflow.
3. The workflow:
   - validates version, lint, tests, parity gates, and the checked-in `.devcontainer` through `just ci`
   - builds release binaries for Linux, Windows, macOS, and FreeBSD
   - keeps `scripts/install.sh` available at the tagged raw GitHub URL for `curl | bash`
   - packages each binary with `README.md` and `LICENSE`
   - generates one `.sha256` file per archive
   - creates GitHub artifact attestations for supply-chain provenance
   - creates a GitHub Release and uploads the assets

Published targets for `v1.0.0-alpha.2`:

- `x86_64-unknown-linux-gnu`
- `aarch64-unknown-linux-musl`
- `x86_64-pc-windows-msvc`
- `x86_64-apple-darwin`
- `aarch64-apple-darwin`
- `x86_64-unknown-freebsd`

Create and push the tag:

```sh
git tag v1.0.0-alpha.2
git push origin v1.0.0-alpha.2
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
