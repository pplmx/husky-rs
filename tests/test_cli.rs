//! Tests for the husky CLI tool.

mod common;

use common::create_temp_dir;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

// ============================================================================
// CLI Helper
// ============================================================================

fn run_husky(args: &[&str], cwd: &PathBuf) -> (String, String, bool) {
    let output = Command::new(env!("CARGO_BIN_EXE_husky"))
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("Failed to execute husky command");

    (
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
        output.status.success(),
    )
}

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

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_cli_version() {
    let dir = create_temp_dir("husky-cli-version-").unwrap();
    let (stdout, _, success) = run_husky(&["version"], &dir);

    assert!(success);
    assert!(stdout.contains("husky-rs"));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_cli_init_creates_directory() {
    let dir = create_temp_dir("husky-cli-init-").unwrap();
    let hooks_dir = dir.join(".husky").join("hooks");

    assert!(!hooks_dir.exists());

    let (stdout, _, success) = run_husky(&["init"], &dir);

    assert!(success);
    assert!(hooks_dir.exists() && hooks_dir.is_dir());
    assert!(stdout.contains("Created .husky/hooks"));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_cli_init_already_exists() {
    let dir = create_temp_dir("husky-cli-init-exists-").unwrap();
    let hooks_dir = dir.join(".husky").join("hooks");
    fs::create_dir_all(&hooks_dir).unwrap();

    let (stdout, _, success) = run_husky(&["init"], &dir);

    assert!(success);
    assert!(stdout.contains("already exists"));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_cli_add_pre_commit() {
    let dir = create_temp_dir("husky-cli-add-").unwrap();
    let hook_file = dir.join(".husky").join("hooks").join("pre-commit");

    let (stdout, _, success) = run_husky(&["add", "pre-commit"], &dir);

    assert!(success);
    assert!(hook_file.exists());

    let content = fs::read_to_string(&hook_file).unwrap();
    assert!(content.starts_with("#!/bin/sh"));
    assert!(content.contains("husky-rs"));
    assert!(stdout.contains("Created hook: pre-commit"));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_cli_add_commit_msg() {
    let dir = create_temp_dir("husky-cli-add-msg-").unwrap();
    let hook_file = dir.join(".husky").join("hooks").join("commit-msg");

    run_husky(&["add", "commit-msg"], &dir);

    assert!(hook_file.exists());
    let content = fs::read_to_string(&hook_file).unwrap();
    assert!(content.contains("commit_msg"));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_cli_add_duplicate_hook() {
    let dir = create_temp_dir("husky-cli-add-dup-").unwrap();

    let (_, _, success1) = run_husky(&["add", "pre-commit"], &dir);
    assert!(success1);

    let (_, stderr, success2) = run_husky(&["add", "pre-commit"], &dir);
    assert!(!success2);
    assert!(stderr.contains("already exists"));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_cli_list_no_hooks() {
    let dir = create_temp_dir("husky-cli-list-empty-").unwrap();
    run_husky(&["init"], &dir);

    let (stdout, _, success) = run_husky(&["list"], &dir);

    assert!(success);
    assert!(stdout.contains("No hooks found") || stdout.contains("husky init"));

    let _ = fs::remove_dir_all(&dir);
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

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_cli_list_with_invalid_hook() {
    let dir = create_temp_dir("husky-cli-list-invalid-").unwrap();
    let hooks_dir = dir.join(".husky").join("hooks");
    fs::create_dir_all(&hooks_dir).unwrap();
    fs::write(hooks_dir.join("not-a-real-hook"), "#!/bin/sh\necho test").unwrap();
    run_husky(&["add", "pre-commit"], &dir);

    let (stdout, _, success) = run_husky(&["list"], &dir);

    assert!(success);
    assert!(stdout.contains("not-a-real-hook"));
    assert!(stdout.contains("⚠") || stdout.contains("not a standard"));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_cli_templates_have_executable_permission() {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let dir = create_temp_dir("husky-cli-perms-").unwrap();
        run_husky(&["add", "pre-commit"], &dir);

        let hook = dir.join(".husky").join("hooks").join("pre-commit");
        let mode = fs::metadata(&hook).unwrap().permissions().mode();

        assert_eq!(mode & 0o111, 0o111, "Hook should be executable");

        let _ = fs::remove_dir_all(&dir);
    }
}

#[test]
fn test_cli_add_missing_hook_name() {
    let dir = create_temp_dir("husky-cli-add-missing-").unwrap();

    let (_, stderr, success) = run_husky(&["add"], &dir);

    assert!(!success);
    assert!(stderr.contains("requires a hook name") || stderr.contains("Usage"));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_cli_unknown_command() {
    let dir = create_temp_dir("husky-cli-unknown-").unwrap();

    let (_, stderr, success) = run_husky(&["nonexistent"], &dir);

    assert!(!success);
    assert!(stderr.contains("Unknown command"));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_cli_pre_push_template() {
    let dir = create_temp_dir("husky-cli-prepush-").unwrap();
    run_husky(&["add", "pre-push"], &dir);

    let hook = dir.join(".husky").join("hooks").join("pre-push");
    let content = fs::read_to_string(&hook).unwrap();

    assert!(content.contains("clippy") || content.contains("checks"));

    let _ = fs::remove_dir_all(&dir);
}
