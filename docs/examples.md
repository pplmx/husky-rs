# Examples

This guide provides practical examples for using `husky-rs` Git hooks in your Rust projects.

## Basic Examples

### Example 1: Running Tests Before Commit

Create `.husky/pre-commit`:

```sh
#!/bin/sh
echo "Running tests before commit..."
cargo test --quiet || {
    echo "❌ Tests failed! Commit aborted."
    exit 1
}
echo "✓ Tests passed"
```

### Example 2: Code Formatting Check

Create `.husky/pre-commit`:

```sh
#!/bin/sh
echo "Checking code formatting..."
cargo fmt --check || {
    echo "❌ Code is not formatted. Run 'cargo fmt' and try again."
    exit 1
}
echo "✓ Code is properly formatted"
```

### Example 3: Linting with Clippy

Create `.husky/pre-commit`:

```sh
#!/bin/sh
echo "Running clippy..."
cargo clippy --all-targets --all-features -- -D warnings || {
    echo "❌ Clippy found issues. Please fix them and try again."
    exit 1
}
echo "✓ No clippy warnings"
```

## Commit Message Validation

### Example 4: Conventional Commits

Create `.husky/commit-msg`:

```sh
#!/bin/sh
commit_msg_file="$1"
commit_msg=$(cat "$commit_msg_file")

# Check for conventional commit format
if ! echo "$commit_msg" | grep -qE "^(feat|fix|docs|style|refactor|test|chore|perf|ci|build|revert)(\(.+\))?:.+"; then
    echo "❌ Invalid commit message format!"
    echo ""
    echo "Commit message must follow Conventional Commits:"
    echo "  type(scope): description"
    echo ""
    echo "Valid types: feat, fix, docs, style, refactor, test, chore, perf, ci, build, revert"
    echo ""
    echo "Examples:"
    echo "  feat(auth): add login functionality"
    echo "  fix(api): handle null pointer exception"
    echo "  docs: update README"
    exit 1
fi
```

### Example 5: Minimum Message Length

Create `.husky/commit-msg`:

```sh
#!/bin/sh
commit_msg=$(cat "$1")
msg_length=$(echo "$commit_msg" | head -n 1 | wc -c)

if [ "$msg_length" -lt 10 ]; then
    echo "❌ Commit message is too short (minimum 10 characters)"
    exit 1
fi

echo "✓ Commit message valid"
```

## Pre-push Checks

### Example 6: Comprehensive Pre-push

Create `.husky/pre-push`:

```sh
#!/bin/sh
set -e

echo "🚀 Running pre-push checks..."

echo "1/3 Checking format..."
cargo fmt --check

echo "2/3 Running clippy..."
cargo clippy --all-targets --all-features -- -D warnings

echo "3/3 Running tests..."
cargo test

echo "✅ All checks passed! Pushing..."
```

### Example 7: Branch Protection

Create `.husky/pre-push`:

```sh
#!/bin/sh
protected_branch='main'
current_branch=$(git symbolic-ref HEAD | sed -e 's,.*/\(.*\),\1,')

if [ "$current_branch" = "$protected_branch" ]; then
    echo "❌ Direct push to $protected_branch is not allowed!"
    echo "Please create a feature branch and submit a PR."
    exit 1
fi

echo "✓ Pushing to $current_branch"
```

## Advanced Examples

### Example 8: Conditional Execution (Skip in CI)

Create `.husky/pre-commit`:

```sh
#!/bin/sh

# Skip hook in CI environment
if [ -n "$CI" ]; then
    echo "Running in CI, skipping local pre-commit hook"
    exit 0
fi

echo "Running local pre-commit checks..."
cargo test --quiet
```

### Example 9: Multi-language Project

Create `.husky/pre-commit`:

```sh
#!/bin/sh
set -e

# Check Rust code
if git diff --cached --name-only | grep -q "\.rs$"; then
    echo "Checking Rust code..."
    cargo fmt --check
    cargo clippy -- -D warnings
fi

# Check Python code (if you have Python scripts)
if git diff --cached --name-only | grep -q "\.py$"; then
    echo "Checking Python code..."
    python -m black --check .
    python -m pylint **/*.py
fi

echo "✓ All checks passed"
```

