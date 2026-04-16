.PHONY: fmt lint test parity install build-release ci verify-self-devcontainer

fmt:
	just fmt

lint:
	just lint

test:
	just test

parity:
	just parity

install:
	just install

build-release:
	just build-release

ci:
	just ci

verify-self-devcontainer:
	just verify-self-devcontainer
