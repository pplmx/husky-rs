//! Hook content and syntax tests for husky-rs.
//!
//! Tests that various hook scripts are properly validated and installed.

mod common;

use common::{validate_shell_syntax, TestProject};
use std::io::Error;

// ============================================================================
// Hook Content Tests
// ============================================================================

/// Hook running cargo test.
#[test]
fn test_hook_cargo_test() -> Result<(), Error> {
    let hook = r#"#!/bin/sh
echo "Running tests..."
cargo test --quiet || exit 1
"#;
    assert!(validate_shell_syntax(hook).is_ok());

    let project = TestProject::new("hook-test-")?;
    project.add_husky_rs("dependencies", false)?;
    project.create_hook("pre-commit", hook)?;
    project.build()?;

    project.assert_hook_installed("pre-commit");
    project.assert_hook_contains("pre-commit", "cargo test --quiet");
    Ok(())
}

/// Hook running cargo fmt.
#[test]
fn test_hook_cargo_fmt() -> Result<(), Error> {
    let hook = r#"#!/bin/sh
echo "Checking format..."
cargo fmt --check || exit 1
"#;
    assert!(validate_shell_syntax(hook).is_ok());

    let project = TestProject::new("hook-fmt-")?;
    project.add_husky_rs("dependencies", false)?;
    project.create_hook("pre-commit", hook)?;
    project.build()?;

    project.assert_hook_installed("pre-commit");
    project.assert_hook_contains("pre-commit", "cargo fmt --check");
    Ok(())
}

/// Hook running clippy.
#[test]
fn test_hook_clippy() -> Result<(), Error> {
    let hook = r#"#!/bin/sh
cargo clippy --all-targets -- -D warnings || exit 1
"#;
    assert!(validate_shell_syntax(hook).is_ok());

    let project = TestProject::new("hook-clippy-")?;
    project.add_husky_rs("dependencies", false)?;
    project.create_hook("pre-commit", hook)?;
    project.build()?;

    project.assert_hook_contains("pre-commit", "cargo clippy");
    Ok(())
}

/// Hook for conventional commits.
#[test]
fn test_hook_conventional_commit() -> Result<(), Error> {
    let hook = r#"#!/bin/sh
commit_msg_file="$1"
commit_msg=$(cat "$commit_msg_file")

if ! echo "$commit_msg" | grep -qE "^(feat|fix|docs|chore)"; then
    echo "Invalid commit message"
    exit 1
fi
"#;
    assert!(validate_shell_syntax(hook).is_ok());

    let project = TestProject::new("hook-commit-")?;
    project.add_husky_rs("dependencies", false)?;
    project.create_hook("commit-msg", hook)?;
    project.build()?;

    project.assert_hook_installed("commit-msg");
    Ok(())
}

/// Comprehensive pre-push hook.
#[test]
fn test_hook_prepush_comprehensive() -> Result<(), Error> {
    let hook = r#"#!/bin/sh
set -e
cargo fmt --check
cargo clippy -- -D warnings
cargo test
"#;
    assert!(validate_shell_syntax(hook).is_ok());

    let project = TestProject::new("hook-prepush-")?;
    project.add_husky_rs("dependencies", false)?;
    project.create_hook("pre-push", hook)?;
    project.build()?;

    project.assert_hook_installed("pre-push");
    project.assert_hook_contains("pre-push", "set -e");
    Ok(())
}

/// Hook with CI skip condition.
#[test]
fn test_hook_ci_skip() -> Result<(), Error> {
    let hook = r#"#!/bin/sh
if [ -n "$CI" ]; then
    exit 0
fi
cargo test --quiet
"#;
    assert!(validate_shell_syntax(hook).is_ok());

    let project = TestProject::new("hook-ci-skip-")?;
    project.add_husky_rs("dependencies", false)?;
    project.create_hook("pre-commit", hook)?;
    project.build()?;

    project.assert_hook_installed("pre-commit");
    Ok(())
}

/// Python-based hook (non-shell).
#[test]
fn test_hook_python() -> Result<(), Error> {
    let hook = r#"#!/usr/bin/env python3
import subprocess
import sys

if subprocess.run(["cargo", "test"]).returncode != 0:
    sys.exit(1)
"#;

    let project = TestProject::new("hook-python-")?;
    project.add_husky_rs("dependencies", false)?;
    project.create_hook("pre-commit", hook)?;
    project.build()?;

    project.assert_hook_installed("pre-commit");
    project.assert_hook_contains("pre-commit", "#!/usr/bin/env python3");
    Ok(())
}

// ============================================================================
// Shebang Verification
// ============================================================================

/// Verify shebang is properly set.
#[test]
fn test_hook_shebang() -> Result<(), Error> {
    let project = TestProject::new("hook-shebang-")?;
    project.add_husky_rs("dependencies", false)?;
    project.create_hooks()?;
    project.build()?;

    let content = project.get_hook_content("pre-commit")?;
    assert!(content.starts_with("#!/bin/sh"));
    Ok(())
}

// ============================================================================
// Shell Syntax Validation
// ============================================================================

#[test]
#[cfg(unix)]
fn test_shell_syntax_validator() {
    let valid = "#!/bin/sh\necho \"Hello\"\nexit 0\n";
    assert!(validate_shell_syntax(valid).is_ok());

    let invalid = "#!/bin/sh\necho \"Hello\nexit 0\n"; // Unclosed quote
    assert!(validate_shell_syntax(invalid).is_err());

    let invalid2 = "#!/bin/sh\nif [ test; then\n"; // Bad syntax
    assert!(validate_shell_syntax(invalid2).is_err());
}
