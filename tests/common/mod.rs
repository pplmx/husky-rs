//! Shared test utilities for husky-rs integration tests.
//!
//! This module provides common functionality used across all test files
//! to reduce code duplication and improve maintainability.

#![allow(dead_code)]

use std::env;
use std::fs;
use std::io::Error;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

// ============================================================================
// Constants
// ============================================================================

/// Common hook types used in tests
pub const HOOK_TYPES: &[&str] = &[
    "pre-commit",
    "prepare-commit-msg",
    "commit-msg",
    "post-commit",
    "pre-push",
];

/// Default hook template content
pub const HOOK_TEMPLATE: &str = "#!/bin/sh\necho \"This is a test hook\"\n";

// ============================================================================
// Path Utilities
// ============================================================================

/// Check if a directory is writable by attempting to create a test file.
pub fn is_writable(path: &Path) -> bool {
    let test_file = path.join(".write_test");
    match fs::File::create(&test_file) {
        Ok(_) => {
            let _ = fs::remove_file(&test_file);
            true
        }
        Err(_) => false,
    }
}

/// A temporary directory that is automatically removed when dropped.
#[derive(Debug)]
pub struct TempDir {
    path: PathBuf,
}

impl TempDir {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Deref for TempDir {
    type Target = Path;

    fn deref(&self) -> &Self::Target {
        &self.path
    }
}

impl AsRef<Path> for TempDir {
    fn as_ref(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        if self.path.exists() {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}

/// Create a temporary directory with the given prefix.
///
/// Prefers the parent directory of the current crate for better cleanup,
/// falls back to system temp directory.
pub fn create_temp_dir(prefix: &str) -> Result<TempDir, Error> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // Try parent directory of current crate first
    let current_crate_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    if let Some(parent_path) = current_crate_path.parent() {
        if is_writable(parent_path) {
            let temp_dir = parent_path.join(format!("{}{}", prefix, timestamp));
            if fs::create_dir_all(&temp_dir).is_ok() {
                return Ok(TempDir::new(temp_dir));
            }
        }
    }

    // Fallback to system temp directory
    let temp_dir = env::temp_dir().join(format!("{}{}", prefix, timestamp));
    fs::create_dir_all(&temp_dir)?;
    Ok(TempDir::new(temp_dir))
}

/// Get the path to the husky-rs crate being tested.
pub fn get_husky_rs_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Returns the relative path from one directory to another.
pub fn get_relative_path(from: &Path, to: &Path) -> PathBuf {
    let from_abs = fs::canonicalize(from).unwrap();
    let to_abs = fs::canonicalize(to).unwrap();

    let from_components: Vec<_> = from_abs.components().collect();
    let to_components: Vec<_> = to_abs.components().collect();

    let common_prefix = from_components
        .iter()
        .zip(to_components.iter())
        .take_while(|&(a, b)| a == b)
        .count();

    let parents = from_components.len() - common_prefix;
    let mut result = PathBuf::new();
    for _ in 0..parents {
        result.push("..");
    }
    for component in &to_components[common_prefix..] {
        result.push(component);
    }
    result
}

// ============================================================================
// Command Execution
// ============================================================================

/// Result of running a command.
#[derive(Debug)]
pub struct CommandOutput {
    pub stdout: String,
    pub stderr: String,
    pub success: bool,
}

/// Run a command and return its output.
pub fn run_command(cmd: &str, args: &[&str], cwd: &Path) -> Result<CommandOutput, Error> {
    let output = Command::new(cmd).args(args).current_dir(cwd).output()?;

    Ok(CommandOutput {
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        success: output.status.success(),
    })
}

/// Run a command and assert it succeeds.
pub fn run_command_success(cmd: &str, args: &[&str], cwd: &Path) -> Result<(), Error> {
    let output = run_command(cmd, args, cwd)?;
    if output.success {
        Ok(())
    } else {
        Err(Error::other(format!(
            "Command `{} {}` failed: {}",
            cmd,
            args.join(" "),
            output.stderr
        )))
    }
}

// ============================================================================
// Cargo/Husky Utilities
// ============================================================================

/// Add husky-rs as a dependency to a Cargo.toml file.
pub fn add_husky_dependency(cargo_toml_path: &Path, husky_path: &Path) -> Result<(), Error> {
    let mut content = fs::read_to_string(cargo_toml_path)?;
    let dep_line = format!(r#"husky-rs = {{ path = {:?} }}"#, husky_path);

    if let Some(pos) = content.find("[dependencies]") {
        let insert_pos = content[pos..]
            .find('\n')
            .map(|p| p + pos + 1)
            .unwrap_or(content.len());
        content.insert_str(insert_pos, &format!("{}\n", dep_line));
    } else {
        content.push_str(&format!("\n[dependencies]\n{}\n", dep_line));
    }

    fs::write(cargo_toml_path, content)
}

/// Add husky-rs to Cargo.toml with options for dependency type and path style.
pub fn add_husky_dependency_with_options(
    cargo_toml_path: &Path,
    project_path: &Path,
    dep_type: &str,
    use_abs_path: bool,
) -> Result<(), Error> {
    let mut content = fs::read_to_string(cargo_toml_path)?;
    let husky_path = if use_abs_path {
        get_husky_rs_path()
    } else {
        get_relative_path(project_path, &get_husky_rs_path())
    };

    let section = format!("[{}]", dep_type);
    let dep_line = format!("husky-rs = {{ path = {:?} }}\n", husky_path);

    if let Some(pos) = content.find(&section) {
        let insert_pos = content[pos..]
            .find('\n')
            .map(|p| p + pos + 1)
            .unwrap_or(content.len());
        content.insert_str(insert_pos, &dep_line);
    } else {
        content.push_str(&format!("\n{}\n{}", section, dep_line));
    }

    fs::write(cargo_toml_path, content)
}

// ============================================================================
// Hook Utilities
// ============================================================================

/// Create a hook file in the .husky/hooks directory.
pub fn create_hook(project_path: &Path, hook_name: &str, content: &str) -> Result<(), Error> {
    let hooks_dir = project_path.join(".husky").join("hooks");
    fs::create_dir_all(&hooks_dir)?;
    fs::write(hooks_dir.join(hook_name), content)
}

/// Create all standard hooks using the default template.
pub fn create_default_hooks(project_path: &Path) -> Result<(), Error> {
    for hook in HOOK_TYPES {
        create_hook(project_path, hook, HOOK_TEMPLATE)?;
    }
    Ok(())
}

/// Check if a hook is installed in .git/hooks.
pub fn verify_hook_installed(project_path: &Path, hook_name: &str) -> bool {
    project_path
        .join(".git")
        .join("hooks")
        .join(hook_name)
        .exists()
}

/// Get the content of an installed hook.
pub fn get_hook_content(project_path: &Path, hook_name: &str) -> Result<String, Error> {
    fs::read_to_string(project_path.join(".git").join("hooks").join(hook_name))
}

/// Assert that a hook is installed.
pub fn assert_hook_installed(project_path: &Path, hook_name: &str) {
    let hook_path = project_path.join(".git").join("hooks").join(hook_name);
    assert!(
        hook_path.exists(),
        "Hook '{}' should be installed at {}",
        hook_name,
        hook_path.display()
    );
}

/// Assert that a hook contains specific content.
pub fn assert_hook_contains(project_path: &Path, hook_name: &str, expected: &str) {
    let content = get_hook_content(project_path, hook_name)
        .unwrap_or_else(|_| panic!("Hook '{}' should exist", hook_name));
    assert!(
        content.contains(expected),
        "Hook '{}' should contain '{}'\nActual:\n{}",
        hook_name,
        expected,
        content
    );
}

// ============================================================================
// Test Project Wrapper
// ============================================================================

/// A wrapper for a temporary test project with automatic cleanup.
pub struct TestProject {
    pub path: TempDir,
}

impl TestProject {
    /// Create a new test project with cargo and git initialized.
    pub fn new(prefix: &str) -> Result<Self, Error> {
        let path = create_temp_dir(prefix)?;
        let project = TestProject { path };
        project.init()?;
        Ok(project)
    }

    /// Create a new test project with only cargo initialized (no git).
    pub fn new_cargo_only(prefix: &str) -> Result<Self, Error> {
        let path = create_temp_dir(prefix)?;
        run_command_success("cargo", &["init", "--bin"], &path)?;
        Ok(TestProject { path })
    }

    /// Initialize cargo and git in the project directory.
    fn init(&self) -> Result<(), Error> {
        run_command_success("cargo", &["init", "--bin"], &self.path)?;
        run_command_success("git", &["init"], &self.path)?;
        run_command_success(
            "git",
            &["config", "user.email", "test@test.com"],
            &self.path,
        )?;
        run_command_success("git", &["config", "user.name", "Test"], &self.path)?;
        Ok(())
    }

    /// Add husky-rs as a dependency.
    pub fn add_husky_rs(&self, dep_type: &str, use_abs_path: bool) -> Result<(), Error> {
        add_husky_dependency_with_options(
            &self.path.join("Cargo.toml"),
            &self.path,
            dep_type,
            use_abs_path,
        )
    }

    /// Create hooks using the default template.
    pub fn create_hooks(&self) -> Result<(), Error> {
        create_default_hooks(&self.path)
    }

    /// Create a specific hook with custom content.
    pub fn create_hook(&self, name: &str, content: &str) -> Result<(), Error> {
        create_hook(&self.path, name, content)
    }

    /// Run a cargo command in the project.
    pub fn cargo(&self, args: &[&str]) -> Result<CommandOutput, Error> {
        run_command("cargo", args, &self.path)
    }

    /// Run cargo build.
    pub fn build(&self) -> Result<(), Error> {
        let output = self.cargo(&["build"])?;
        if output.success {
            Ok(())
        } else {
            Err(Error::other(format!(
                "cargo build failed: {}",
                output.stderr
            )))
        }
    }

    /// Run cargo test.
    pub fn test(&self) -> Result<(), Error> {
        let output = self.cargo(&["test"])?;
        if output.success {
            Ok(())
        } else {
            Err(Error::other(format!(
                "cargo test failed: {}",
                output.stderr
            )))
        }
    }

    /// Run cargo clean.
    pub fn clean(&self) -> Result<(), Error> {
        run_command_success("cargo", &["clean"], &self.path)
    }

    /// Verify hooks are installed (or not).
    pub fn verify_hooks(&self, expect_installed: bool) -> Result<(), Error> {
        for hook in HOOK_TYPES {
            let installed = verify_hook_installed(&self.path, hook);
            if expect_installed {
                assert!(installed, "Hook {} should be installed", hook);
                assert_hook_contains(&self.path, hook, "husky-rs");
            } else {
                let content = get_hook_content(&self.path, hook).unwrap_or_default();
                assert!(
                    !installed || !content.contains("husky-rs"),
                    "Hook {} should not be installed with husky-rs content",
                    hook
                );
            }
        }
        Ok(())
    }

    /// Assert a specific hook is installed.
    pub fn assert_hook_installed(&self, name: &str) {
        assert_hook_installed(&self.path, name);
    }

    /// Assert a hook contains specific content.
    pub fn assert_hook_contains(&self, name: &str, expected: &str) {
        assert_hook_contains(&self.path, name, expected);
    }

    /// Get hook content.
    pub fn get_hook_content(&self, name: &str) -> Result<String, Error> {
        get_hook_content(&self.path, name)
    }
}

// Note: Cleanup is handled automatically by TempDir's Drop implementation

// ============================================================================
// Shell Validation (Unix only)
// ============================================================================

/// Validate shell script syntax (Unix only).
#[cfg(unix)]
pub fn validate_shell_syntax(script: &str) -> Result<(), String> {
    let temp_file = format!(
        "/tmp/husky_validate_{}.sh",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );

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

/// Validate shell script syntax (no-op on non-Unix).
#[cfg(not(unix))]
pub fn validate_shell_syntax(_script: &str) -> Result<(), String> {
    Ok(())
}