### Example 10: Prevent Secrets Commit

Create `.husky/pre-commit`:

```sh
#!/bin/sh

# Check for common secret patterns
if git diff --cached | grep -iE "(password|api_key|secret|token|private_key)\s*=\s*[\"'][^\"']+[\"']"; then
    echo "❌ Potential secret detected in commit!"
    echo "Please remove sensitive data before committing."
    exit 1
fi

# Check for common secret files
if git diff --cached --name-only | grep -E "\.(pem|key|env)$"; then
    echo "⚠️  Warning: You're about to commit a file that might contain secrets"
    echo "Files: $(git diff --cached --name-only | grep -E '\.(pem|key|env)$')"
    echo "Press Ctrl+C to abort or Enter to continue"
    read -r confirm
fi

echo "✓ No obvious secrets detected"
```

### Example 11: Auto-generate Documentation

Create `.husky/pre-commit`:

```sh
#!/bin/sh

# Generate docs if Rust code changed
if git diff --cached --name-only | grep -q "src/.*\.rs$"; then
    echo "Rust source changed, updating documentation..."
    cargo doc --no-deps

    # Optionally add generated docs to commit
    # git add target/doc
fi
```

### Example 12: Performance Benchmark Check

Create `.husky/pre-push`:

```sh
#!/bin/sh

echo "Running performance benchmarks..."

# Run benchmarks and save results
cargo bench --no-run || {
    echo "❌ Benchmarks failed to compile"
    exit 1
}

echo "✓ Benchmarks compiled successfully"
```

## Using Python Hooks

### Example 13: Python-based Hook

Create `.husky/pre-commit`:

```python
#!/usr/bin/env python3
import subprocess
import sys

def run_command(cmd):
    result = subprocess.run(cmd, shell=True, capture_output=True, text=True)
    if result.returncode != 0:
        print(f"❌ {cmd} failed")
        print(result.stderr)
        sys.exit(1)
    print(f"✓ {cmd} passed")

if __name__ == "__main__":
    print("Running Python-based pre-commit checks...")
    run_command("cargo fmt --check")
    run_command("cargo clippy -- -D warnings")
    run_command("cargo test --quiet")
    print("✅ All checks passed!")
```

## Tips and Best Practices

### Keep Hooks Fast

Hooks that take too long will slow down your workflow. Aim for:

- Pre-commit: < 10 seconds
- Commit-msg: < 1 second
- Pre-push: < 30 seconds

### Use `--quiet` or `--all` Flags

```sh
# Instead of:
cargo test

# Use:
cargo test --quiet  # Less verbose output
```

### Provide Clear Error Messages

Always tell users what went wrong and how to fix it:

```sh
cargo fmt --check || {
    echo "❌ Code formatting failed"
    echo ""
    echo "To fix: cargo fmt"
    echo ""
    exit 1
}
```

### Test Your Hooks

Before committing your hooks, test them manually:

```sh
# Test pre-commit hook
.git/hooks/pre-commit

# Test commit-msg hook
echo "test message" | .git/hooks/commit-msg
```

### Use `set -e` for Safety

Add `set -e` at the top of shell scripts to exit on any error:

```sh
#!/bin/sh
set -e  # Exit immediately if any command fails

cargo fmt --check
cargo clippy -- -D warnings
cargo test
```

### Skip Hooks When Needed

You can skip hooks with `--no-verify`:

```sh
# Skip pre-commit and commit-msg hooks
git commit --no-verify -m "WIP: temporary commit"

# Skip pre-push hook
git push --no-verify
```

## Troubleshooting

### Hook Not Running

1. Check hook file permissions (should be executable on Unix)
2. Verify hook name matches Git's supported hooks
3. Check for syntax errors: `sh -n .husky/pre-commit`

### Hook Running Twice

This shouldn't happen with `husky-rs`. If it does, check that you don't have duplicate hooks in:

- `.git/hooks/`
- `.husky/`

### Slow Hook Execution

Profile your hook to find bottlenecks:

```sh
#!/bin/sh
time cargo test --quiet
time cargo clippy -- -D warnings
```
