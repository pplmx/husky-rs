//! Tests for the husky CLI tool.

mod common;

use common::{
    add_husky_dependency, create_temp_dir, get_husky_rs_path, run_command, run_command_success,
    run_husky, verify_hook_installed,
};
use std::fs;

// ============================================================================
// CLI Tests
// ============================================================================

#[test]
fn test_cli_help() {
    let dir = create_temp_dir("husky-cli-help-").unwrap();
    let (stdout, _, success) = run_husky(&["help"], &dir);

    assert!(success);
    assert!(stdout.contains("USAGE:"));
    assert!(stdout.contains("COMMANDS:"));
    assert!(stdout.contains("init") && stdout.contains("add"));
}

#[test]
fn test_cli_version() {
    let dir = create_temp_dir("husky-cli-version-").unwrap();
    let (stdout, _, success) = run_husky(&["version"], &dir);

    assert!(success);
    assert!(stdout.contains("husky-rs"));
}

#[test]
fn test_cli_init_creates_directory() {
    let dir = create_temp_dir("husky-cli-init-").unwrap();
    let hooks_dir = dir.join(".husky");

    assert!(!hooks_dir.exists());

    let (stdout, _, success) = run_husky(&["init"], &dir);

    assert!(success);
    assert!(hooks_dir.exists() && hooks_dir.is_dir());
    assert!(stdout.contains("Created .husky directory"));
}

#[test]
fn test_cli_init_already_exists() {
    let dir = create_temp_dir("husky-cli-init-exists-").unwrap();
    let hooks_dir = dir.join(".husky");
    fs::create_dir_all(&hooks_dir).unwrap();

    let (stdout, _, success) = run_husky(&["init"], &dir);

    assert!(success);
    assert!(stdout.contains("already exists"));
}

#[test]
fn test_cli_add_pre_commit() {
    let dir = create_temp_dir("husky-cli-add-").unwrap();
    let hook_file = dir.join(".husky").join("pre-commit");

    let (stdout, _, success) = run_husky(&["add", "pre-commit"], &dir);

    assert!(success);
    assert!(hook_file.exists());

    let content = fs::read_to_string(&hook_file).unwrap();
    assert!(content.starts_with("#!/bin/sh"));
    assert!(stdout.contains("Created hook: pre-commit"));
}

#[test]
fn test_cli_add_commit_msg() {
    let dir = create_temp_dir("husky-cli-add-msg-").unwrap();
    let hook_file = dir.join(".husky").join("commit-msg");

    run_husky(&["add", "commit-msg"], &dir);

    assert!(hook_file.exists());
    let content = fs::read_to_string(&hook_file).unwrap();
    assert!(content.contains("commit_msg"));
}

#[test]
fn test_cli_add_duplicate_hook() {
    let dir = create_temp_dir("husky-cli-add-dup-").unwrap();

    let (_, _, success1) = run_husky(&["add", "pre-commit"], &dir);
    assert!(success1);

    let (_, stderr, success2) = run_husky(&["add", "pre-commit"], &dir);
    assert!(!success2);
    assert!(stderr.contains("already exists"));
}

#[test]
fn test_cli_list_no_hooks() {
    let dir = create_temp_dir("husky-cli-list-empty-").unwrap();
    run_husky(&["init"], &dir);

    let (stdout, _, success) = run_husky(&["list"], &dir);

    assert!(success);
    assert!(stdout.contains("No hooks found") || stdout.contains("husky init"));
}

#[test]
fn test_cli_list_with_hooks() {
    let dir = create_temp_dir("husky-cli-list-").unwrap();
    run_husky(&["add", "pre-commit"], &dir);
    run_husky(&["add", "pre-push"], &dir);

    let (stdout, _, success) = run_husky(&["list"], &dir);

    assert!(success);
    assert!(stdout.contains("pre-commit") && stdout.contains("pre-push"));
    assert!(stdout.contains("✓"));
}

