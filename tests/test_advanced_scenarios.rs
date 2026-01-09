//! Advanced test scenarios for husky-rs.
//!
//! This module covers complex Git and Cargo configurations:
//! - Git Submodules
//! - Cargo Workspaces

use std::env;
use std::fs;
use std::io::Error;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

// --- Helper Functions (similar to test_husky.rs) ---

fn create_temp_dir(prefix: &str) -> Result<PathBuf, Error> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let temp_dir = env::temp_dir().join(format!("{}{}", prefix, timestamp));
    fs::create_dir_all(&temp_dir)?;
    Ok(temp_dir)
}

fn get_husky_rs_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn run_command(cmd: &str, args: &[&str], cwd: &PathBuf) -> Result<(String, String, bool), Error> {
    let output = Command::new(cmd).args(args).current_dir(cwd).output()?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    Ok((stdout, stderr, output.status.success()))
}

fn add_husky_dependency(cargo_toml_path: &PathBuf, husky_path: &PathBuf) -> Result<(), Error> {
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

fn create_hook(project_path: &Path, hook_name: &str, content: &str) -> Result<(), Error> {
    let hooks_dir = project_path.join(".husky").join("hooks");
    fs::create_dir_all(&hooks_dir)?;
    fs::write(hooks_dir.join(hook_name), content)
}

fn verify_hook_installed(project_path: &Path, hook_name: &str) -> bool {
    project_path
        .join(".git")
        .join("hooks")
        .join(hook_name)
        .exists()
}

// --- Git Submodule Test ---

/// Test that husky-rs correctly installs hooks inside a Git submodule.
///
/// In a submodule, `.git` is a file pointing to the parent's `.git/modules/<name>` directory.
/// This test verifies our build.rs correctly finds and uses the actual git directory.
#[test]
fn test_git_submodule_hook_installation() -> Result<(), Error> {
    let parent_dir = create_temp_dir("husky-submodule-parent-")?;
    let submodule_name = "my-submodule";

    // 1. Initialize parent git repo
    run_command("git", &["init"], &parent_dir)?;
    run_command(
        "git",
        &["config", "user.email", "test@test.com"],
        &parent_dir,
    )?;
    run_command("git", &["config", "user.name", "Test"], &parent_dir)?;

    // Create a dummy file and commit so we have a valid repo
    fs::write(parent_dir.join("README.md"), "# Parent")?;
    run_command("git", &["add", "."], &parent_dir)?;
    run_command("git", &["commit", "-m", "initial"], &parent_dir)?;

    // 2. Create a bare repo to act as the submodule source
    let submodule_source = create_temp_dir("husky-submodule-source-")?;
    run_command("git", &["init", "--bare"], &submodule_source)?;

    // 3. Clone the "source" into the parent as a submodule
    // First, we need to create a non-bare clone to push to the bare repo
    let submodule_init_dir = create_temp_dir("husky-submodule-init-")?;
    run_command(
        "git",
        &["clone", submodule_source.to_str().unwrap(), "."],
        &submodule_init_dir,
    )?;
    run_command(
        "git",
        &["config", "user.email", "test@test.com"],
        &submodule_init_dir,
    )?;
    run_command("git", &["config", "user.name", "Test"], &submodule_init_dir)?;

    // Initialize a Cargo project in the submodule source
    run_command("cargo", &["init", "--bin"], &submodule_init_dir)?;
    run_command("git", &["add", "."], &submodule_init_dir)?;
    run_command("git", &["commit", "-m", "cargo init"], &submodule_init_dir)?;
    run_command("git", &["push", "origin", "master"], &submodule_init_dir)?;

    // Now add the submodule to the parent
    let (_, stderr, success) = run_command(
        "git",
        &[
            "submodule",
            "add",
            submodule_source.to_str().unwrap(),
            submodule_name,
        ],
        &parent_dir,
    )?;

    if !success {
        // Cleanup and skip if submodule add fails (e.g., due to git version issues)
        let _ = fs::remove_dir_all(&parent_dir);
        let _ = fs::remove_dir_all(&submodule_source);
        let _ = fs::remove_dir_all(&submodule_init_dir);
        println!(
            "Skipping submodule test: git submodule add failed: {}",
            stderr
        );
        return Ok(());
    }

    let submodule_path = parent_dir.join(submodule_name);

    // 4. Verify `.git` in submodule is a file (not a directory)
    let git_path = submodule_path.join(".git");
    assert!(
        git_path.exists(),
        ".git should exist in submodule: {}",
        git_path.display()
    );
    assert!(
        git_path.is_file(),
        ".git in submodule should be a FILE (gitlink), not a directory: {}",
        git_path.display()
    );

    // 5. Add husky-rs dependency and create hooks
    add_husky_dependency(&submodule_path.join("Cargo.toml"), &get_husky_rs_path())?;
    create_hook(
        &submodule_path,
        "pre-commit",
        "#!/bin/sh\necho 'submodule hook'\n",
    )?;

    // 6. Build the submodule project
    let (_, build_stderr, build_success) = run_command("cargo", &["build"], &submodule_path)?;

    if !build_success {
        println!("Build stderr: {}", build_stderr);
    }
    assert!(build_success, "cargo build should succeed in submodule");

    // 7. Verify hook was installed
    // In a submodule, hooks are in `.git/modules/<name>/hooks/` from parent's perspective,
    // or equivalently, the build script should resolve the gitlink and install correctly.
    // We check from the submodule's perspective using the actual .git directory.

    // Read the .git file to find the actual git directory
    let git_content = fs::read_to_string(&git_path)?;
    let git_dir_line = git_content.trim();
    assert!(
        git_dir_line.starts_with("gitdir: "),
        "Expected gitdir: prefix"
    );
    let relative_git_dir = git_dir_line.strip_prefix("gitdir: ").unwrap();
    let actual_git_dir = submodule_path.join(relative_git_dir);

    let hook_path = actual_git_dir.join("hooks").join("pre-commit");
    assert!(
        hook_path.exists(),
        "pre-commit hook should be installed at {}",
        hook_path.display()
    );

    let hook_content = fs::read_to_string(&hook_path)?;
    assert!(
        hook_content.contains("husky-rs"),
        "Hook should contain husky-rs marker"
    );
    assert!(
        hook_content.contains("submodule hook"),
        "Hook should contain user script"
    );

    // Cleanup
    let _ = fs::remove_dir_all(&parent_dir);
    let _ = fs::remove_dir_all(&submodule_source);
    let _ = fs::remove_dir_all(&submodule_init_dir);

    Ok(())
}

// --- Cargo Workspace Test ---

/// Test that husky-rs works correctly in a Cargo workspace setup.
///
/// This test creates a workspace with a member crate that depends on husky-rs.
/// Hooks should be installed when building the member crate.
#[test]
fn test_cargo_workspace_hook_installation() -> Result<(), Error> {
    let workspace_dir = create_temp_dir("husky-workspace-")?;

    // 1. Initialize git repo
    run_command("git", &["init"], &workspace_dir)?;
    run_command(
        "git",
        &["config", "user.email", "test@test.com"],
        &workspace_dir,
    )?;
    run_command("git", &["config", "user.name", "Test"], &workspace_dir)?;

    // 2. Create workspace Cargo.toml
    let workspace_cargo_toml = r#"[workspace]
members = ["member-crate"]
resolver = "2"
"#;
    fs::write(workspace_dir.join("Cargo.toml"), workspace_cargo_toml)?;

    // 3. Create member crate
    let member_dir = workspace_dir.join("member-crate");
    fs::create_dir_all(&member_dir)?;

    let member_cargo_toml = format!(
        r#"[package]
name = "member-crate"
version = "0.1.0"
edition = "2021"

[dependencies]
husky-rs = {{ path = {:?} }}
"#,
        get_husky_rs_path()
    );
    fs::write(member_dir.join("Cargo.toml"), member_cargo_toml)?;

    // Create src/main.rs for member
    fs::create_dir_all(member_dir.join("src"))?;
    fs::write(member_dir.join("src").join("main.rs"), "fn main() {}")?;

    // 4. Create hooks at workspace root (where .git is)
    create_hook(
        &workspace_dir,
        "pre-commit",
        "#!/bin/sh\necho 'workspace hook'\n",
    )?;

    // 5. Build from workspace root
    let (_, build_stderr, build_success) = run_command("cargo", &["build"], &workspace_dir)?;

    if !build_success {
        println!("Build stderr: {}", build_stderr);
    }
    assert!(build_success, "cargo build should succeed in workspace");

    // 6. Verify hook was installed at workspace root
    assert!(
        verify_hook_installed(&workspace_dir, "pre-commit"),
        "pre-commit hook should be installed at workspace root"
    );

    let hook_content =
        fs::read_to_string(workspace_dir.join(".git").join("hooks").join("pre-commit"))?;
    assert!(
        hook_content.contains("husky-rs"),
        "Hook should contain husky-rs marker"
    );
    assert!(
        hook_content.contains("workspace hook"),
        "Hook should contain user script"
    );

    // 7. Also test building from member directory
    // First, clean to force rebuild
    run_command("cargo", &["clean"], &workspace_dir)?;

    let (_, build_stderr2, build_success2) = run_command("cargo", &["build"], &member_dir)?;

    if !build_success2 {
        println!("Build from member stderr: {}", build_stderr2);
    }
    assert!(
        build_success2,
        "cargo build should succeed from member crate"
    );

    // Hook should still be at workspace root
    assert!(
        verify_hook_installed(&workspace_dir, "pre-commit"),
        "pre-commit hook should still be at workspace root when building from member"
    );

    // Cleanup
    let _ = fs::remove_dir_all(&workspace_dir);

    Ok(())
}

// --- Additional Edge Cases ---

/// Test that husky-rs handles a project without a .git directory gracefully.
#[test]
fn test_no_git_directory() -> Result<(), Error> {
    let project_dir = create_temp_dir("husky-no-git-")?;

    // Initialize cargo project but NOT git
    run_command("cargo", &["init", "--bin"], &project_dir)?;

    // Add husky-rs dependency
    add_husky_dependency(&project_dir.join("Cargo.toml"), &get_husky_rs_path())?;

    // Create hooks directory and hook
    create_hook(&project_dir, "pre-commit", "#!/bin/sh\necho 'no git'\n")?;

    // Build should succeed (husky-rs should gracefully skip installation)
    let (_, build_stderr, build_success) = run_command("cargo", &["build"], &project_dir)?;

    // Build should succeed but log a message about no .git directory found
    assert!(
        build_success || build_stderr.contains("Unable to find .git"),
        "Build should either succeed with a warning or fail gracefully"
    );

    // Cleanup
    let _ = fs::remove_dir_all(&project_dir);

    Ok(())
}

/// Test rebuilding after modifying a hook triggers reinstallation.
#[test]
fn test_hook_change_triggers_reinstall() -> Result<(), Error> {
    let project_dir = create_temp_dir("husky-hook-change-")?;

    // Initialize project
    run_command("git", &["init"], &project_dir)?;
    run_command("cargo", &["init", "--bin"], &project_dir)?;
    add_husky_dependency(&project_dir.join("Cargo.toml"), &get_husky_rs_path())?;

    // Create initial hook
    create_hook(&project_dir, "pre-commit", "#!/bin/sh\necho 'version 1'\n")?;

    // First build
    let (_, _, build1_success) = run_command("cargo", &["build"], &project_dir)?;
    assert!(build1_success, "First build should succeed");

    let hook_path = project_dir.join(".git").join("hooks").join("pre-commit");
    let content1 = fs::read_to_string(&hook_path)?;
    assert!(
        content1.contains("version 1"),
        "Hook should contain version 1"
    );

    // Modify hook
    create_hook(&project_dir, "pre-commit", "#!/bin/sh\necho 'version 2'\n")?;

    // Second build (should detect change and reinstall)
    let (_, _, build2_success) = run_command("cargo", &["build"], &project_dir)?;
    assert!(build2_success, "Second build should succeed");

    let content2 = fs::read_to_string(&hook_path)?;
    assert!(
        content2.contains("version 2"),
        "Hook should be updated to version 2"
    );

    // Cleanup
    let _ = fs::remove_dir_all(&project_dir);

    Ok(())
}
