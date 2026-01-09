.PHONY: help build test fmt fmt-check clippy fix doc clean ci
.DEFAULT_GOAL := help

APP_NAME := husky-rs

# Build the project
build:
	@cargo build --release

# Run tests (same as CI)
test:
	@cargo test --all-features

# Format code
fmt:
	@cargo fmt --all

# Check formatting (CI style)
fmt-check:
	@cargo fmt --all --check

# Run clippy (same as CI)
clippy:
	@cargo clippy --all-targets --all-features --workspace -- -D warnings

# Run all checks (locally simulate CI)
ci: fmt-check clippy test

# Fix
fix:
	@cargo clippy --fix --allow-dirty --allow-staged --all-targets --all-features --workspace -- -D warnings
	@cargo fmt --all

# Generate documentation
doc:
	@cargo doc --no-deps

# Clean build artifacts
clean:
	@cargo clean
	@rm -rf target

# Show help
help:
	@echo ""
	@echo "Usage:"
	@echo "    make [target]"
	@echo ""
	@echo "Targets:"
	@awk '/^[a-zA-Z\-_0-9]+:/ \
	{ \
		helpMessage = match(lastLine, /^# (.*)/); \
		if (helpMessage) { \
			helpCommand = substr($$1, 0, index($$1, ":")-1); \
			helpMessage = substr(lastLine, RSTART + 2, RLENGTH); \
			printf "\033[36m%-22s\033[0m %s\n", helpCommand,helpMessage; \
		} \
	} { lastLine = $$0 }' $(MAKEFILE_LIST)
