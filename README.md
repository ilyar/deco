# deco

`deco` is a Rust-first dev container CLI for reading devcontainer configs, preparing a local runtime, and running common devcontainer workflows from the terminal.

License: MIT  
Author: ilyar

## Who This Is For

Use `deco` if you want to:

- inspect a local `.devcontainer/devcontainer.json`
- build or start a devcontainer-oriented runtime
- run commands inside that runtime
- execute lifecycle hooks from the config
- inspect local features, templates, and lockfiles

## Current Status

`deco` is being prepared for the first public alpha release: `v1.0.0-alpha.1`.

What is already covered:

- JSONC devcontainer parsing
- image, Dockerfile, and baseline compose config support
- `read-configuration`, `build`, `up`, `exec`, `run-user-commands`, `set-up`
- local `features`, `templates`, `outdated`, and `upgrade` flows
- machine-readable JSON output on `stdout`

What is still intentionally incomplete:

- full parity with upstream `devcontainer-cli`
- broad Docker runtime coverage in CI
- advanced publishing flows for features and templates

## Install

From the repository root:

```sh
cargo install --path . --locked
deco --version
deco --help
```

For local development without install:

```sh
cargo run -p deco -- --help
```

More detail: [DEVELOP.md](/home/ilyar/startup/deco/repo/deco/DEVELOP.md:1)

## Typical User Flow

Assume a workspace like this:

```text
my-app/
  .devcontainer/
    devcontainer.json
```

### Inspect the config

```sh
deco read-configuration --workspace-folder /path/to/my-app
```

Point to a specific file if needed:

```sh
deco read-configuration \
  --config /path/to/my-app/.devcontainer/devcontainer.json
```

### Build the runtime

```sh
deco build --workspace-folder /path/to/my-app
```

### Start the runtime

```sh
deco up --workspace-folder /path/to/my-app
```

### Run a command inside the runtime

```sh
deco exec --workspace-folder /path/to/my-app -- pwd
deco exec --workspace-folder /path/to/my-app -- cargo test
```

If you already know the container id:

```sh
deco exec --container-id <container-id> -- env
```

### Run lifecycle hooks from the config

```sh
deco run-user-commands --workspace-folder /path/to/my-app
```

### Run the combined setup flow

```sh
deco set-up --workspace-folder /path/to/my-app
```

## Command Examples

### `read-configuration`

```sh
deco read-configuration --workspace-folder /path/to/my-app
deco read-configuration --workspace-folder /path/to/my-app --include-merged-configuration
```

### `build`

```sh
deco build --workspace-folder /path/to/my-app
```

### `up`

```sh
deco up --workspace-folder /path/to/my-app
```

### `exec`

```sh
deco exec --workspace-folder /path/to/my-app -- pwd
deco exec --workspace-folder /path/to/my-app -- cargo check
deco exec --container-id abc123 -- ls -la
```

### `run-user-commands`

```sh
deco run-user-commands --workspace-folder /path/to/my-app
```

### `set-up`

```sh
deco set-up --workspace-folder /path/to/my-app
```

### `features`

Inspect a local features directory:

```sh
deco features --manifest-dir /path/to/features
```

Resolve dependencies:

```sh
deco features resolve-dependencies --manifest-dir /path/to/features
deco features resolve-dependencies --workspace-folder /path/to/my-app
```

Run local manifest checks:

```sh
deco features test --manifest-dir /path/to/features
```

### `templates`

Show template metadata from a local collection:

```sh
deco templates metadata --manifest-path /path/to/templates
```

Apply a template:

```sh
deco templates apply \
  --manifest-path /path/to/templates \
  --template-id alpha \
  --target-dir /tmp/template-output
```

### `outdated`

```sh
deco outdated --workspace-folder /path/to/my-app
deco outdated --lockfile /path/to/deco-lock.json
```

### `upgrade`

```sh
deco upgrade --workspace-folder /path/to/my-app --dry-run
deco upgrade --lockfile /path/to/deco-lock.json
```

## Output Contract

`deco` writes machine-readable command results to `stdout`.

Diagnostics and logs go to `stderr`.

That means scripting should usually redirect `stdout`, for example:

```sh
deco read-configuration --workspace-folder /path/to/my-app > result.json
```

More detail: [docs/stderr-convention.md](/home/ilyar/startup/deco/repo/deco/docs/stderr-convention.md:1)

## More Documentation

- [DEVELOP.md](/home/ilyar/startup/deco/repo/deco/DEVELOP.md:1) for building, testing, releasing, and verifying artifacts
- [CONTRIBUTING.md](/home/ilyar/startup/deco/repo/deco/CONTRIBUTING.md:1) for contribution workflow and review expectations
- [docs/installing.md](/home/ilyar/startup/deco/repo/deco/docs/installing.md:1) for install and local release commands
