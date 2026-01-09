use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

// Helper to create temp directory
fn create_test_dir(prefix: &str) -> PathBuf {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let temp_dir = env::temp_dir().join(format!("{}{}", prefix, timestamp));
    fs::create_dir_all(&temp_dir).unwrap();
    temp_dir
}

// Helper to run husky CLI command
fn run_husky(args: &[&str], cwd: &PathBuf) -> (String, String, bool) {
    let output = Command::new(env!("CARGO_BIN_EXE_husky"))
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("Failed to execute husky command");

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let success = output.status.success();

    (stdout, stderr, success)
}

#[test]
fn test_cli_help() {
    let dir = create_test_dir("husky-cli-help-");
    let (stdout, _, success) = run_husky(&["help"], &dir);

    assert!(success, "help command should succeed");
    assert!(stdout.contains("USAGE:"), "help should show usage");
    assert!(stdout.contains("COMMANDS:"), "help should show commands");
    assert!(stdout.contains("init"), "help should mention init");
    assert!(stdout.contains("add"), "help should mention add");

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn test_cli_version() {
    let dir = create_test_dir("husky-cli-version-");
    let (stdout, _, success) = run_husky(&["version"], &dir);

    assert!(success, "version command should succeed");
    assert!(
        stdout.contains("husky-rs"),
        "version should contain 'husky-rs'"
    );

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn test_cli_init_creates_directory() {
    let dir = create_test_dir("husky-cli-init-");
    let hooks_dir = dir.join(".husky").join("hooks");

    assert!(
        !hooks_dir.exists(),
        "hooks directory should not exist initially"
    );

    let (stdout, _, success) = run_husky(&["init"], &dir);

    assert!(success, "init command should succeed");
    assert!(hooks_dir.exists(), "hooks directory should be created");
    assert!(
        hooks_dir.is_dir(),
        "hooks path should be a directory"
    );
    assert!(
        stdout.contains("Created .husky/hooks"),
        "should confirm directory creation"
    );

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn test_cli_init_already_exists() {
    let dir = create_test_dir("husky-cli-init-exists-");
    let hooks_dir = dir.join(".husky").join("hooks");

    // Pre-create directory
    fs::create_dir_all(&hooks_dir).unwrap();

    let (stdout, _, success) = run_husky(&["init"], &dir);

    assert!(success, "init should succeed even if dir exists");
    assert!(
        stdout.contains("already exists"),
        "should indicate directory already exists"
    );

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn test_cli_add_pre_commit() {
    let dir = create_test_dir("husky-cli-add-");
    let hooks_dir = dir.join(".husky").join("hooks");
    let hook_file = hooks_dir.join("pre-commit");

    let (stdout, _, success) = run_husky(&["add", "pre-commit"], &dir);

    assert!(success, "add command should succeed");
    assert!(hook_file.exists(), "hook file should be created");

    let content = fs::read_to_string(&hook_file).unwrap();
    assert!(
        content.starts_with("#!/bin/sh"),
        "hook should have shebang"
    );
    assert!(
        content.contains("husky-rs"),
        "hook should contain husky-rs comment"
    );
    assert!(
        stdout.contains("Created hook: pre-commit"),
        "should confirm creation"
    );

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn test_cli_add_commit_msg() {
    let dir = create_test_dir("husky-cli-add-msg-");
    let hook_file = dir.join(".husky").join("hooks").join("commit-msg");

    run_husky(&["add", "commit-msg"], &dir);

    assert!(hook_file.exists(), "commit-msg hook should be created");

    let content = fs::read_to_string(&hook_file).unwrap();
    assert!(
        content.contains("commit_msg"),
        "commit-msg template should be specific"
    );

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn test_cli_add_duplicate_hook() {
    let dir = create_test_dir("husky-cli-add-dup-");

    // Create hook first time
    let (_, _, success1) = run_husky(&["add", "pre-commit"], &dir);
    assert!(success1, "first add should succeed");

    // Try to create again
    let (_, stderr, success2) = run_husky(&["add", "pre-commit"], &dir);
    assert!(!success2, "second add should fail");
    assert!(
        stderr.contains("already exists"),
        "should indicate hook already exists"
    );

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn test_cli_list_no_hooks() {
    let dir = create_test_dir("husky-cli-list-empty-");

    // Initialize but don't add any hooks
    run_husky(&["init"], &dir);

    let (stdout, _, success) = run_husky(&["list"], &dir);

    assert!(success, "list should succeed");
    assert!(
        stdout.contains("No hooks found") || stdout.contains("husky init"),
        "should indicate no hooks"
    );

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn test_cli_list_with_hooks() {
    let dir = create_test_dir("husky-cli-list-");

    // Add some hooks
    run_husky(&["add", "pre-commit"], &dir);
    run_husky(&["add", "pre-push"], &dir);

    let (stdout, _, success) = run_husky(&["list"], &dir);

    assert!(success, "list should succeed");
    assert!(stdout.contains("pre-commit"), "should list pre-commit");
    assert!(stdout.contains("pre-push"), "should list pre-push");
    assert!(stdout.contains("✓"), "should show checkmark for valid hooks");

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn test_cli_list_with_invalid_hook() {
    let dir = create_test_dir("husky-cli-list-invalid-");
    let hooks_dir = dir.join(".husky").join("hooks");
    fs::create_dir_all(&hooks_dir).unwrap();

    // Create an invalid hook name
    fs::write(hooks_dir.join("not-a-real-hook"), "#!/bin/sh\necho test").unwrap();
    // Create a valid hook
    run_husky(&["add", "pre-commit"], &dir);

    let (stdout, _, success) = run_husky(&["list"], &dir);

    assert!(success, "list should succeed");
    assert!(
        stdout.contains("not-a-real-hook"),
        "should list invalid hook"
    );
    assert!(
        stdout.contains("⚠") || stdout.contains("not a standard"),
        "should warn about invalid hook"
    );
    assert!(
        stdout.contains("pre-commit"),
        "should also list valid hook"
    );

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn test_cli_templates_have_executable_permission() {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let dir = create_test_dir("husky-cli-perms-");

        run_husky(&["add", "pre-commit"], &dir);

        let hook_file = dir.join(".husky").join("hooks").join("pre-commit");
        let metadata = fs::metadata(&hook_file).unwrap();
        let mode = metadata.permissions().mode();

        assert_eq!(
            mode & 0o111,
            0o111,
            "hook should have executable permissions"
        );

        fs::remove_dir_all(&dir).ok();
    }
}

#[test]
fn test_cli_add_missing_hook_name() {
    let dir = create_test_dir("husky-cli-add-missing-");

    let (_, stderr, success) = run_husky(&["add"], &dir);

    assert!(!success, "add without hook name should fail");
    assert!(
        stderr.contains("requires a hook name") || stderr.contains("Usage"),
        "should show error about missing hook name"
    );

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn test_cli_unknown_command() {
    let dir = create_test_dir("husky-cli-unknown-");

    let (_, stderr, success) = run_husky(&["nonexistent"], &dir);

    assert!(!success, "unknown command should fail");
    assert!(
        stderr.contains("Unknown command"),
        "should indicate unknown command"
    );

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn test_cli_pre_push_template() {
    let dir = create_test_dir("husky-cli-prepush-");

    run_husky(&["add", "pre-push"], &dir);

    let hook_file = dir.join(".husky").join("hooks").join("pre-push");
    let content = fs::read_to_string(&hook_file).unwrap();

    assert!(
        content.contains("clippy") || content.contains("checks"),
        "pre-push template should contain checks"
    );

    fs::remove_dir_all(&dir).ok();
}
