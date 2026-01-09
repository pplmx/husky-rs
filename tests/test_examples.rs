//! Tests for example hook configurations from documentation.

mod common;

use common::{create_temp_dir, get_husky_rs_path, run_command_success, validate_shell_syntax};
use std::fs;
use std::io::Error;
use std::path::PathBuf;

// ============================================================================
// Example Project Helper
// ============================================================================

struct ExampleProject {
    path: PathBuf,
}

impl ExampleProject {
    fn new(prefix: &str) -> Result<Self, Error> {
        let path = create_temp_dir(prefix)?;
        run_command_success("cargo", &["init", "--bin"], &path)?;
        run_command_success("git", &["init"], &path)?;
        Ok(ExampleProject { path })
    }

    fn add_husky_rs(&self) -> Result<(), Error> {
        let cargo_toml = self.path.join("Cargo.toml");
        let mut content = fs::read_to_string(&cargo_toml)?;
        let dep = format!(r#"husky-rs = {{ path = {:?} }}"#, get_husky_rs_path());

        if let Some(pos) = content.find("[dependencies]") {
            let insert = content[pos..]
                .find('\n')
                .map(|p| p + pos + 1)
                .unwrap_or(content.len());
            content.insert_str(insert, &format!("{}\n", dep));
        } else {
            content.push_str(&format!("\n[dependencies]\n{}\n", dep));
        }
        fs::write(&cargo_toml, content)
    }

    fn create_hook(&self, name: &str, content: &str) -> Result<(), Error> {
        let hooks_dir = self.path.join(".husky").join("hooks");
        fs::create_dir_all(&hooks_dir)?;
        fs::write(hooks_dir.join(name), content)
    }

    fn build(&self) -> Result<(), Error> {
        run_command_success("cargo", &["build"], &self.path)
    }

    fn hook_installed(&self, name: &str) -> bool {
        self.path.join(".git").join("hooks").join(name).exists()
    }

    fn hook_contains(&self, name: &str, text: &str) -> bool {
        fs::read_to_string(self.path.join(".git").join("hooks").join(name))
            .map(|c| c.contains(text))
            .unwrap_or(false)
    }
}

impl Drop for ExampleProject {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

// ============================================================================
// Example Hook Tests
// ============================================================================

#[test]
fn test_example_1_running_tests() -> Result<(), Error> {
    let project = ExampleProject::new("example-1-")?;
    project.add_husky_rs()?;

    let hook = r#"#!/bin/sh
echo "Running tests..."
cargo test --quiet || exit 1
"#;
    assert!(validate_shell_syntax(hook).is_ok());

    project.create_hook("pre-commit", hook)?;
    project.build()?;

    assert!(project.hook_installed("pre-commit"));
    assert!(project.hook_contains("pre-commit", "cargo test --quiet"));
    Ok(())
}

#[test]
fn test_example_2_formatting_check() -> Result<(), Error> {
    let project = ExampleProject::new("example-2-")?;
    project.add_husky_rs()?;

    let hook = r#"#!/bin/sh
echo "Checking code formatting..."
cargo fmt --check || {
    echo "❌ Code is not formatted. Run 'cargo fmt' and try again."
    exit 1
}
echo "✓ Code is properly formatted"
"#;
    assert!(validate_shell_syntax(hook).is_ok());

    project.create_hook("pre-commit", hook)?;
    project.build()?;

    assert!(project.hook_installed("pre-commit"));
    assert!(project.hook_contains("pre-commit", "cargo fmt --check"));
    Ok(())
}

#[test]
fn test_example_3_clippy_linting() -> Result<(), Error> {
    let project = ExampleProject::new("example-3-")?;
    project.add_husky_rs()?;

    let hook = r#"#!/bin/sh
echo "Running clippy..."
cargo clippy --all-targets --all-features -- -D warnings || {
    echo "❌ Clippy found issues. Please fix them and try again."
    exit 1
}
echo "✓ No clippy warnings"
"#;
    assert!(validate_shell_syntax(hook).is_ok());

    project.create_hook("pre-commit", hook)?;
    project.build()?;

    assert!(project.hook_installed("pre-commit"));
    assert!(project.hook_contains("pre-commit", "cargo clippy"));
    Ok(())
}

#[test]
fn test_example_4_conventional_commits() -> Result<(), Error> {
    let project = ExampleProject::new("example-4-")?;
    project.add_husky_rs()?;

    let hook = r#"#!/bin/sh
commit_msg_file="$1"
commit_msg=$(cat "$commit_msg_file")

if ! echo "$commit_msg" | grep -qE "^(feat|fix|docs|style|refactor|test|chore|perf|ci|build|revert)(\(.+\))?:.+"; then
    echo "❌ Invalid commit message format!"
    echo ""
    echo "Commit message must follow Conventional Commits:"
    echo "  type(scope): description"
    exit 1
fi
"#;
    assert!(validate_shell_syntax(hook).is_ok());

    project.create_hook("commit-msg", hook)?;
    project.build()?;

    assert!(project.hook_installed("commit-msg"));
    assert!(project.hook_contains("commit-msg", "Conventional Commits"));
    Ok(())
}

#[test]
fn test_example_6_comprehensive_prepush() -> Result<(), Error> {
    let project = ExampleProject::new("example-6-")?;
    project.add_husky_rs()?;

    let hook = r#"#!/bin/sh
set -e

echo "🚀 Running pre-push checks..."

echo "1/3 Checking format..."
cargo fmt --check

echo "2/3 Running clippy..."
cargo clippy --all-targets --all-features -- -D warnings

echo "3/3 Running tests..."
cargo test

echo "✅ All checks passed! Pushing..."
"#;
    assert!(validate_shell_syntax(hook).is_ok());

    project.create_hook("pre-push", hook)?;
    project.build()?;

    assert!(project.hook_installed("pre-push"));
    assert!(project.hook_contains("pre-push", "set -e"));
    Ok(())
}

#[test]
fn test_example_8_conditional_ci_skip() -> Result<(), Error> {
    let project = ExampleProject::new("example-8-")?;
    project.add_husky_rs()?;

    let hook = r#"#!/bin/sh

# Skip hook in CI environment
if [ -n "$CI" ]; then
    echo "Running in CI, skipping local pre-commit hook"
    exit 0
fi

echo "Running local pre-commit checks..."
cargo test --quiet
"#;
    assert!(validate_shell_syntax(hook).is_ok());

    project.create_hook("pre-commit", hook)?;
    project.build()?;

    assert!(project.hook_installed("pre-commit"));
    Ok(())
}

#[test]
fn test_example_13_python_hook() -> Result<(), Error> {
    let project = ExampleProject::new("example-13-")?;
    project.add_husky_rs()?;

    let hook = r#"#!/usr/bin/env python3
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
"#;

    project.create_hook("pre-commit", hook)?;
    project.build()?;

    assert!(project.hook_installed("pre-commit"));
    assert!(project.hook_contains("pre-commit", "#!/usr/bin/env python3"));
    Ok(())
}

#[test]
#[cfg(unix)]
fn test_shell_syntax_validator() {
    let valid = "#!/bin/sh\necho \"Hello\"\nexit 0\n";
    assert!(validate_shell_syntax(valid).is_ok());

    let invalid = "#!/bin/sh\necho \"Hello\nexit 0\n";
    assert!(validate_shell_syntax(invalid).is_err());

    let invalid2 = "#!/bin/sh\nif [ test; then\n    echo \"bad\"\n";
    assert!(validate_shell_syntax(invalid2).is_err());
}
