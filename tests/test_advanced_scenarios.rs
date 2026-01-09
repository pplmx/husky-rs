//! Advanced test scenarios for husky-rs.
//!
//! This module covers complex Git and Cargo configurations:
//! - Git Submodules
//! - Cargo Workspaces

mod common;

use common::{
    add_husky_dependency, create_hook, create_temp_dir, get_hook_content, get_husky_rs_path,
    run_command, run_command_success, verify_hook_installed, TestProject,
};
use std::fs;
use std::io::Error;

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
    run_command_success("git", &["init"], &parent_dir)?;
    run_command_success(
        "git",
        &["config", "user.email", "test@test.com"],
        &parent_dir,
    )?;
    run_command_success("git", &["config", "user.name", "Test"], &parent_dir)?;

    // Create a dummy file and commit
    fs::write(parent_dir.join("README.md"), "# Parent")?;
    run_command_success("git", &["add", "."], &parent_dir)?;
    run_command_success("git", &["commit", "-m", "initial"], &parent_dir)?;

    // 2. Create a bare repo to act as the submodule source
    let submodule_source = create_temp_dir("husky-submodule-source-")?;
    run_command_success("git", &["init", "--bare"], &submodule_source)?;

    // 3. Clone and initialize the submodule source
    let submodule_init_dir = create_temp_dir("husky-submodule-init-")?;
    run_command_success(
        "git",
        &["clone", submodule_source.to_str().unwrap(), "."],
        &submodule_init_dir,
    )?;
    run_command_success(
        "git",
        &["config", "user.email", "test@test.com"],
        &submodule_init_dir,
    )?;
    run_command_success("git", &["config", "user.name", "Test"], &submodule_init_dir)?;
    run_command_success("cargo", &["init", "--bin"], &submodule_init_dir)?;
    run_command_success("git", &["add", "."], &submodule_init_dir)?;
    run_command_success("git", &["commit", "-m", "cargo init"], &submodule_init_dir)?;
    run_command_success(
        "git",
        &["push", "-u", "origin", "HEAD"],
        &submodule_init_dir,
    )?;

    // 4. Add submodule to parent
    let result = run_command(
        "git",
        &[
            "submodule",
            "add",
            submodule_source.to_str().unwrap(),
            submodule_name,
        ],
        &parent_dir,
    )?;

    if !result.success {
        let _ = fs::remove_dir_all(&parent_dir);
        let _ = fs::remove_dir_all(&submodule_source);
        let _ = fs::remove_dir_all(&submodule_init_dir);
        println!(
            "Skipping submodule test: git submodule add failed: {}",
            result.stderr
        );
        return Ok(());
    }

    let submodule_path = parent_dir.join(submodule_name);

    // 5. Verify .git is a file (gitlink)
    let git_path = submodule_path.join(".git");
    assert!(
        git_path.exists() && git_path.is_file(),
        ".git should be a file in submodule"
    );

    // 6. Add husky-rs and create hooks
    add_husky_dependency(&submodule_path.join("Cargo.toml"), &get_husky_rs_path())?;
    create_hook(
        &submodule_path,
        "pre-commit",
        "#!/bin/sh\necho 'submodule hook'\n",
    )?;

    // 7. Build
    let build_result = run_command("cargo", &["build"], &submodule_path)?;
    assert!(build_result.success, "cargo build should succeed");

    // 8. Verify hook installation
    let git_content = fs::read_to_string(&git_path)?;
    let git_dir_line = git_content.trim();
    assert!(
        git_dir_line.starts_with("gitdir: "),
        "Expected gitdir: prefix"
    );
    let relative_git_dir = git_dir_line.strip_prefix("gitdir: ").unwrap();
    let actual_git_dir = submodule_path.join(relative_git_dir);
    let hook_path = actual_git_dir.join("hooks").join("pre-commit");

    assert!(hook_path.exists(), "Hook should be installed");
    let hook_content = fs::read_to_string(&hook_path)?;
    assert!(hook_content.contains("husky-rs") && hook_content.contains("submodule hook"));

    // Cleanup
    let _ = fs::remove_dir_all(&parent_dir);
    let _ = fs::remove_dir_all(&submodule_source);
    let _ = fs::remove_dir_all(&submodule_init_dir);

    Ok(())
}

