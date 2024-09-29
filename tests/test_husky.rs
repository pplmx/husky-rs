use assert_cmd::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::Builder;

fn get_relative_path(from: &Path, to: &Path) -> PathBuf {
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

#[test]
fn test_husky_rs_integration() -> Result<(), Box<dyn std::error::Error>> {
    // Create a temporary directory for our test project
    let temp_dir = Builder::new().prefix("husky-rs-test").tempdir()?;
    let project_path = temp_dir.path();
    println!("Project path: {}", project_path.display());

    // Get the path of the current crate
    let current_crate_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    // Get the relative path from the temp project to the current crate
    let relative_crate_path = get_relative_path(project_path, &current_crate_path);

    // Initialize a new Rust project
    Command::new("cargo")
        .args(["init", "--bin"])
        .current_dir(project_path)
        .assert()
        .success();

    // Modify Cargo.toml to include husky-rs
    let cargo_toml_path = project_path.join("Cargo.toml");
    let mut cargo_toml = fs::read_to_string(&cargo_toml_path)?;
    cargo_toml.push_str(&format!(
        "husky-rs = {{ path = {:?} }}\n",
        relative_crate_path
    ));
    fs::write(&cargo_toml_path, cargo_toml)?;

    // Create .husky directory and hooks
    let husky_dir = project_path.join(".husky").join("hooks");
    fs::create_dir_all(&husky_dir)?;
    for hook in &["pre-commit", "commit-msg", "pre-push"] {
        fs::write(husky_dir.join(hook), "#!/bin/sh\necho This is a test hook")?;
    }

    // Run cargo build
    Command::new("cargo")
        .arg("build")
        .current_dir(project_path)
        .assert()
        .success();

    // Check if hooks were created
    let git_hooks_dir = project_path.join(".git").join("hooks");
    for hook in &["pre-commit", "commit-msg", "pre-push"] {
        let hook_path = git_hooks_dir.join(hook);
        assert!(hook_path.exists(), "Hook {} was not created", hook);

        let hook_content = fs::read_to_string(&hook_path)?;
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
    }

    Ok(())
}
