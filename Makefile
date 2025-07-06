run:
	cd ui && rm -f quetty.log && cargo run 

backtrace:
	cd ui && rm -f quetty.log && RUST_BACKTRACE=1 cargo run

test:
	cargo test

test-lib:
	cargo test --lib

test-all:
	cargo test --all-targets --all-features

clippy:
	cargo clippy

clippy-all:
	cargo clippy --all-targets --all-features

clippy-fix:
	cargo clippy --fix --lib

check:
	cargo check

clean:
	cargo clean

fmt:
	cargo fmt

.PHONY: run backtrace test test-lib test-all clippy clippy-all clippy-fix check clean
