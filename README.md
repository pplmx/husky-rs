# husky-rs

[![CI](https://github.com/pplmx/husky-rs/workflows/CI/badge.svg)](https://github.com/pplmx/husky-rs/actions)
[![Crates.io](https://img.shields.io/crates/v/husky-rs.svg)](https://crates.io/crates/husky-rs)
[![Documentation](https://docs.rs/husky-rs/badge.svg)](https://docs.rs/husky-rs)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](#license)

`husky-rs` is a `husky-like` Git hooks management tool for Rust projects.

## Features

- **Easy setup and configuration**
- **Automatic installation of Git hooks**
- **Support for all Git hooks**
- **Cross-platform compatibility** (Unix-like systems and Windows)

## Installation

To use `husky-rs`, add it to your `Cargo.toml` file:

```toml
[dev-dependencies]
husky-rs = "0.1.0"
```

## Usage

1. Create a `.husky/hooks` directory at your project root:

   ```sh
   mkdir -p .husky/hooks
   ```

2. Add Git hooks in the `.husky/hooks` directory. For example, a `pre-commit` hook:

   ```sh
   cat << EOF > .husky/hooks/pre-commit
   #!/bin/sh

   echo "hi, pre-commit"

   EOF

   ```

3. Install the hooks by running `cargo build` or `cargo test`:

   ```sh
   # If you've used husky-rs before, ensure a clean build
   cargo clean
   cargo build
   # or
   # cargo test
   ```

> **Note:** `cargo clean` is required when any change is made to `.husky/hooks`.

## Supported Git Hooks

`husky-rs` supports all Git hooks, such as:

- `pre-commit`
- `prepare-commit-msg`
- `commit-msg`
- `post-commit`
- `pre-push`

Simply create the corresponding file in the `.husky/hooks` directory, ensuring the file name matches the Git hook name exactly.

For a full list of available Git hooks, see the [Git documentation](https://git-scm.com/docs/githooks).

## Configuration

By default, `hooks` are installed during the `build process`. To skip hook installation, set the `CARGO_HUSKY_DONT_INSTALL_HOOKS` environment variable:

```sh
CARGO_HUSKY_DONT_INSTALL_HOOKS=1 cargo build
```

## Best Practices

- Keep your hooks lightweight to avoid slowing down Git operations.
- Use hooks for tasks such as running tests, linting code, and validating commit messages.
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
