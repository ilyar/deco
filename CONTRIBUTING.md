# CONTRIBUTING

Contribution guide for `deco`.

## Before You Open a Pull Request

Run the required local checks from `repo/deco`:

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

## Scope Expectations

Good contributions are:

- small enough to review clearly
- explicit about behavior changes
- covered by tests when behavior changes
- aligned with the JSON `stdout` contract

Avoid:

- mixing unrelated refactors with feature work
- weakening lints or tests to make a change pass
- changing command output shape without updating tests and docs

## Runtime Changes

If a change affects Docker runtime behavior, do not stop at unit tests alone.

Add or run relevant manual validation where applicable:

```sh
deco build --workspace-folder /path/to/workspace
deco up --workspace-folder /path/to/workspace
deco exec --workspace-folder /path/to/workspace -- pwd
deco run-user-commands --workspace-folder /path/to/workspace
```

## Release and Tagging

Releases are produced from Git tags such as:

```text
v1.0.0-alpha.3
```

The tag-triggered workflow validates the release gates, builds the published binaries, publishes a GitHub Release, and generates artifact attestations for supply-chain provenance.

Use the checked-in dev container when you want a reproducible contributor environment for Rust tooling, Docker CLI access, and Linux-side release preparation.

The canonical local task entrypoint is [Justfile](./Justfile). Prefer `just` over ad hoc command sequences.

Expected published targets for `v1.0.0-alpha.3`:

- `x86_64-unknown-linux-gnu`
- `aarch64-unknown-linux-musl`
- `x86_64-pc-windows-msvc`
- `x86_64-apple-darwin`
- `aarch64-apple-darwin`
- `x86_64-unknown-freebsd`

## License

By contributing, you agree that your changes will be distributed under the MIT License in [LICENSE](./LICENSE).
