# Contributing

Every change in `repo/deco` is expected to pass the same local quality gates as CI.

Run these commands from `repo/deco` before opening or updating a pull request:

```sh
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test -q
cargo test -q -p deco-cli --test parity_harness
```

Equivalent helper targets:

```sh
make fmt
make lint
make test
make parity
```

CI intentionally stays Docker-independent:

- the required parity check is the local `deco-cli` parity harness only;
- upstream-backed parity remains opt-in and is not part of the merge gate;
- runtime Docker integration scenarios should be validated separately when a change needs them.

If one command fails, fix the root cause instead of weakening the test or lint configuration.
