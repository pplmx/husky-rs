# husky-rs

[![CI](https://github.com/pplmx/husky-rs/workflows/CI/badge.svg)](https://github.com/pplmx/husky-rs/actions)
[![Crates.io](https://img.shields.io/crates/v/husky-rs.svg)](https://crates.io/crates/husky-rs)
[![Documentation](https://docs.rs/husky-rs/badge.svg)](https://docs.rs/husky-rs)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](README.md#license)

`husky-rs` is `husky-like` hooks tool for Rust projects.

## Features

- Easy setup and configuration
- Automatic installation of Git hooks
- Support for all Git hooks
- Cross-platform compatibility (Unix-like systems and Windows)

## Installation

Add `husky-rs` to your `Cargo.toml`:

```toml
[dependencies]
husky-rs = "0.0.1"
```

## Usage

1. After adding husky-rs to your project, create a `.husky/hooks` directory in your project root:

   ```sh
   mkdir -p .husky/hooks
   ```

2. Create your Git hooks in the `.husky/hooks` directory. For example, a pre-commit hook:

   ```sh
   echo '#!/bin/sh
   echo "Running pre-commit hook"
   cargo test' > .husky/hooks/pre-commit
   ```

3. Make sure your hook files are executable:

   ```sh
   chmod +x .husky/hooks/pre-commit
   ```

4. husky-rs will automatically install these hooks into your Git repository when you build your project.

## Supported Hooks

husky-rs supports all Git hooks. This includes, but is not limited to:

- pre-commit
- prepare-commit-msg
- commit-msg
- post-commit
- pre-push

You can add any of these hooks by creating the corresponding file in the `.husky/hooks` directory. The file name should match the hook name exactly.

For a complete list and description of available Git hooks, please refer to the [Git documentation on hooks](https://git-scm.com/docs/githooks).

## Configuration

By default, husky-rs will install hooks during the build process. If you want to skip hook installation, you can set the `CARGO_HUSKY_DONT_INSTALL_HOOKS` environment variable:

```sh
CARGO_HUSKY_DONT_INSTALL_HOOKS=1 cargo build
```

## Best Practices

- Keep your hooks script as lightweight as possible to avoid slowing down Git operations.
- Use hooks for tasks like running tests, linting code, checking commit message format, etc.
- If a hook script exits with a non-zero status, Git will abort the operation. Use this to enforce quality checks.

## Development

For information on setting up the development environment, running tests, and contributing to the project, please refer to our [Development Guide](docs/development.md).

## Troubleshooting

If you encounter any issues while using `husky-rs`, please check our [Troubleshooting Guide](docs/troubleshooting.md) for common problems and their solutions. If you can't find a solution to your problem, please [open an issue](https://github.com/pplmx/husky-rs/issues) on our GitHub repository.

## Contributing

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md) for details on how to submit pull requests, report issues, or suggest improvements.

## License

This project is licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Changelog

For a detailed history of changes to this project, please refer to our [CHANGELOG.md](CHANGELOG.md).

## Acknowledgments

- Inspired by [cargo-husky](https://github.com/rhysd/cargo-husky)
- Thanks to the Rust community for their amazing tools and libraries
