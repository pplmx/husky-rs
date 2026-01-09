use std::env;
use std::fs;
use std::io::Error;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

// Import test utilities from test_husky
// Since we can't directly import from another test file, we'll duplicate minimal helpers

fn create_temp_dir(prefix: &str) -> PathBuf {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let temp_dir = env::temp_dir().join(format!("{}{}", prefix, timestamp));
    fs::create_dir_all(&temp_dir).unwrap();
    temp_dir
}

struct ExampleProject {
    path: PathBuf,
}

impl ExampleProject {
    fn new(prefix: &str) -> Result<Self, Error> {
        let path = create_temp_dir(prefix);
        let project = ExampleProject { path };
        project.init()?;
        Ok(project)
    }

    fn init(&self) -> Result<(), Error> {
        Command::new("cargo")
            .args(["init", "--bin"])
            .current_dir(&self.path)
            .status()?;

        Command::new("git")
            .arg("init")
            .current_dir(&self.path)
            .status()?;

        Ok(())
    }

    fn add_husky_rs(&self) -> Result<(), Error> {
        let cargo_toml_path = self.path.join("Cargo.toml");
        let mut cargo_toml = fs::read_to_string(&cargo_toml_path)?;
        let current_crate = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

        let husky_dep = format!(r#"husky-rs = {{ path = {:?} }}"#, current_crate);

        // Find or create [dependencies] section
        if let Some(pos) = cargo_toml.find("[dependencies]") {
            // Insert after [dependencies] line
            let insert_pos = cargo_toml[pos..]
                .find('\n')
                .map(|p| p + pos + 1)
                .unwrap_or(cargo_toml.len());
            cargo_toml.insert_str(insert_pos, &format!("{}\n", husky_dep));
        } else {
            // Add new [dependencies] section
            cargo_toml.push_str(&format!("\n[dependencies]\n{}\n", husky_dep));
        }

        fs::write(&cargo_toml_path, cargo_toml)?;
        Ok(())
    }

    fn create_hook(&self, name: &str, content: &str) -> Result<(), Error> {
        let hooks_dir = self.path.join(".husky").join("hooks");
        fs::create_dir_all(&hooks_dir)?;
        fs::write(hooks_dir.join(name), content)?;
        Ok(())
    }

    fn build(&self) -> Result<(), Error> {
        let status = Command::new("cargo")
            .arg("build")
            .current_dir(&self.path)
            .status()?;

        if status.success() {
            Ok(())
        } else {
            Err(Error::new(std::io::ErrorKind::Other, "Build failed"))
        }
    }

    fn hook_was_installed(&self, name: &str) -> bool {
        self.path.join(".git").join("hooks").join(name).exists()
    }

    fn hook_contains(&self, name: &str, text: &str) -> bool {
        if let Ok(content) = fs::read_to_string(self.path.join(".git").join("hooks").join(name)) {
            content.contains(text)
        } else {
            false
        }
    }
}

impl Drop for ExampleProject {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

// Validate shell script syntax
// Validate shell script syntax (Unix only)
fn validate_shell_syntax(script: &str) -> Result<(), String> {
    #[cfg(unix)]
    {
        let temp_file = format!("/tmp/husky_validate_{}.sh", rand_suffix());

        if let Err(e) = fs::write(&temp_file, script) {
            return Err(format!("Failed to write temp file: {}", e));
        }

        let output = Command::new("sh").args(["-n", &temp_file]).output();

        let _ = fs::remove_file(&temp_file);

        match output {
            Ok(out) if out.status.success() => Ok(()),
            Ok(out) => Err(format!(
                "Shell syntax error: {}",
                String::from_utf8_lossy(&out.stderr)
            )),
            Err(e) => Err(format!("Failed to run sh: {}", e)),
        }
    }
    #[cfg(not(unix))]
    {
        // Skip shell validation on non-Unix platforms
        Ok(())
    }
}

fn rand_suffix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64
}

// Example 1: Running Tests Before Commit
#[test]
fn test_example_1_running_tests() -> Result<(), Error> {
    let project = ExampleProject::new("example-1-")?;
    project.add_husky_rs()?;

    let hook_content = r#"#!/bin/sh
echo "Running tests..."
cargo test --quiet || exit 1
"#;

    // Validate shell syntax
    assert!(
        validate_shell_syntax(hook_content).is_ok(),
        "Example 1 hook should have valid shell syntax"
    );

    project.create_hook("pre-commit", hook_content)?;
    project.build()?;

    assert!(
        project.hook_was_installed("pre-commit"),
        "Hook should be installed"
    );
    assert!(
        project.hook_contains("pre-commit", "cargo test --quiet"),
        "Hook should contain the test command"
    );

    Ok(())
}

// Example 2: Code Formatting Check
#[test]
fn test_example_2_formatting_check() -> Result<(), Error> {
    let project = ExampleProject::new("example-2-")?;
    project.add_husky_rs()?;

    let hook_content = r#"#!/bin/sh
echo "Checking code formatting..."
cargo fmt --check || {
    echo "❌ Code is not formatted. Run 'cargo fmt' and try again."
    exit 1
}
echo "✓ Code is properly formatted"
"#;

    assert!(
        validate_shell_syntax(hook_content).is_ok(),
        "Example 2 hook should have valid shell syntax"
    );

    project.create_hook("pre-commit", hook_content)?;
    project.build()?;

    assert!(project.hook_was_installed("pre-commit"));
    assert!(project.hook_contains("pre-commit", "cargo fmt --check"));

    Ok(())
}

// Example 3: Linting with Clippy
#[test]
fn test_example_3_clippy_linting() -> Result<(), Error> {
    let project = ExampleProject::new("example-3-")?;
    project.add_husky_rs()?;

    let hook_content = r#"#!/bin/sh
echo "Running clippy..."
cargo clippy --all-targets --all-features -- -D warnings || {
    echo "❌ Clippy found issues. Please fix them and try again."
    exit 1
}
echo "✓ No clippy warnings"
"#;

    assert!(
        validate_shell_syntax(hook_content).is_ok(),
        "Example 3 hook should have valid shell syntax"
    );

    project.create_hook("pre-commit", hook_content)?;
    project.build()?;

    assert!(project.hook_was_installed("pre-commit"));
    assert!(project.hook_contains("pre-commit", "cargo clippy"));

    Ok(())
}

// Example 4: Conventional Commits
#[test]
fn test_example_4_conventional_commits() -> Result<(), Error> {
    let project = ExampleProject::new("example-4-")?;
    project.add_husky_rs()?;

    let hook_content = r#"#!/bin/sh
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

    assert!(
        validate_shell_syntax(hook_content).is_ok(),
        "Example 4 hook should have valid shell syntax"
    );

    project.create_hook("commit-msg", hook_content)?;
    project.build()?;

    assert!(project.hook_was_installed("commit-msg"));
    assert!(project.hook_contains("commit-msg", "Conventional Commits"));

    Ok(())
}

// Example 6: Comprehensive Pre-push
#[test]
fn test_example_6_comprehensive_prepush() -> Result<(), Error> {
    let project = ExampleProject::new("example-6-")?;
    project.add_husky_rs()?;

    let hook_content = r#"#!/bin/sh
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

    assert!(
        validate_shell_syntax(hook_content).is_ok(),
        "Example 6 hook should have valid shell syntax"
    );

    project.create_hook("pre-push", hook_content)?;
    project.build()?;

    assert!(project.hook_was_installed("pre-push"));
    assert!(project.hook_contains("pre-push", "set -e"));
    assert!(project.hook_contains("pre-push", "cargo fmt --check"));
    assert!(project.hook_contains("pre-push", "cargo clippy"));
    assert!(project.hook_contains("pre-push", "cargo test"));

    Ok(())
}

// Test conditional execution (CI skip) example
#[test]
fn test_example_8_conditional_ci_skip() -> Result<(), Error> {
    let project = ExampleProject::new("example-8-")?;
    project.add_husky_rs()?;

    let hook_content = r#"#!/bin/sh

# Skip hook in CI environment
if [ -n "$CI" ]; then
    echo "Running in CI, skipping local pre-commit hook"
    exit 0
fi

echo "Running local pre-commit checks..."
cargo test --quiet
"#;

    assert!(
        validate_shell_syntax(hook_content).is_ok(),
        "Example 8 hook should have valid shell syntax"
    );

    project.create_hook("pre-commit", hook_content)?;
    project.build()?;

    assert!(project.hook_was_installed("pre-commit"));
    assert!(project.hook_contains("pre-commit", "if [ -n \"$CI\" ]"));

    Ok(())
}

// Test Python-based hook example
#[test]
fn test_example_13_python_hook() -> Result<(), Error> {
    let project = ExampleProject::new("example-13-")?;
    project.add_husky_rs()?;

    let hook_content = r#"#!/usr/bin/env python3
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

    // Note: We're validating that the hook is created correctly,
    // not that Python runs successfully (that would require Python to be installed)

    project.create_hook("pre-commit", hook_content)?;
    project.build()?;

    assert!(project.hook_was_installed("pre-commit"));
    assert!(project.hook_contains("pre-commit", "#!/usr/bin/env python3"));
    assert!(project.hook_contains("pre-commit", "subprocess.run"));

    Ok(())
}

// Test shell syntax validation utility itself (Unix only)
#[test]
#[cfg(unix)]
fn test_shell_syntax_validator() {
    // Valid script
    let valid = r#"#!/bin/sh
echo "Hello"
exit 0
"#;
    assert!(validate_shell_syntax(valid).is_ok());

    // Invalid script - unclosed quote
    let invalid = r#"#!/bin/sh
echo "Hello
exit 0
"#;
    assert!(validate_shell_syntax(invalid).is_err());

    // Invalid script - bad syntax
    let invalid2 = r#"#!/bin/sh
if [ test; then
    echo "bad"
"#;
    assert!(validate_shell_syntax(invalid2).is_err());
}
