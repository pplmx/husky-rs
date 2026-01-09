# Contribution guidelines

First off, thank you for considering contributing to `husky-rs`.

If your contribution is not straightforward, please first discuss the change you
wish to make by creating a new issue before making the change.

## Reporting issues

Before reporting an issue on the
[issue tracker](https://github.com/pplmx/husky-rs/issues),
please check that it has not already been reported by searching for some related
keywords.

## Pull requests

Try to do one pull request per change.

### Updating the changelog

Update the changes you have made in
[CHANGELOG](CHANGELOG.md)
file under the **Unreleased** section.

Add the changes of your pull request to one of the following subsections,
depending on the types of changes defined by
[Keep a changelog](https://keepachangelog.com/en/1.0.0/):

- `Added` for new features.
- `Changed` for changes in existing functionality.
- `Deprecated` for soon-to-be removed features.
- `Removed` for now removed features.
- `Fixed` for any bug fixes.
- `Security` in case of vulnerabilities.

If the required subsection does not exist yet under **Unreleased**, create it!

## Developing

### Set up

Clone the repository and run tests:

```shell
git clone https://github.com/pplmx/husky-rs
cd husky-rs
cargo test
```

### Code Style

We follow standard Rust conventions:

```shell
# Format code
cargo fmt

# Run linter
cargo clippy --all-targets --all-features -- -D warnings

# Check all at once
make clippy  # or cargo clippy --fix --allow-dirty
```

**Guidelines:**

- Use meaningful variable names
- Add doc comments for public items
- Keep functions focused and small
- Prefer explicit over clever code

### Testing Requirements

All contributions must include appropriate tests:

**For new features:**

- Add unit tests in the same file
- Add integration tests in `tests/`
- Update or add examples in `docs/examples.md`

**For bug fixes:**

- Add regression test demonstrating the bug
- Ensure fix makes the test pass

**Running tests:**

```shell
# All tests
cargo test

# Specific test
cargo test test_name

# With verbose output
cargo test -- --nocapg

# Integration tests only
cargo test --test test_husky

# CLI tests (requires cli feature)
cargo test --features=cli --test test_cli
```

### Commit Message Format

We use [Conventional Commits](https://www.conventionalcommits.org/):

```text
<type>(<scope>): <description>

[optional body]

[optional footer]
```

**Types:**

- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `test`: Adding or updating tests
- `refactor`: Code change that neither fixes a bug nor adds a feature
- `perf`: Performance improvement
- `chore`: Changes to build process or auxiliary tools

**Examples:**

```text
feat(cli): add list command

docs: update installation instructions

fix(build): handle empty hook files correctly
test(lib): add tests for should_skip_installation
```

### Local Development Workflow

1. **Create a feature branch:**

   ```shell
   git checkout -b feat/my-new-feature
   ```

2. **Make your changes**

3. **Test thoroughly:**

   ```shell
   cargo test --all-features
   cargo clippy --all-targets --all-features
   cargo fmt --check
   ```

4. **Commit with conventional format:**

   ```shell
   git commit -m "feat(scope): add new feature"
   ```

5. **Push and create PR:**

   ```shell
   git push origin feat/my-new-feature
   ```

### Building Documentation

```shell
# Build docs
cargo doc --no-deps

# Open in browser
cargo doc --no-deps --open

# Check for broken links
cargo doc --no-deps 2>&1 | grep warning
```

### Useful Commands

See the [Makefile](../Makefile) for all available commands:

```shell
make help        # Show all commands
make build       # Build release
make test        # Run tests
make fmt         # Format code
make clippy      # Run linter
make fix         # Auto-fix issues
make doc         # Generate docs
make clean       # Clean build artifacts
```
