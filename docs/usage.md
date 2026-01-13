# Usage Guide

Complete guide to using `husky-rs` in your Rust projects.

## Table of Contents

- [Quick Start](#quick-start)
- [Installation Methods](#installation-methods)
- [Creating Hooks](#creating-hooks)
- [Hook Configuration](#hook-configuration)
- [Environment Variables](#environment-variables)
- [Common Workflows](#common-workflows)
- [Advanced Usage](#advanced-usage)
- [CLI Tool](#cli-tool)

## Quick Start

The fastest way to get started with `husky-rs`:

```sh
# 1. Add to your project
cargo add husky-rs

# 2. Create hooks directory
mkdir -p .husky/hooks

# 3. Create your first hook
cat > .husky/hooks/pre-commit << 'EOF'
#!/bin/sh
echo "Running tests..."
cargo test --quiet
EOF

# 4. Build your project to install hooks
cargo build
```

That's it! Your hook is now installed and will run before every commit.

## Installation Methods

### As a Regular Dependency

Hooks install on both `cargo build` and `cargo test`:

```toml
[dependencies]
husky-rs = "0.2"
```

```sh
cargo build  # Installs hooks
```

### As a Dev Dependency

Hooks only install on `cargo test` (recommended for most projects):

```toml
[dev-dependencies]
husky-rs = "0.2"
```

```sh
cargo test  # Installs hooks
cargo build  # Does NOT install hooks
```

### From Git

Use the latest development version:

```sh
cargo add --git https://github.com/pplmx/husky-rs --branch main
```

## Creating Hooks

### Supported Hooks

husky-rs supports all Git hooks:

**Client-Side Hooks:**

- `pre-commit` - Before commit is created
- `prepare-commit-msg` - Edit commit message
- `commit-msg` - Validate commit message
- `post-commit` - After commit is created
- `pre-push` - Before push to remote
- `pre-rebase` - Before rebase
- `post-checkout` - After checkout
- `post-merge` - After merge
- `post-rewrite` - After amend/rebase

**Server-Side Hooks:**

- `pre-receive`, `update`, `post-receive`, `post-update`
- And 17 more (see full list in [examples](examples.md))

### Hook File Format

Hooks are shell scripts in `.husky/hooks/`:

```sh
#!/bin/sh

# Your hook logic here
cargo fmt --check || exit 1
cargo test --quiet || exit 1

echo "✓ Pre-commit checks passed"
```

**Requirements:**

- File name must match a Git hook name
- Should have a shebang (`#!/bin/sh` etc.)
- Exit with non-zero to abort Git operation
- Keep scripts fast (< 10 seconds recommended)

### Using Different Interpreters

You can use any scripting language:

**Bash:**

```bash
#!/usr/bin/env bash
set -e

cargo clippy -- -D warnings
```

**Python:**

```python
#!/usr/bin/env python3
import subprocess
import sys

result = subprocess.run(["cargo", "test"], capture_output=True)
if result.returncode != 0:
    print("Tests failed!")
    sys.exit(1)
```

**Node.js:**

```javascript
#!/usr/bin/env node
const { execSync } = require('child_process');

try {
    execSync('cargo fmt --check', { stdio: 'inherit' });
} catch (error) {
    process.exit(1);
}
```

## Hook Configuration

### Accessing Hook Arguments

Some hooks receive arguments from Git:

**commit-msg** - Receives commit message file:

```sh
#!/bin/sh
commit_msg_file="$1"
commit_msg=$(cat "$commit_msg_file")

if [ ${#commit_msg} -lt 10 ]; then
    echo "Commit message too short!"
    exit 1
fi
```

**prepare-commit-msg** - Can modify the message:

```sh
#!/bin/sh
commit_msg_file="$1"
commit_source="$2"

# Add issue number if branch name contains it
branch=$(git symbolic-ref --short HEAD)
if echo "$branch" | grep -qE "^issue-[0-9]+"; then
    issue=$(echo "$branch" | grep -oE "[0-9]+")
    echo "\nRefs #$issue" >> "$commit_msg_file"
fi
```

**pre-push** - Receives remote info:

```sh
#!/bin/sh
remote="$1"
url="$2"

echo "Pushing to: $remote ($url)"

# Run full test suite before pushing
cargo test --release
```

## Environment Variables

### NO_HUSKY_HOOKS

Skip hook installation entirely:

```sh
# Skip during CI builds
NO_HUSKY_HOOKS=1 cargo build

# Skip for specific command
NO_HUSKY_HOOKS=1 cargo test
```

**Use cases:**

- CI/CD pipelines (hooks already verified locally)
- Docker builds
- Distribution packages
- When hooks would fail in environment

### CI Detection

Check if running in CI within your hook:

```sh
#!/bin/sh

if [ -n "$CI" ]; then
    echo "Running in CI, skipping local checks"
    exit 0
fi

# Local-only checks
cargo test --quiet
```

## Common Workflows

### Updating Hooks

Just edit the file and rebuild:

```sh
vim .husky/hooks/pre-commit
cargo build  # Automatically reinstalls
```

No `cargo clean` needed!

### Temporarily Bypass Hooks

Use Git's `--no-verify` flag:

```sh
# Skip pre-commit and commit-msg hooks
git commit --no-verify -m "WIP"

# Skip pre-push hook
git push --no-verify
```

**Warning:** Only use when you understand the consequences!

### Sharing Hooks with Team

Commit `.husky/` directory to version control:

```sh
git add .husky/
git commit -m "chore: add Git hooks"
git push
```

Team members get hooks automatically on next `cargo build`/`cargo test`.

### Different Hooks for Different Branches

```sh
#!/bin/sh
# .husky/hooks/pre-push

branch=$(git symbolic-ref --short HEAD)

if [ "$branch" = "main" ]; then
    # Stricter checks for main branch
    cargo test --release
    cargo clippy -- -D warnings -D clippy::all
else
    # Faster checks for feature branches
    cargo test --quiet
fi
```

## Advanced Usage

### Conditional Hook Execution

Only run hooks for specific file types:

```sh
#!/bin/sh
# Only run if Rust files changed

if git diff --cached --name-only | grep -q "\.rs$"; then
    echo "Rust files changed, running checks..."
    cargo fmt --check
    cargo clippy -- -D warnings
else
    echo "No Rust files changed, skipping checks"
fi
```

### Performance Optimization

Cache results to speed up repeated runs:

```sh
#!/bin/sh

cache_file="/tmp/husky_test_cache_$(git rev-parse HEAD)"

if [ -f "$cache_file" ]; then
    echo "✓ Tests cached for this commit"
    exit 0
fi

if cargo test --quiet; then
    touch "$cache_file"
    echo "✓ Tests passed and cached"
else
    exit 1
fi
```

### Hook Dependencies

Run multiple checks in sequence:

```sh
#!/bin/sh
set -e  # Exit on first failure

echo "1/4 Checking format..."
cargo fmt --check

echo "2/4 Running clippy..."
cargo clippy -- -D warnings

echo "3/4 Running tests..."
cargo test --quiet

echo "4/4 Checking docs..."
cargo doc --no-deps --quiet

echo "✅ All pre-commit checks passed!"
```

### Using Rust in Hooks

You can even use Rust for hook logic:

```sh
#!/bin/sh
# Pre-compile a Rust binary for your hooks

cargo run --bin hook-validator --quiet
```

Then create `src/bin/hook-validator.rs` with your validation logic.

## CLI Tool

Install the optional CLI for convenience commands:

```sh
cargo install husky-rs --features=cli
```

### Commands

**Initialize hooks directory:**

```sh
husky init
```

**Add a new hook from template:**

```sh
husky add pre-commit
husky add commit-msg
husky add pre-push
```

**List installed hooks:**

```sh
husky list
```

**Get help:**

```sh
husky help
husky --version
```

### CLI Benefits

- Faster setup with `init` command
- Smart templates for common hooks
- Automatic validation of hook names
- Sets executable permissions automatically

## Library API (Optional)

For advanced use cases, you can use husky-rs programmatically:

```rust
use husky_rs::{hooks_dir, is_valid_hook_name, should_skip_installation};

// Check if hooks should be installed
if !should_skip_installation() {
    let hooks_path = hooks_dir(".");
    println!("Hooks directory: {}", hooks_path.display());
}

// Validate a hook name
if is_valid_hook_name("pre-commit") {
    println!("Valid hook!");
}
```

**Note:** The library API is completely optional. You don't need to call any functions for basic usage!

## Troubleshooting

For common issues and solutions, see the [Troubleshooting Guide](troubleshooting.md).

## Next Steps

- Browse [Examples](examples.md) for ready-to-use hook configurations
- Read [Development Guide](development.md) to contribute
- Check [Troubleshooting](troubleshooting.md) if you encounter issues

## Related Resources

- [Git Hooks Documentation](https://git-scm.com/docs/githooks)
- [Conventional Commits](https://www.conventionalcommits.org/)
- [cargo-husky Comparison](https://github.com/rhysd/cargo-husky) (alternative tool)
