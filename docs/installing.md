# Installing and Releasing

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

## Local quality gates

```sh
make fmt
make lint
make test
make parity
```

The same commands are documented in `docs/contributing.md`.

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
tar -czf deco-$(cargo pkgid | sed 's/.*#//').tar.gz -C target/release deco
```
