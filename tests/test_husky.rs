use std::env;
use std::fs::{self};
use std::io::Error;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

// Creates a temporary directory with a given prefix, using the current time to ensure uniqueness
fn create_temp_dir(prefix: &str) -> Result<PathBuf, Error> {
    let time_since_epoch = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    let temp_dir = env::temp_dir().join(format!("{}{}", prefix, time_since_epoch.as_secs()));
    fs::create_dir_all(&temp_dir)?;
    Ok(temp_dir)
}

// Returns the relative path from one directory to another
fn get_relative_path(from: &Path, to: &Path) -> PathBuf {
    let from_abs = fs::canonicalize(from).unwrap();
    let to_abs = fs::canonicalize(to).unwrap();

    // Break down path into components
    let from_components: Vec<_> = from_abs.components().collect();
    let to_components: Vec<_> = to_abs.components().collect();

    // Find common prefix between both paths
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

// Struct representing a test project with a path
struct TestProject {
    path: PathBuf,
}

impl TestProject {
    // Creates a new test project in a temporary directory
    fn new(prefix: &str) -> Result<Self, Error> {
        let project = TestProject {
            path: create_temp_dir(prefix)?,
        };
        project.init()?;
        Ok(project)
    }

    // Initializes a new cargo project in the test directory
    fn init(&self) -> Result<(), Error> {
        Command::new("cargo")
            .args(["init", "--bin"])
            .current_dir(&self.path)
            .status()?;
        Ok(())
    }

    // Adds husky-rs to the Cargo.toml file, depending on whether it is a regular or dev dependency
    fn add_husky_rs_to_toml(&self, dependencies_type: &str) -> Result<(), Error> {
        let cargo_toml_path = self.path.join("Cargo.toml");
        let mut cargo_toml = fs::read_to_string(&cargo_toml_path)?;
        let current_crate_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let relative_crate_path = get_relative_path(&self.path, &current_crate_path);

        let section = format!("[{}]", dependencies_type);
        let husky_rs_dep = format!("husky-rs = {{ path = {:?} }}\n", relative_crate_path);

        // Insert husky-rs into the correct section of Cargo.toml
        if let Some(pos) = cargo_toml.find(&section) {
            let insert_pos = cargo_toml[pos..]
                .find('\n')
                .map(|p| p + pos + 1)
                .unwrap_or(cargo_toml.len());
            cargo_toml.insert_str(insert_pos, &husky_rs_dep);
        } else {
            cargo_toml.push_str(&format!("\n{}\n{}", section, husky_rs_dep));
        }

        fs::write(&cargo_toml_path, cargo_toml)?;
        Ok(())
    }

    // Creates Husky Git hooks with the given content
    fn create_hooks(&self, content: &str) -> Result<(), Error> {
        let husky_dir = self.path.join(".husky").join("hooks");
        fs::create_dir_all(&husky_dir)?;
        for hook in &["pre-commit", "commit-msg", "pre-push"] {
            let path = husky_dir.join(hook);
            fs::write(&path, content)?;
        }
        Ok(())
    }

    // Runs a cargo command (e.g., build, test, clean) in the project directory
    fn run_cargo_command(&self, command: &str) -> Result<(), Error> {
        Command::new("cargo")
            .arg(command)
            .current_dir(&self.path)
            .status()?;
        Ok(())
    }

    // Verifies the existence and content of Git hooks
    fn verify_hooks(&self, expect_hooks: bool) -> Result<(), Error> {
        let git_hooks_dir = self.path.join(".git").join("hooks");
        for hook in &["pre-commit", "commit-msg", "pre-push"] {
            let hook_path = git_hooks_dir.join(hook);
            let hook_exists = hook_path.exists();
            let hook_content = if hook_exists {
                fs::read_to_string(&hook_path)?
            } else {
                String::new()
            };

            if expect_hooks {
                assert!(hook_exists, "Hook {} was not created", hook);
                assert!(
                    hook_content.contains("This hook was set by husky-rs"),
                    "Hook {} does not contain husky-rs header",
                    hook
                );
                assert!(
                    hook_content.contains("This is a test hook"),
                    "Hook {} does not contain original content",
                    hook
                );
            } else {
                assert!(
                    !hook_exists || !hook_content.contains("This hook was set by husky-rs"),
                    "Hook {} was unexpectedly created or contains husky-rs content",
                    hook
                );
            }
        }
        Ok(())
    }
}

// Clean up the test project directory when the object is dropped
impl Drop for TestProject {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

// Test: Verify husky-rs works as a regular dependency
#[test]
fn test_husky_rs_with_dependencies() -> Result<(), Error> {
    let project = TestProject::new("husky-rs-dep-test-")?;
    project.add_husky_rs_to_toml("dependencies")?;
    project.create_hooks("#!/bin/sh\necho \"This is a test hook\"\n")?;
    project.run_cargo_command("build")?;
    project.verify_hooks(true)
}

// Test: Verify husky-rs works as a dev dependency with cargo test
#[test]
fn test_husky_rs_with_dev_dependencies_and_cargo_test() -> Result<(), Error> {
    let project = TestProject::new("husky-rs-dev-dep-test-")?;
    project.add_husky_rs_to_toml("dev-dependencies")?;
    project.create_hooks("#!/bin/sh\necho \"This is a test hook\"\n")?;
    project.run_cargo_command("test")?;
    project.verify_hooks(true)
}

// Test: Verify husky-rs works as a dev dependency with cargo build, no hooks expected
#[test]
fn test_husky_rs_with_dev_dependencies_and_cargo_build() -> Result<(), Error> {
    let project = TestProject::new("husky-rs-dev-dep-build-test-")?;
    project.add_husky_rs_to_toml("dev-dependencies")?;
    project.create_hooks("#!/bin/sh\necho \"This is a test hook\"\n")?;
    project.run_cargo_command("build")?;
    project.verify_hooks(false)
}

// Test: Verify husky-rs works correctly after a cargo clean
#[test]
fn test_husky_rs_after_cargo_clean() -> Result<(), Error> {
    let project = TestProject::new("husky-rs-clean-test-")?;
    project.add_husky_rs_to_toml("dependencies")?;
    project.create_hooks("#!/bin/sh\necho \"This is a test hook\"\n")?;
    project.run_cargo_command("build")?;
    project.run_cargo_command("clean")?;
    project.run_cargo_command("build")?;
    project.verify_hooks(true)
}
