default: all

all: lint build test

test: build
	cargo test

build:
	$(MAKE) -C ../token build || break;
	cargo build --target wasm32v1-none --release

lint: fmt clippy

fmt:
	cargo fmt --all

clippy: build
	cargo clippy --all-targets -- -D warnings

clean:
	cargo clean
