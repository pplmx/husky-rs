# Changelog

All notable changes to this project will be documented in this file.

## [0.3.3] - 2026-05-14

### 🐛 Bug Fixes

- **`cargo:rerun-if-changed` watched wrong directory**: The build script monitored husky-rs's own `.husky/` instead of the user's project `.husky/`. If `cargo build`/`cargo test` ran before `.husky/` existed, subsequent runs were cached and never re-ran — `core.hooksPath` was never set, hooks never triggered. (#15)
- **Submodule and worktree support**: `.git` files were not correctly parsed (missing `gitdir:` prefix stripping). Hooks were silently never installed in submodules and git worktrees.
- **`GitConfigFailed` now tolerated**: Build no longer fails if `git` is unavailable on PATH (e.g. NixOS without nix-shell). Users can fall back to `husky init` manually.

### ✨ Improvements

- **Windows compatibility**: Hook trigger tests now convert backslashes to forward slashes for Git Bash compatibility.
- **Comprehensive test coverage**: Added 17 new tests covering lazy hook creation, idempotent repeated execution, mixed `cargo test`/`cargo build`, hook triggering (pre-commit, commit-msg, post-commit, failure abort), CLI full workflow, and `git` failure graceful degradation.

## [0.3.2] - 2026-02-11

### 🚀 Improvements & Refinements

- **Lean Implementation**: Removed `.husky/_/husky.sh` in favor of direct skip logic in hook templates, keeping the `.husky/` directory clean.
- **Improved templates**: `husky add` now generates hooks with `[ "$HUSKY" = "0" ] && exit 0` for explicit and transparent skip logic.
- **Robustness**: Enhanced `build.rs` and `husky init` to gracefully handle environments without Git or outside a Git repository.
- **Incremental efficiency**: `build.rs` now skips redundant `git config` calls if `core.hooksPath` is already correctly set.

## [0.3.1] - 2026-02-11

### 🔧 Maintenance

- **Clippy Fixes**: Resolved various `needless_borrows_for_generic_args` warnings in the CLI tool.
- **Code Quality**: Cleaned up internal CLI logic and improved error feedback for manual configuration steps.

## [0.3.0] - 2026-02-11

### 🎯 Highlights

> This release marks a significant architectural shift to the **Modern Husky approach**, using Git's native `core.hooksPath` configuration for better performance, flexibility, and support for auxiliary scripts.

### ⚠️ Breaking Changes

- **Hook Location**: Hooks are now stored directly in `.husky/` instead of `.husky/hooks/`.
- **Mechanism Shift**: Switched from copying/wrapping hooks to using `git config core.hooksPath .husky`.
- **Directory Structure**: The `.husky/hooks/` subdirectory is no longer used; move your hooks to `.husky/`.
- **CLI Feature**: The `cli` feature has been removed. The `husky` binary is now included by default without extra configuration.

### ✨ New Features

- **Helper Script Support**: Arbitrary files (like `_helpers.sh`) can now be stored in `.husky/` and sourced from hooks.
- **Runtime Control**: Support for `HUSKY=0` environment variable to skip hooks at runtime (e.g., `HUSKY=0 git commit`).
- **CLI Enhancements**:
    - `husky init`: Now configures `core.hooksPath` immediately.
    - `husky uninstall`: New command to unset `core.hooksPath`.

---

## [0.2.2] - 2026-01-09

### 🧪 Testing

- **Test infrastructure refactoring**: Consolidated test utilities into `tests/common/mod.rs`
- **Test reorganization**: Merged 4 test files into 2 clear files (`test_installation.rs`, `test_hooks.rs`)
- **New test scenarios**: Git worktrees, multiple hooks (10 types), error message validation
- **Total**: 41 tests covering submodules, workspaces, worktrees, and edge cases

### 🔧 CI/CD

- **Code coverage**: Added `coverage.yml` workflow with tarpaulin and Codecov integration
- **Coverage badge**: Added to README

### 📦 Build

- **Makefile improvements**:
    - Added `make coverage` target
    - Added `make doc-check` target
    - Included `doc-check` in `make ci`
    - Fixed CRLF line endings
- **.gitignore**: Added coverage report files

---

## [0.2.1] - 2026-01-09

### 🐛 Bug Fixes

- **CI/Cross-platform**: Fixed lint errors, MSRV compliance (Rust 1.70), and race conditions in tests.
- **Windows Support**: Improved test stability on Windows by skipping shell syntax validation where `sh` is unavailable.
- **Makefile**: Added `make ci` and updated test commands to consistent with CI configuration.

---

## [0.2.0] - 2026-01-09

### 🎯 Highlights

> This release transforms husky-rs into a **production-ready** Git hooks manager with optional advanced features while maintaining the **zero-config** philosophy.

**Key additions:**

- 🔄 Smart hook change detection (no more `cargo clean`)
- 🛠️ Optional CLI tool (`husky init/add/list`)
- 📚 Optional library API for advanced use cases
- 📖 Comprehensive documentation suite

---

### ✨ New Features

#### Build Script Improvements

- **Granular rerun triggers** — Individual hook file monitoring
    - Hooks automatically reinstall when modified
    - No more `cargo clean` required
- **Validation & warnings** — Helpful feedback during build
    - Empty hook detection with clear warnings
    - Missing shebang detection with suggested defaults
- **Improved logging** — Clear, informative output
    - Per-hook status: `✓ pre-commit`, `✓ pre-push`
    - Summary: `Installed 3 Git hook(s)`

#### Optional CLI Tool

> Install with: `cargo install husky-rs --features=cli`

| Command            | Description                      |
| ------------------ | -------------------------------- |
| `husky init`       | Create `.husky/hooks` directory  |
| `husky add <hook>` | Create hook from smart template  |
| `husky list`       | List installed hooks with status |
| `husky help`       | Show help message                |
| `husky version`    | Display version                  |

#### Optional Library API

```rust
use husky_rs::{hooks_dir, should_skip_installation, is_valid_hook_name};
```

- `hooks_dir(path)` — Get hooks directory path
- `should_skip_installation()` — Check `NO_HUSKY_HOOKS` env var
- `is_valid_hook_name(name)` — Validate Git hook names
- `SUPPORTED_HOOKS` — All 27 supported hook names

---

### 📚 Documentation

| Document                                   | Description                                 |
| ------------------------------------------ | ------------------------------------------- |
| [Usage Guide](docs/usage.md)               | Installation, configuration, advanced usage |
| [Examples](docs/examples.md)               | 13 ready-to-use hook examples               |
| [Troubleshooting](docs/troubleshooting.md) | Common issues and solutions                 |
| [Contributing](CONTRIBUTING.md)            | Development guide                           |

**Example collection includes:**

1. Running tests before commit
2. Code formatting check
3. Clippy linting
4. Conventional commits validation
5. Branch protection
6. Comprehensive pre-push checks
7. CI environment detection
8. Multi-language project support
9. And more...

---

### 🧪 Testing

| Metric             | Before | After | Change    |
| ------------------ | ------ | ----- | --------- |
| Total tests        | 13     | 48    | **+269%** |
| CLI coverage       | 0%     | 90%   | —         |
| Lib coverage       | 40%    | 100%  | —         |
| Example validation | 0%     | 54%   | —         |

---

### ⚠️ Breaking Changes

**Build script auto-detection**: Hooks now automatically reinstall when source files change.

- **Impact**: Workflows relying on manual hook reinstallation may behave differently
- **Migration**: Use `NO_HUSKY_HOOKS=1` if you need manual control
- **Benefit**: Significantly improved developer experience

---

### 📦 Package Updates

- **Version**: `0.1.5` → `0.2.0`
- **MSRV**: Rust 1.70+
- **Keywords**: `git`, `hooks`, `husky`, `development-tools`, `pre-commit`
- **Categories**: `development-tools`, `command-line-utilities`

---

## [0.1.5] - 2025-06-03

### 🐛 Bug Fixes

- Resolve cross-drive temp dir issue on Windows runners
- Simplify build script testing

### 🧪 Testing

- Improve test coverage for dependency types

---

## [0.1.4] - 2025-05-30

### 🚀 Features

- Refactor build script with better symlink handling
- Enhance test coverage

---

## [0.1.3] - 2025-05-25

### 🎯 Features

- Simplify installation process
- Improve hook installation logging

### 🐛 Bug Fixes

- Fix hook installation on Windows

---

## [0.1.2] - 2025-05-20

### 🚀 Features

- Add comprehensive error handling
- Support for Git submodules

---

## [0.1.1] - 2025-05-15

### 🐛 Bug Fixes

- Fix permission issues on Unix systems
- Improve error messages

---

## [0.1.0] - 2025-05-10

### 🎉 Initial Release

- Basic Git hooks installation via `build.rs`
- Support for all standard Git hooks
- Cross-platform compatibility
- Zero-configuration setup

---

[0.3.2]: https://github.com/pplmx/husky-rs/compare/v0.3.1...v0.3.2
[0.3.1]: https://github.com/pplmx/husky-rs/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/pplmx/husky-rs/compare/v0.2.2...v0.3.0
[0.2.2]: https://github.com/pplmx/husky-rs/compare/v0.2.1...v0.2.2
[0.2.1]: https://github.com/pplmx/husky-rs/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/pplmx/husky-rs/compare/v0.1.5...v0.2.0
[0.1.5]: https://github.com/pplmx/husky-rs/compare/v0.1.4...v0.1.5
[0.1.4]: https://github.com/pplmx/husky-rs/compare/v0.1.3...v0.1.4
[0.1.3]: https://github.com/pplmx/husky-rs/compare/v0.1.2...v0.1.3
[0.1.2]: https://github.com/pplmx/husky-rs/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/pplmx/husky-rs/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/pplmx/husky-rs/releases/tag/v0.1.0
