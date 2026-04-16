# DEVELOP

Developer guide for building, testing, packaging, and releasing `deco`.

License: MIT  
Author: ilyar

## Prerequisites

- Rust stable with `cargo`
- Linux or macOS shell environment
- Docker if you want to validate runtime-oriented commands manually

## Dev Container

The repository includes [.devcontainer/devcontainer.json](/home/ilyar/startup/deco/repo/deco/.devcontainer/devcontainer.json:1) and [.devcontainer/Dockerfile](/home/ilyar/startup/deco/repo/deco/.devcontainer/Dockerfile:1).

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

## Quality Gates

The standard local gates are:

```sh
bash -n scripts/install.sh
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

Install the published Unix binary with:

```sh
curl -fsSL https://raw.githubusercontent.com/ilyar/deco/v1.0.0-alpha.1/scripts/install.sh | bash
```

Build a local optimized binary:

```sh
make build-release
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

The first public pre-release target is:

```text
v1.0.0-alpha.1
```

Release workflow model:

1. Push a semantic version tag such as `v1.0.0-alpha.1`.
2. GitHub Actions runs the release workflow.
3. The workflow:
   - validates version, lint, tests, and parity gates
   - builds release binaries for Linux, Windows, macOS, and FreeBSD
   - keeps `scripts/install.sh` available at the tagged raw GitHub URL for `curl | bash`
   - packages each binary with `README.md` and `LICENSE`
   - generates one `.sha256` file per archive
   - creates GitHub artifact attestations for supply-chain provenance
   - creates a GitHub Release and uploads the assets

Published targets for `v1.0.0-alpha.1`:

- `x86_64-unknown-linux-gnu`
- `aarch64-unknown-linux-musl`
- `x86_64-pc-windows-msvc`
- `x86_64-apple-darwin`
- `aarch64-apple-darwin`
- `x86_64-unknown-freebsd`

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
