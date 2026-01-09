# husky-rs

[![CI](https://github.com/pplmx/husky-rs/workflows/CI/badge.svg)](https://github.com/pplmx/husky-rs/actions)
[![Crates.io](https://img.shields.io/crates/v/husky-rs.svg)](https://crates.io/crates/husky-rs)
[![Documentation](https://docs.rs/husky-rs/badge.svg)](https://docs.rs/husky-rs)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](#license)

`husky-rs` is a Git hooks management tool for Rust projects, inspired by Husky.

## Features

- 🚀 **Zero-configuration** - Just add the dependency and create hooks
- ⚡ **Automatic installation** - Hooks install on `cargo build` or `cargo test`
- 🔄 **Smart rerun detection** - No need for `cargo clean` when updating hooks
- ✅ **Comprehensive validation** - Warns about empty hooks and missing shebangs
- 🎯 **All 27 Git hooks supported** - Client-side and server-side hooks
- 🌍 **Cross-platform** - Works on Unix-like systems and Windows
- 🛠️ **Optional CLI tool** - `husky init`, `husky add`, `husky list` commands
- 📚 **Optional library API** - Helper functions for advanced use cases

## Quick Start

1. Adding `husky-rs` to your project:

   You have several options:

   ```sh
   # Option 1: Add as a Regular Dependency
   cargo add husky-rs

   # Option 2: Add as a Dev Dependency
   cargo add --dev husky-rs

   # Option 3: Use the Main Branch
   cargo add --git https://github.com/pplmx/husky-rs --branch main
   cargo add --dev --git https://github.com/pplmx/husky-rs --branch main
   ```

2. Create hooks directory:

   ```sh
   mkdir -p .husky/hooks
   ```

3. Add a hook (e.g., `pre-commit`):

   ```sh
   echo '#!/bin/sh\necho "Running pre-commit hook"' > .husky/hooks/pre-commit
   ```

4. Install hooks:

   ```sh
   cargo build
   ```

   Or if you're using as a dev-dependency:

   ```sh
   cargo test
   ```

**Tip:** If you add this library to the `[dependencies]` section, both `cargo build` and `cargo test` will work. However, if it's added under `[dev-dependencies]`, only `cargo test` will function as expected.

## Usage

### Supported Git Hooks

`husky-rs` supports all 27 Git hooks, including:

- `pre-commit` - Run before commit is created
- `prepare-commit-msg` - Edit commit message before committing
- `commit-msg` - Validate commit message format
- `post-commit` - Run after commit is created
- `pre-push` - Run before pushing to remote
- And 22 more...

For a complete list, refer to the [Git documentation](https://git-scm.com/docs/githooks).

If you encounter any unsupported hooks, please [open an issue](https://github.com/pplmx/husky-rs/issues).

### Configuration

To skip hook installation (useful in CI environments):

```sh
NO_HUSKY_HOOKS=1 cargo build
```

You can also set this in your environment or CI configuration.

## Optional Tools

### CLI Tool

For added convenience, install the `husky` command-line tool:

```sh
cargo install husky-rs --features=cli
```

The CLI provides helpful commands:

```sh
husky init              # Create .husky/hooks directory
husky add pre-commit    # Add hook from smart template
husky list              # List all installed hooks
husky help              # Show help
```

*Note*: The CLI is completely optional - the core functionality works without it!

### Library API

For advanced use cases, husky-rs exposes utility functions:

```rust
use husky_rs::{hooks_dir, should_skip_installation, is_valid_hook_name};

// Check if hook installation should be skipped
if !should_skip_installation() {
    let hooks_path = hooks_dir(".");
    println!("Hooks directory: {}", hooks_path.display());
}

// Validate a hook name
if is_valid_hook_name("pre-commit") {
    println!("Valid hook!");
}
```

See [API documentation](https://docs.rs/husky-rs) for more details.

*Note*: You don't need to call any functions for basic usage - just add the dependency!

## Best Practices

- Keep hooks lightweight to avoid slowing down Git operations
- Use hooks for tasks like running tests, linting code, and validating commit messages
- Non-zero exit status in a hook script will abort the Git operation

## Documentation

📖 **Complete guides for all users:**

- [Usage Guide](docs/usage.md) - Installation, configuration, and advanced usage
- [Examples](docs/examples.md) - 13 ready-to-use hook examples
- [Troubleshooting](docs/troubleshooting.md) - Solutions to common issues
- [Development](docs/development.md) - Contributing guide

## Contributing

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md) for details on how to submit pull requests, report issues, or suggest improvements.

## License

This project is licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

## Changelog

For a detailed history of changes to this project, please refer to our [CHANGELOG.md](CHANGELOG.md).

## Acknowledgments

- Inspired by [cargo-husky](https://github.com/rhysd/cargo-husky)
- Thanks to the Rust community for their amazing tools and libraries