#[test]
fn test_cli_list_with_invalid_hook() {
    let dir = create_temp_dir("husky-cli-list-invalid-").unwrap();
    let hooks_dir = dir.join(".husky");
    fs::create_dir_all(&hooks_dir).unwrap();
    fs::write(hooks_dir.join("not-a-real-hook"), "#!/bin/sh\necho test").unwrap();
    run_husky(&["add", "pre-commit"], &dir);

    let (stdout, _, success) = run_husky(&["list"], &dir);

    assert!(success);
    assert!(stdout.contains("not-a-real-hook"));
    assert!(stdout.contains("⚠") || stdout.contains("not a standard"));
}

#[test]
fn test_cli_templates_have_executable_permission() {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let dir = create_temp_dir("husky-cli-perms-").unwrap();
        run_husky(&["add", "pre-commit"], &dir);

        let hook = dir.join(".husky").join("pre-commit");
        let mode = fs::metadata(&hook).unwrap().permissions().mode();

        assert_eq!(mode & 0o111, 0o111, "Hook should be executable");
    }
}

#[test]
fn test_cli_add_missing_hook_name() {
    let dir = create_temp_dir("husky-cli-add-missing-").unwrap();

    let (_, stderr, success) = run_husky(&["add"], &dir);

    assert!(!success);
    assert!(stderr.contains("requires a hook name") || stderr.contains("Usage"));
}

#[test]
fn test_cli_unknown_command() {
    let dir = create_temp_dir("husky-cli-unknown-").unwrap();

    let (_, stderr, success) = run_husky(&["nonexistent"], &dir);

    assert!(!success);
    assert!(stderr.contains("Unknown command"));
}

#[test]
fn test_cli_pre_push_template() {
    let dir = create_temp_dir("husky-cli-prepush-").unwrap();
    run_husky(&["add", "pre-push"], &dir);

    let hook = dir.join(".husky").join("pre-push");
    let content = fs::read_to_string(&hook).unwrap();

    assert!(content.contains("clippy") || content.contains("checks"));
}

#[test]
fn test_cli_uninstall() {
    let dir = create_temp_dir("husky-cli-uninstall-").unwrap();

    run_command_success("git", &["init"], &dir).unwrap();

    run_command_success("git", &["config", "core.hooksPath", ".husky"], &dir).unwrap();
    assert_eq!(
        run_command("git", &["config", "core.hooksPath"], &dir)
            .unwrap()
            .stdout
            .trim(),
        ".husky"
    );

    let (stdout, _, success) = run_husky(&["uninstall"], &dir);
    assert!(success);
    assert!(stdout.contains("Unset core.hooksPath"));

    let output = run_command("git", &["config", "core.hooksPath"], &dir).unwrap();
    assert!(!output.success);
}

// ============================================================================
// Full Workflow
// ============================================================================

/// End-to-end CLI workflow: husky init → husky add → cargo build → hooks installed.
#[test]
fn test_cli_full_workflow_init_add_build() {
    let dir = create_temp_dir("husky-cli-workflow-").unwrap();

    // Setup a git+cargo project
    run_command_success("git", &["init"], &dir).unwrap();
    run_command_success("git", &["config", "user.email", "t@t.com"], &dir).unwrap();
    run_command_success("git", &["config", "user.name", "T"], &dir).unwrap();
    run_command_success("cargo", &["init", "--bin"], &dir).unwrap();

    // Step 1: husky init
    let (stdout, _, success) = run_husky(&["init"], &dir);
    assert!(success);
    assert!(stdout.contains("Created .husky directory"));

    // Step 2: husky add pre-commit
    let (stdout, _, success) = run_husky(&["add", "pre-commit"], &dir);
    assert!(success);
    assert!(stdout.contains("Created hook: pre-commit"));
    assert!(dir.join(".husky").join("pre-commit").exists());

    // Step 3: add husky-rs as dependency
    add_husky_dependency(&dir.join("Cargo.toml"), &get_husky_rs_path()).unwrap();

    // Step 4: cargo build triggers build.rs installation
    let output = run_command("cargo", &["build"], &dir).unwrap();
    assert!(output.success);

    // Verify: hooksPath set + hook file exists
    assert!(verify_hook_installed(&dir, "pre-commit"));
}
