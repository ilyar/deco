set shell := ["bash", "-euo", "pipefail", "-c"]

default:
  @just --list

install-script-check:
  bash -n install.sh
  bash -n scripts/install.sh

fmt:
  cargo fmt --check

lint:
  cargo clippy --workspace --all-targets -- -D warnings

build-root:
  cargo build -q -p deco

test-workspace:
  cargo test -q --workspace --exclude deco-cli

test-cli-lib:
  CARGO_BIN_EXE_deco="$PWD/target/debug/deco" cargo test -q -p deco-cli --lib

test-cli-smoke:
  CARGO_BIN_EXE_deco="$PWD/target/debug/deco" cargo test -q -p deco-cli --test cli_smoke

test-parity:
  CARGO_BIN_EXE_deco="$PWD/target/debug/deco" cargo test -q -p deco-cli --test parity_harness

parity: test-parity

test: build-root test-workspace test-cli-lib test-cli-smoke test-parity

ci: install-script-check fmt lint test verify-self-devcontainer

install:
  cargo install --path . --locked

build-release:
  cargo build --release --locked -p deco

build-release-target target:
  cargo build --release --locked -p deco --target {{target}}

package-unix version target binary out="dist":
  chmod +x scripts/package-unix.sh
  scripts/package-unix.sh "{{version}}" "{{target}}" "{{binary}}" "{{out}}"

package-windows version target binary out="dist":
  pwsh -File ./scripts/package-windows.ps1 -Version "{{version}}" -Target "{{target}}" -BinaryPath "{{binary}}" -OutputDir "{{out}}"

verify-self-devcontainer:
  cargo run -q -p deco -- read-configuration --workspace-folder . >/dev/null

deco-read:
  cargo run -q -p deco -- read-configuration --workspace-folder .

deco-build:
  cargo run -q -p deco -- build --workspace-folder .

deco-up:
  cargo run -q -p deco -- up --workspace-folder .

deco-setup:
  cargo run -q -p deco -- set-up --workspace-folder .

deco-run-user-commands:
  cargo run -q -p deco -- run-user-commands --workspace-folder .

deco-exec +args:
  cargo run -q -p deco -- exec --workspace-folder . -- {{args}}
