# husky-rs justfile — git hooks manager
default:
    @just --list

# Build release binary
build:
    cargo build --release

# Run all tests
test:
    cargo test --all-features --workspace

# Run tests with nextest (faster)
nextest:
    cargo nextest run --all-features --workspace

# Format check (CI style)
fmt-check:
    cargo fmt --all --check

# Run clippy (CI style)
clippy:
    cargo clippy --all-targets --all-features --workspace -- -D warnings

# Check documentation (CI style)
doc-check:
    RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --document-private-items --all-features --workspace

# MSRV check (keep in sync with rust-version in Cargo.toml)
msrv-check:
    cargo +1.78 check --all-targets --all-features --workspace

# Generate coverage report
coverage:
    cargo tarpaulin --all-features --workspace --exclude-files 'src/bin/*'

# Quick read-only checks (local loop)
quick: fmt-check clippy doc-check test

# Full CI gate
ci: fmt-check clippy doc-check msrv-check test deny

# Auto-fix clippy + format
fix:
    cargo clippy --fix --allow-dirty --allow-staged --all-targets --all-features --workspace -- -D warnings
    cargo fmt --all

# Security audit
audit:
    cargo audit

# Dependency policy check
deny:
    cargo deny check

# Generate documentation
doc:
    cargo doc --no-deps --open

# Clean build artifacts
clean:
    cargo clean
