run:
	cd ui && cargo run 

backtrace:
	cd ui && RUST_BACKTRACE=1 cargo run

test:
	cd ui && cargo test

test-lib:
	cd ui && cargo test --lib

test-all:
	cd ui && cargo test --all-targets --all-features

clippy:
	cd ui && cargo clippy

clippy-all:
	cd ui && cargo clippy --all-targets --all-features

clippy-fix:
	cd ui && cargo clippy --fix --lib

check:
	cd ui && cargo check

clean:
	cd ui && cargo clean

fmt:
	cd ui && cargo fmt

.PHONY: run backtrace test test-lib test-all clippy clippy-all clippy-fix check clean
