# Changelog

All notable changes to this project will be documented in this file.

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

| Command | Description |
| --------- | ------------- |
| `husky init` | Create `.husky/hooks` directory |
| `husky add <hook>` | Create hook from smart template |
| `husky list` | List installed hooks with status |
| `husky help` | Show help message |
| `husky version` | Display version |

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

| Document | Description |
| ---------- | ------------- |
| [Usage Guide](docs/usage.md) | Installation, configuration, advanced usage |
| [Examples](docs/examples.md) | 13 ready-to-use hook examples |
| [Troubleshooting](docs/troubleshooting.md) | Common issues and solutions |
| [Contributing](CONTRIBUTING.md) | Development guide |

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

| Metric | Before | After | Change |
| --------- | -------- | ------- | -------- |
| Total tests | 13 | 48 | **+269%** |
| CLI coverage | 0% | 90% | — |
| Lib coverage | 40% | 100% | — |
| Example validation | 0% | 54% | — |

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

[0.2.1]: https://github.com/pplmx/husky-rs/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/pplmx/husky-rs/compare/v0.1.5...v0.2.0
[0.1.5]: https://github.com/pplmx/husky-rs/compare/v0.1.4...v0.1.5
[0.1.4]: https://github.com/pplmx/husky-rs/compare/v0.1.3...v0.1.4
[0.1.3]: https://github.com/pplmx/husky-rs/compare/v0.1.2...v0.1.3
[0.1.2]: https://github.com/pplmx/husky-rs/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/pplmx/husky-rs/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/pplmx/husky-rs/releases/tag/v0.1.0
