run:
	rm -f logs/quetty.log && cargo run --bin quetty

backtrace:
	rm -f logs/quetty.log && RUST_BACKTRACE=1 cargo run --bin quetty

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

test-server:
	@if [ -z "$(QUEUE)" ]; then \
		echo "Usage: make test-server QUEUE=<queue-name>"; \
		echo ""; \
		echo "Example: make test-server QUEUE=my-test-queue"; \
		echo ""; \
		echo "Environment variables (can be set in .env file or system environment):"; \
		echo "  SERVICEBUS__ENCRYPTED_CONNECTION_STRING  Required: Encrypted Azure Service Bus connection string"; \
		echo "  TRAFFIC_MIN_RATE             Optional: Minimum messages per minute (default: 60)"; \
		echo "  TRAFFIC_MAX_RATE             Optional: Maximum messages per minute (default: 180)"; \
		echo "  TRAFFIC_MESSAGE_PREFIX       Optional: Message prefix (default: 'TrafficSim')"; \
		echo ""; \
		echo "Create a .env file in the project root with your connection string for easier usage"; \
		exit 1; \
	fi
	cd traffic-simulator && cargo run $(QUEUE)

.PHONY: run backtrace test test-lib test-all clippy clippy-all clippy-fix check clean fmt test-server
