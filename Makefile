.PHONY: help build test fmt clippy fix doc image clean
.DEFAULT_GOAL := help

APP_NAME := husky-rs

# Build the project
build:
	@cargo build --release

# Run tests
test:
	@cargo test

# Format code
fmt:
	@cargo fmt

# Run clippy
clippy:
	@cargo clippy -- -D warnings

# Fix
fix:
	@cargo clippy --fix --allow-dirty --allow-staged --all-targets --all-features --workspace -- -D warnings

# Generate documentation
doc:
	@cargo doc --no-deps

# Build image
image:
	@docker image build -t $(APP_NAME) .

# Start a compose service
compose-up:
	@docker compose -f ./compose.yml -p $(APP_NAME) up -d

# Shutdown a compose service
compose-down:
	@docker compose -f ./compose.yml down

# Clean build artifacts
clean:
	@cargo clean
	@rm -rf target
	@docker compose -f ./compose.yml down -v
	@docker image rm -f $(APP_NAME)

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