// --- Cargo Workspace Test ---

/// Test that husky-rs works correctly in a Cargo workspace setup.
#[test]
fn test_cargo_workspace_hook_installation() -> Result<(), Error> {
    let workspace_dir = create_temp_dir("husky-workspace-")?;

    // Initialize git
    run_command_success("git", &["init"], &workspace_dir)?;
    run_command_success(
        "git",
        &["config", "user.email", "test@test.com"],
        &workspace_dir,
    )?;
    run_command_success("git", &["config", "user.name", "Test"], &workspace_dir)?;

    // Create workspace Cargo.toml
    fs::write(
        workspace_dir.join("Cargo.toml"),
        r#"[workspace]
members = ["member-crate"]
resolver = "2"
"#,
    )?;

    // Create member crate
    let member_dir = workspace_dir.join("member-crate");
    fs::create_dir_all(&member_dir)?;
    fs::write(
        member_dir.join("Cargo.toml"),
        format!(
            r#"[package]
name = "member-crate"
version = "0.1.0"
edition = "2021"

[dependencies]
husky-rs = {{ path = {:?} }}
"#,
            get_husky_rs_path()
        ),
    )?;
    fs::create_dir_all(member_dir.join("src"))?;
    fs::write(member_dir.join("src").join("main.rs"), "fn main() {}")?;

    // Create hooks at workspace root
    create_hook(
        &workspace_dir,
        "pre-commit",
        "#!/bin/sh\necho 'workspace hook'\n",
    )?;

    // Build from workspace root
    let build_result = run_command("cargo", &["build"], &workspace_dir)?;
    assert!(build_result.success, "cargo build should succeed");

    // Verify hook
    assert!(verify_hook_installed(&workspace_dir, "pre-commit"));
    let content = get_hook_content(&workspace_dir, "pre-commit")?;
    assert!(content.contains("husky-rs") && content.contains("workspace hook"));

    // Test building from member
    run_command_success("cargo", &["clean"], &workspace_dir)?;
    let build_result2 = run_command("cargo", &["build"], &member_dir)?;
    assert!(build_result2.success);
    assert!(verify_hook_installed(&workspace_dir, "pre-commit"));

    let _ = fs::remove_dir_all(&workspace_dir);
    Ok(())
}

// --- Additional Edge Cases ---

/// Test that husky-rs handles a project without a .git directory gracefully.
#[test]
fn test_no_git_directory() -> Result<(), Error> {
    let project = TestProject::new_cargo_only("husky-no-git-")?;
    project.add_husky_rs("dependencies", false)?;
    project.create_hook("pre-commit", "#!/bin/sh\necho 'no git'\n")?;

    // Build should succeed with a warning
    let result = project.cargo(&["build"])?;
    assert!(
        result.success || result.stderr.contains("Unable to find .git"),
        "Should succeed or fail gracefully"
    );

    Ok(())
}

/// Test rebuilding after modifying a hook triggers reinstallation.
#[test]
fn test_hook_change_triggers_reinstall() -> Result<(), Error> {
    let project = TestProject::new("husky-hook-change-")?;
    project.add_husky_rs("dependencies", false)?;
    project.create_hook("pre-commit", "#!/bin/sh\necho 'version 1'\n")?;

    project.build()?;
    let content1 = project.get_hook_content("pre-commit")?;
    assert!(content1.contains("version 1"));

    // Modify hook
    project.create_hook("pre-commit", "#!/bin/sh\necho 'version 2'\n")?;
    project.build()?;
    let content2 = project.get_hook_content("pre-commit")?;
    assert!(content2.contains("version 2"));

    Ok(())
}
