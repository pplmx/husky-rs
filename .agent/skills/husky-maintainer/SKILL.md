---
name: Husky Maintainer
description: Specific workflows and standards for maintaining the husky-rs project.
---

# Husky Maintainer Checklist

This skill encapsulates the specific workflows for the `husky-rs` project.

## Development Environment

- **Rust Version**: The MSRV (Minimum Supported Rust Version) is **1.83**. All changes must be compatible with this version.
    - Use `just msrv-check` to verify compatibility.
- **Build System**: This project uses a `justfile` to simplify common tasks (the old `Makefile` is gone).
    - **Run CI locally**: `just ci` (runs fmt, clippy, doc-check, msrv-check, test, deny, and doctest).
    - **Fix Style**: `just fix` (auto-fixes clippy issues and formatting).
    - **Coverage**: `just coverage` (generates tarpaulin report).

## Project Structure

- **Binaries**: The main CLI is in `src/bin/husky.rs`.
- **Library**: Core logic is in `src/lib.rs`.
- **Tests**: Integration tests are in `tests/`.

## Workflow Guidelines

1. **Before Committing**:
    - Run `just fix` to ensure code is formatted and clippy-clean.
    - Run `just test` (or `just nextest` if you have it installed) to verify correctness.
2. **Making Changes**:
    - If modifying the CLI, check `tests/test_cli.rs` for regression tests.
    - If adding a feature, update `CHANGELOG.md` under the `[Unreleased]` section.
    - Ensure `Cargo.toml` metadata remains accurate.
3. **Documentation**:
    - `docs/` contains detailed usage guides. Update them if CLI flags change.
    - Run `just doc-check` to ensure no broken links or invalid code blocks in docs.

## Release Process

- Update version in `Cargo.toml`.
- Update `CHANGELOG.md`:
    - Rename `[Unreleased]` to the new version `[0.x.y] - YYYY-MM-DD`.
    - Create a new empty `[Unreleased]` section at the top.
    - Update comparison links at the bottom of the file.
- Tag the commit with `v0.x.y`.
