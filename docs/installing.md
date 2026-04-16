# Installing and Releasing

License: MIT
Author: ilyar

## Download Prebuilt Releases

The first public pre-release is:

```text
v1.0.0-alpha.1
```

Download it from:

- `https://github.com/ilyar/deco/releases/tag/v1.0.0-alpha.1`

Available release assets:

- `deco-v1.0.0-alpha.1-x86_64-unknown-linux-gnu.tar.gz`
- `deco-v1.0.0-alpha.1-aarch64-unknown-linux-musl.tar.gz`
- `deco-v1.0.0-alpha.1-x86_64-pc-windows-msvc.zip`
- `deco-v1.0.0-alpha.1-x86_64-apple-darwin.tar.gz`
- `deco-v1.0.0-alpha.1-aarch64-apple-darwin.tar.gz`
- `deco-v1.0.0-alpha.1-x86_64-unknown-freebsd.tar.gz`

Each archive has a matching `.sha256` file and a GitHub artifact attestation.

## Prerequisites

- Rust stable toolchain with `cargo`
- Linux or macOS shell environment

Run all commands from `repo/deco`.

## Local development run

Use the canonical root entrypoint during development:

```sh
cargo run -p deco -- --help
cargo run -p deco -- --version
```

## Install from source

Install `deco` into Cargo's standard bin directory:

```sh
cargo install --path . --locked
deco --help
deco --version
```

## Install from a release archive

Linux, macOS, and FreeBSD example:

```sh
tar -xzf deco-v1.0.0-alpha.1-<target>.tar.gz
./deco-v1.0.0-alpha.1-<target>/deco --version
```

Windows PowerShell example:

```powershell
Expand-Archive deco-v1.0.0-alpha.1-x86_64-pc-windows-msvc.zip
.\deco-v1.0.0-alpha.1-x86_64-pc-windows-msvc\deco.exe --version
```

## Local quality gates

```sh
make fmt
make lint
make test
make parity
```

The maintained developer guide lives in [DEVELOP.md](/home/ilyar/startup/deco/repo/deco/DEVELOP.md:1).
The contribution workflow lives in [CONTRIBUTING.md](/home/ilyar/startup/deco/repo/deco/CONTRIBUTING.md:1).

## Local release build

Build an optimized local artifact:

```sh
make build-release
```

The release binary will be available at:

```text
target/release/deco
```

If you need a local archive, create it explicitly from the built binary, for example:

```sh
scripts/package-unix.sh "$(cargo run -q -p deco -- --version | awk '{print $2}')" \
  "$(rustc -vV | awk '/host:/ {print $2}')" \
  target/release/deco
```
