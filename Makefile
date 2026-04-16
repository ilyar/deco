.PHONY: fmt lint test parity install build-release

fmt:
	cargo fmt --check

lint:
	cargo clippy --workspace --all-targets -- -D warnings

test:
	cargo test -q

parity:
	cargo test -q -p deco-cli --test parity_harness

install:
	cargo install --path . --locked

build-release:
	cargo build --release
