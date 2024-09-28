# husky-rs

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

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- Inspired by [cargo-husky](https://github.com/rhysd/cargo-husky)
- Thanks to the Rust community for their amazing tools and libraries
