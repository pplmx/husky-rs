.PHONY: help build test fmt fmt-check clippy fix doc doc-check clean ci coverage

.DEFAULT_GOAL := help

# Build release binary
build:
	@cargo build --release

# Run all tests
test:
	@cargo test --all-features --workspace

# Run tests by cargo-nextest(A much more modern test runner)
nextest:
	@cargo nextest run --all-features --workspace

# Format code
fmt:
	@cargo fmt --all

# Check formatting (CI style)
fmt-check:
	@cargo fmt --all --check

# Run clippy (CI style)
clippy:
	@cargo clippy --all-targets --all-features --workspace -- -D warnings

# Check documentation (CI style)
doc-check:
	@RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --document-private-items --all-features --workspace

# Run all CI checks locally
ci: fmt-check clippy doc-check test

# Auto-fix clippy warnings and format
fix:
	@cargo clippy --fix --allow-dirty --allow-staged --all-targets --all-features --workspace -- -D warnings
	@cargo fmt --all

# Generate documentation
doc:
	@cargo doc --no-deps --open

# Generate coverage report (requires cargo-tarpaulin)
coverage:
	@cargo tarpaulin --all-features --workspace --exclude-files 'src/bin/*'

# Clean build artifacts
clean:
	@cargo clean

# Show help
help:
	@echo ""
	@echo "Usage: make [target]"
	@echo ""
	@echo "Targets:"
	@awk '/^[a-zA-Z\-_0-9]+:/ { \
		helpMessage = match(lastLine, /^# (.*)/); \
		if (helpMessage) { \
			helpCommand = substr($$1, 0, index($$1, ":")-1); \
			helpMessage = substr(lastLine, RSTART + 2, RLENGTH); \
			printf "  \033[36m%-12s\033[0m %s\n", helpCommand, helpMessage; \
		} \
	} { lastLine = $$0 }' $(MAKEFILE_LIST)
	@echo ""
