# Troubleshooting Guide

Common issues and solutions when using husky-rs.

## Table of Contents

- [Hooks Not Running](#hooks-not-running)
- [Build Errors](#build-errors)
- [Platform-Specific Issues](#platform-specific-issues)
- [Hook Execution Problems](#hook-execution-problems)
- [FAQ](#frequently-asked-questions)

## Hooks Not Running

### Hook files exist but don't execute

**Symptoms:**

- Hooks are in `.husky/`
- `cargo build` succeeds
- Commits/pushes proceed without running hooks

**Solutions:**

1. **Check `core.hooksPath` is configured:**

   ```sh
   git config core.hooksPath
   ```

   You should see `.husky`.

2. **Verify hook has execute permissions (Unix):**

   ```sh
   ls -l .husky/pre-commit
   # Should show: -rwxr-xr-x
   ```

   If not executable:

   ```sh
   chmod +x .husky/pre-commit
   ```

3. **Check for hooks installed by other tools:**
   Old hooks might be overwritten. Reinstall:

   ```sh
   cargo build
   ```

4. **Verify hook name is correct:**

   ```sh
   # Should be exact match to Git hook names
   pre-commit  ✅
   precommit   ❌
   pre_commit  ❌
   ```

### Hooks stopped working after update

**Solution:**
husky-rs now detects changes automatically. Just rebuild:

```sh
cargo build  # No need for cargo clean anymore!
```

If still not working, force re-run build:

```sh
cargo clean && cargo build
```

### NO_HUSKY_HOOKS environment variable

**Symptoms:**

- Hooks don't install
- Build output shows "skipping hook installation"

**Solution:**

```sh
# Check if set
env | grep NO_HUSKY_HOOKS

# Unset it
unset NO_HUSKY_HOOKS

# Then rebuild
cargo build
```

## Build Errors

### "User hook script is empty"

**Error:**

```text
Error: EmptyUserHook("/path/to/.husky/pre-commit")
```

**Cause:**
Hook file contains only whitespace or empty lines.

**Solution:**
Add actual content to your hook:

```sh
# Bad - empty or whitespace only
.husky/pre-commit:



# Good - at least some command
.husky/pre-commit:
#!/bin/sh
echo "Running hook"
```

### "Git directory not found"

**Error:**

```text
Error: GitDirNotFound("/path/to/project")
```

**Common causes:**

1. **Not in a Git repository:**

   ```sh
   git init  # Initialize git repo
   cargo build
   ```

2. **Building in wrong directory:**

   ```sh
   cd /path/to/your/project  # Go to repo root
   cargo build
   ```

3. **Git submodule issue:**
   The `.git` file might not point to valid directory. Check:

   ```sh
   cat .git  # Should show: gitdir: /path/to/.git/modules/...
   ```

### Build script fails during compilation

**Error:**

```text
error: failed to run custom build command for `husky-rs`
```

**Debug steps:**

1. **Enable verbose output:**

   ```sh
   cargo build -vv
   ```

2. **Check build script output:**
   Look for lines starting with `cargo:warning=husky-rs:`

3. **Temporarily skip hooks:**

   ```sh
   NO_HUSKY_HOOKS=1 cargo build
   ```

4. **Validate hook syntax:**

   ```sh
   sh -n .husky/pre-commit  # Check for syntax errors
   ```

## Platform-Specific Issues

### Windows

**Issue: Hooks not executable**

Windows doesn't have Unix-style execute permissions. husky-rs handles this automatically by wrapping hooks in shell scripts.

**If hooks still don't run:**

1. Ensure Git Bash or WSL is installed
2. Hooks execute via `sh` even on Windows
3. Check hook shebangs are Unix-style

**Issue: Path separators**

Use forward slashes or `Path` API in hooks:

```sh
# Good
path/to/file

# Avoid
path\to\file  # May fail in sh
```

### macOS

**Issue: Permission denied**

macOS may block execution of downloaded scripts:

**Solution:**

```sh
chmod +x .husky/*
cargo build
```

### Unix/Linux

**Issue: Hook shebang not found**

**Error:**

```text
.git/hooks/pre-commit: /usr/bin/env: 'python3': No such file or directory
```

**Solution:**
Install the required interpreter:

```sh
# For Python hooks
sudo apt-get install python3  # Debian/Ubuntu
sudo yum install python3      # RHEL/CentOS

# For Node.js hooks
curl -fsSL https://deb.nodesource.com/setup_lts.x | sudo -E bash -
sudo apt-get install -y nodejs
```

Or change shebang to installed interpreter:

```sh
#!/usr/bin/env python  # Instead of python3
```

## Hook Execution Problems

### Hook takes too long

**Symptoms:**

- Git operations slow down
- Commits take 30+ seconds

**Solutions:**

1. **Profile your hook:**

   ```sh
   time .git/hooks/pre-commit
   ```

2. **Run only changed files:**

   ```sh
   #!/bin/sh
   # Only check staged files
   git diff --cached --name-only --diff-filter=ACM | \
       grep "\.rs$" | \
       xargs -r cargo clippy --
   ```

3. **Use parallel execution:**

   ```sh
   #!/bin/sh
   cargo fmt --check &
   cargo clippy -- -D warnings &
   wait  # Wait for both to finish
   ```

4. **Cache expensive operations:**

   ```sh
   #!/bin/sh
   last_run_hash=$(cat .git/hooks/.last-test-run 2>/dev/null || echo "")
   current_hash=$(git rev-parse HEAD)

   if [ "$last_run_hash" = "$current_hash" ]; then
       echo "✓ Tests cached"
       exit 0
   fi

   if cargo test --quiet; then
       echo "$current_hash" > .git/hooks/.last-test-run
   fi
   ```

### Hook fails but provides no output

**Solution:**
Capture and display errors:

```sh
#!/bin/sh

output=$(cargo test 2>&1)
status=$?

if [ $status -ne 0 ]; then
    echo "Tests failed:"
    echo "$output"
    exit $status
fi
```

### Hook environment different from terminal

**Symptoms:**

- Hook fails in Git but commands work in terminal
- "Command not found" in hooks

**Cause:**
Git hooks run with minimal environment variables.

**Solution:**
Explicitly set PATH or use full paths:

```sh
#!/bin/sh

# Option 1: Set PATH
export PATH="/usr/local/bin:/usr/bin:/bin:$PATH"

# Option 2: Use full paths
/usr/local/bin/cargo test

# Option 3: Source your shell config
source ~/.bashrc
cargo test
```

### Hook works locally but fails in CI

**Cause:**
CI environments have different setups.

**Solutions:**

1. **Skip hooks in CI:**

   ```yaml
   # .github/workflows/ci.yml
   env:
     NO_HUSKY_HOOKS: 1
   ```

2. **Make hook CI-aware:**

   ```sh
   #!/bin/sh
   if [ -n "$CI" ]; then
       echo "Running in CI, using different checks..."
       cargo test --release
   else
       cargo test --quiet
   fi
   ```

## Frequently Asked Questions

### Do I need to run `cargo clean` after changing hooks?

**No!** As of the latest version, husky-rs automatically detects hook changes. Just run:

```sh
cargo build  # Automatically reinstalls updated hooks
```

### Can I use husky-rs with existing Git hooks?

Yes, but husky-rs will overwrite hooks it manages. To preserve existing hooks:

1. **Move existing hooks:**

   ```sh
   mv .git/hooks/pre-commit .husky/pre-commit
   ```

2. **Or call existing hooks from husky hooks:**

   ```sh
   #!/bin/sh
   # .husky/pre-commit

   # Run existing hook
   .git/hooks/pre-commit.old

   # Run new checks
   cargo test
   ```

### How do I share hooks with my team?

Commit the `.husky/` directory:

```sh
git add .husky/
git commit -m "chore: add Git hooks"
```

Team members get hooks automatically on `cargo build` or `cargo test`.

### Can I have different hooks for different environments?

Yes! Use environment variables or branch detection:

```sh
#!/bin/sh

if [ "$ENVIRONMENT" = "production" ]; then
    # Strict checks for production
    cargo test --release
    cargo clippy -- -D warnings -D clippy::all
else
    # Faster checks for development
    cargo test --lib
fi
```

### How do I debug hook installation?

Enable verbose build output:

```sh
cargo build -vv 2>&1 | grep husky
```

Look for lines like:

```text
cargo:warning=husky-rs: ✓ pre-commit
cargo:warning=husky-rs: Installed 3 Git hook(s)
```

### Why does hook work when I run it manually but not via Git?

**Common causes:**

1. **Working directory:** Hooks run from repository root
2. **Environment:** Git provides minimal environment
3. **Shell:** Git might use different shell than your terminal

**Debug:**

```sh
#!/bin/sh
echo "PWD: $PWD" >> /tmp/hook-debug.log
echo "PATH: $PATH" >> /tmp/hook-debug.log
echo "SHELL: $SHELL" >> /tmp/hook-debug.log
env >> /tmp/hook-debug.log
```

Then check `/tmp/hook-debug.log` after running Git command.

## Getting Further Help

If your issue isn't covered here:

1. **Check GitHub Issues:** [github.com/pplmx/husky-rs/issues](https://github.com/pplmx/husky-rs/issues)
2. **Open a new issue:** Include:
   - OS and version
   - Rust version (`rustc --version`)
   - husky-rs version
   - Hook file contents
   - Full error output (`cargo build -vv`)
   - Output of `ls -la .git/hooks/`
3. **Enable debug logging:**

   ```sh
   RUST_LOG=debug cargo build
   ```

## Related Documentation

- [Usage Guide](usage.md) - Complete usage documentation
- [Examples](examples.md) - Ready-to-use hook examples
- [Development Guide](development.md) - Contributing to husky-rs
