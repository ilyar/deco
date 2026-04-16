# CONTRIBUTING

Contribution guide for `deco`.

License: MIT  
Author: ilyar

## Before You Open a Pull Request

Run the required local checks from `repo/deco`:

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
v1.0.0-alpha.1
```

The tag-triggered workflow builds the release artifacts, publishes a GitHub Release, and generates artifact attestations for supply-chain provenance.

## License

By contributing, you agree that your changes will be distributed under the MIT License in [LICENSE](/home/ilyar/startup/deco/repo/deco/LICENSE:1).
