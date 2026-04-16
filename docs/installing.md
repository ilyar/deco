# Installing and Releasing

License: MIT
Author: ilyar

## Download Prebuilt Releases

The current public pre-release is:

```text
v1.0.0-alpha.2
```

Download it from:

- `https://github.com/ilyar/deco/releases/tag/v1.0.0-alpha.2`

Available release assets:

- `deco-v1.0.0-alpha.2-x86_64-unknown-linux-gnu.tar.gz`
- `deco-v1.0.0-alpha.2-aarch64-unknown-linux-musl.tar.gz`
- `deco-v1.0.0-alpha.2-x86_64-pc-windows-msvc.zip`
- `deco-v1.0.0-alpha.2-x86_64-apple-darwin.tar.gz`
- `deco-v1.0.0-alpha.2-aarch64-apple-darwin.tar.gz`
- `deco-v1.0.0-alpha.2-x86_64-unknown-freebsd.tar.gz`

Each archive has a matching `.sha256` file and a GitHub artifact attestation.

## Prerequisites

- Rust stable toolchain with `cargo`
- Linux, macOS, or FreeBSD shell environment

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

## Install with `curl | bash`

For Linux, macOS, and FreeBSD, install the published binary directly:

```sh
curl -fsSL https://raw.githubusercontent.com/ilyar/deco/v1.0.0-alpha.2/scripts/install.sh | bash
```

Optional flags:

```sh
curl -fsSL https://raw.githubusercontent.com/ilyar/deco/v1.0.0-alpha.2/scripts/install.sh | \
  bash -s -- --install-dir "$HOME/.local/bin" --version v1.0.0-alpha.2
```

The script:

- detects the current OS and architecture
- downloads the matching release archive and `.sha256`
- verifies the checksum
- installs `deco` into `~/.local/bin` by default

## Install from a release archive

Linux, macOS, and FreeBSD example:

```sh
tar -xzf deco-v1.0.0-alpha.2-<target>.tar.gz
./deco-v1.0.0-alpha.2-<target>/deco --version
```

Windows PowerShell example:

```powershell
Expand-Archive deco-v1.0.0-alpha.2-x86_64-pc-windows-msvc.zip
.\deco-v1.0.0-alpha.2-x86_64-pc-windows-msvc\deco.exe --version
```

Windows users should download the `.zip` asset from the release page directly.

## Local quality gates

```sh
just ci
```

The maintained developer guide lives in [DEVELOP.md](/home/ilyar/startup/deco/repo/deco/DEVELOP.md:1).
The contribution workflow lives in [CONTRIBUTING.md](/home/ilyar/startup/deco/repo/deco/CONTRIBUTING.md:1).

## Local release build

Build an optimized local artifact:

```sh
just build-release
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
