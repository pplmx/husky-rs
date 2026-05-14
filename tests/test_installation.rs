//! Installation tests for husky-rs.
//!
//! Tests hook installation mechanics:
//! - Dependency types (dependencies vs dev-dependencies)
//! - Path types (relative vs absolute)
//! - Build/clean cycles
//! - Environment variables
//! - Git submodules and workspaces

mod common;

use common::{
    add_husky_dependency, create_hook, create_temp_dir, get_hook_content, get_husky_rs_path,
    run_command, run_command_success, verify_hook_installed, TestProject, HOOK_TYPES,
};
use std::fs;
use std::io::Error;

// ============================================================================
// Basic Installation
// ============================================================================

/// Install with regular dependency (relative path).
#[test]
fn test_install_with_dep() -> Result<(), Error> {
    let project = TestProject::new("install-dep-")?;
    project.add_husky_rs("dependencies", false)?;
    project.create_hooks()?;
    project.build()?;

    for hook in HOOK_TYPES {
        project.assert_hook_installed(hook);
    }
    Ok(())
}

/// Install with regular dependency (absolute path).
#[test]
fn test_install_with_dep_abs_path() -> Result<(), Error> {
    let project = TestProject::new("install-dep-abs-")?;
    project.add_husky_rs("dependencies", true)?;
    project.create_hooks()?;
    project.build()?;
    project.verify_hooks(true)
}

/// Install survives cargo clean.
#[test]
fn test_install_survives_clean() -> Result<(), Error> {
    let project = TestProject::new("install-clean-")?;
    project.add_husky_rs("dependencies", false)?;
    project.create_hooks()?;
    project.build()?;
    project.clean()?;
    project.build()?;
    project.verify_hooks(true)
}

// ============================================================================
// Dev Dependencies
// ============================================================================

/// Dev dependency + cargo test installs hooks.
#[test]
fn test_install_with_dev_dep_test() -> Result<(), Error> {
    let project = TestProject::new("install-dev-test-")?;
    project.add_husky_rs("dev-dependencies", false)?;
    project.create_hooks()?;
    project.test()?;
    project.verify_hooks(true)
}

/// Dev dependency + cargo build does NOT install hooks.
#[test]
fn test_install_with_dev_dep_build_skips() -> Result<(), Error> {
    let project = TestProject::new("install-dev-build-")?;
    project.add_husky_rs("dev-dependencies", false)?;
    project.create_hooks()?;
    project.build()?;
    project.verify_hooks(false)
}

// ============================================================================
// Environment Variables
// ============================================================================

/// NO_HUSKY_HOOKS=1 skips installation.
#[test]
fn test_install_skipped_with_env_var() -> Result<(), Error> {
    let project = TestProject::new("install-skip-env-")?;
    project.add_husky_rs("dependencies", false)?;
    project.create_hooks()?;

    let output = std::process::Command::new("cargo")
        .arg("build")
        .current_dir(&project.path)
        .env("NO_HUSKY_HOOKS", "1")
        .output()?;

    assert!(output.status.success());
    project.verify_hooks(false)
}

// ============================================================================
// Edge Cases
// ============================================================================

/// Symbolic link hooks work (Unix only).
#[test]
fn test_install_symlink_hook() -> Result<(), Error> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::{symlink, PermissionsExt};

        let project = TestProject::new("install-symlink-")?;
        project.add_husky_rs("dependencies", false)?;

        let hooks_dir = project.path.join(".husky");
        fs::create_dir_all(&hooks_dir)?;

        let script = project.path.join("script.sh");
        fs::write(&script, "#!/bin/sh\necho 'symlink'\nexit 0")?;
        fs::set_permissions(&script, fs::Permissions::from_mode(0o755))?;

        symlink(&script, hooks_dir.join("pre-commit"))?;
        project.build()?;
        project.assert_hook_contains("pre-commit", "symlink");
    }
    Ok(())
}

/// No .git directory is handled gracefully.
#[test]
fn test_install_no_git_directory() -> Result<(), Error> {
    let project = TestProject::new_cargo_only("install-no-git-")?;
    project.add_husky_rs("dependencies", false)?;
    project.create_hook("pre-commit", "#!/bin/sh\necho 'test'\n")?;

    let result = project.cargo(&["build"])?;
    assert!(
        result.success || result.stderr.contains("Unable to find .git"),
        "Should handle missing .git gracefully"
    );
    Ok(())
}

/// Git config failures are handled gracefully — build should not fail.
#[test]
fn test_install_git_config_fails_gracefully() -> Result<(), Error> {
    let project = TestProject::new("install-git-fail-")?;
    project.add_husky_rs("dependencies", false)?;
    project.create_hooks()?;

    // Create a fake git script that exits non-zero, simulating git failure
    let fake_dir = project.path.join("fake-bin");
    fs::create_dir_all(&fake_dir)?;
    let git_script = fake_dir.join("git");
    fs::write(&git_script, "#!/bin/sh\nexit 1\n")?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&git_script, fs::Permissions::from_mode(0o755))?;
    }

    let path = format!(
        "{}:{}",
        fake_dir.display(),
        std::env::var("PATH").unwrap_or_default()
    );

    let output = std::process::Command::new("cargo")
        .arg("build")
        .current_dir(&project.path)
        .env("PATH", &path)
        .output()?;

    assert!(
        output.status.success(),
        "build should succeed even if git config fails"
    );

    // hooksPath was not set (fake git failed), but hook files still exist on disk
    for hook in HOOK_TYPES {
        assert!(
            project.path.join(".husky").join(hook).exists(),
            "hook file {} should still exist on disk",
            hook
        );
    }
    Ok(())
}

/// Hook changes trigger reinstall.
#[test]
fn test_install_detects_hook_changes() -> Result<(), Error> {
    let project = TestProject::new("install-change-")?;
    project.add_husky_rs("dependencies", false)?;
    project.create_hook("pre-commit", "#!/bin/sh\necho 'v1'\n")?;

    project.build()?;
    assert!(project.get_hook_content("pre-commit")?.contains("v1"));

    project.create_hook("pre-commit", "#!/bin/sh\necho 'v2'\n")?;
    project.build()?;
    assert!(project.get_hook_content("pre-commit")?.contains("v2"));
    Ok(())
}

// ============================================================================
// Git Submodules
// ============================================================================

/// Install in Git submodule (.git is a file).
#[test]
fn test_install_in_submodule() -> Result<(), Error> {
    let parent = create_temp_dir("install-sub-parent-")?;
    let submodule_name = "sub";

    // Setup parent repo
    run_command_success("git", &["init"], &parent)?;
    run_command_success("git", &["config", "user.email", "t@t.com"], &parent)?;
    run_command_success("git", &["config", "user.name", "T"], &parent)?;
    fs::write(parent.join("README.md"), "# P")?;
    run_command_success("git", &["add", "."], &parent)?;
    run_command_success("git", &["commit", "-m", "init"], &parent)?;

    // Setup bare repo for submodule
    let bare = create_temp_dir("install-sub-bare-")?;
    run_command_success("git", &["init", "--bare"], &bare)?;

    // Clone, init cargo, push
    let clone = create_temp_dir("install-sub-clone-")?;
    run_command_success("git", &["clone", bare.to_str().unwrap(), "."], &clone)?;
    run_command_success("git", &["config", "user.email", "t@t.com"], &clone)?;
    run_command_success("git", &["config", "user.name", "T"], &clone)?;
    run_command_success("cargo", &["init", "--bin"], &clone)?;
    run_command_success("git", &["add", "."], &clone)?;
    run_command_success("git", &["commit", "-m", "init"], &clone)?;
    run_command_success("git", &["push", "-u", "origin", "HEAD"], &clone)?;

    // Add submodule
    let result = run_command(
        "git",
        &["submodule", "add", bare.to_str().unwrap(), submodule_name],
        &parent,
    )?;

    if !result.success {
        return Ok(()); // Skip if submodule fails
    }

    let sub_path = parent.join(submodule_name);
    assert!(sub_path.join(".git").is_file(), ".git should be a file");

    // Add husky and build
    add_husky_dependency(&sub_path.join("Cargo.toml"), &get_husky_rs_path())?;
    create_hook(&sub_path, "pre-commit", "#!/bin/sh\necho 'sub'\n")?;

    let build = run_command("cargo", &["build"], &sub_path)?;
    assert!(build.success);
    assert!(verify_hook_installed(&sub_path, "pre-commit"));

    Ok(())
}

// ============================================================================
// Cargo Workspaces
// ============================================================================

/// Install in Cargo workspace.
#[test]
fn test_install_in_workspace() -> Result<(), Error> {
    let ws = create_temp_dir("install-ws-")?;

    run_command_success("git", &["init"], &ws)?;
    run_command_success("git", &["config", "user.email", "t@t.com"], &ws)?;
    run_command_success("git", &["config", "user.name", "T"], &ws)?;

    fs::write(
        ws.join("Cargo.toml"),
        r#"[workspace]
members = ["member"]
resolver = "2"
"#,
    )?;

    let member = ws.join("member");
    fs::create_dir_all(&member)?;
    fs::write(
        member.join("Cargo.toml"),
        format!(
            r#"[package]
name = "member"
version = "0.1.0"
edition = "2021"

[dependencies]
husky-rs = {{ path = {:?} }}
"#,
            get_husky_rs_path()
        ),
    )?;
    fs::create_dir_all(member.join("src"))?;
    fs::write(member.join("src").join("main.rs"), "fn main() {}")?;

    create_hook(&ws, "pre-commit", "#!/bin/sh\necho 'ws'\n")?;

    let build = run_command("cargo", &["build"], &ws)?;
    assert!(build.success);
    assert!(verify_hook_installed(&ws, "pre-commit"));

    let content = get_hook_content(&ws, "pre-commit")?;
    assert!(content.contains("ws"));

    Ok(())
}

// ============================================================================
// Lazy Hook Creation
// ============================================================================

/// Dev dependency: cargo test runs without .husky/, hooks created later,
/// subsequent cargo test detects and installs them.
/// This validates the cargo:rerun-if-changed fix for the user's .husky/.
#[test]
fn test_lazy_hooks_dev_dep() -> Result<(), Error> {
    let project = TestProject::new("lazy-dev-")?;
    project.add_husky_rs("dev-dependencies", false)?;

    // Step 1: cargo test without any hooks
    project.test()?;
    assert!(!verify_hook_installed(&project.path, "pre-commit"));

    // Step 2: create hooks
    project.create_hooks()?;

    // Step 3: cargo test again — should re-run build script and install hooks
    project.test()?;
    project.verify_hooks(true)
}

/// Regular dependency: cargo build runs without .husky/, hooks created later,
/// subsequent cargo build detects and installs them.
#[test]
fn test_lazy_hooks_dep() -> Result<(), Error> {
    let project = TestProject::new("lazy-dep-")?;
    project.add_husky_rs("dependencies", false)?;

    // Step 1: cargo build without any hooks
    project.build()?;
    assert!(!verify_hook_installed(&project.path, "pre-commit"));

    // Step 2: create hooks
    project.create_hooks()?;

    // Step 3: cargo build again — should re-run build script and install hooks
    project.build()?;
    project.verify_hooks(true)
}

// ============================================================================
// core.hooksPath Already Set
// ============================================================================

/// core.hooksPath is already ".husky" — no re-config attempt needed.
#[test]
fn test_install_hooks_path_already_set() -> Result<(), Error> {
    let project = TestProject::new("already-set-")?;
    project.add_husky_rs("dependencies", false)?;
    project.create_hooks()?;

    // First build sets hooksPath
    project.build()?;
    project.verify_hooks(true)?;

    // Second build — hooksPath already ".husky", should be a no-op
    project.build()?;
    project.verify_hooks(true)?;

    Ok(())
}

/// core.hooksPath is set to a different value — gets overridden to ".husky".
#[test]
fn test_install_overrides_existing_hooks_path() -> Result<(), Error> {
    let project = TestProject::new("override-")?;

    // Set a custom hooksPath before husky runs
    run_command_success(
        "git",
        &["config", "core.hooksPath", "custom-hooks"],
        &project.path,
    )?;

    project.add_husky_rs("dependencies", false)?;
    project.create_hooks()?;
    project.build()?;
    project.verify_hooks(true)
}

// ============================================================================
// Idempotent Repeated Execution
// ============================================================================

/// Repeated cargo build (3x) on an already-installed project does not break hooks.
#[test]
fn test_idempotent_repeated_build() -> Result<(), Error> {
    let project = TestProject::new("idem-build-")?;
    project.add_husky_rs("dependencies", false)?;
    project.create_hooks()?;

    // Install once
    project.build()?;
    project.verify_hooks(true)?;

    // Repeat — hooks should remain intact
    for i in 1..=3 {
        project.build()?;
        assert!(
            verify_hook_installed(&project.path, "pre-commit"),
            "hooks should still be installed after build #{}",
            i
        );
    }
    Ok(())
}

/// Repeated cargo test (3x) with dev-dependency does not break hooks.
#[test]
fn test_idempotent_repeated_test() -> Result<(), Error> {
    let project = TestProject::new("idem-test-")?;
    project.add_husky_rs("dev-dependencies", false)?;
    project.create_hooks()?;

    // Install once via cargo test
    project.test()?;
    project.verify_hooks(true)?;

    // Repeat — hooks should remain intact
    for i in 1..=3 {
        project.test()?;
        assert!(
            verify_hook_installed(&project.path, "pre-commit"),
            "hooks should still be installed after test #{}",
            i
        );
    }
    Ok(())
}

// ============================================================================
// Mixed cargo test + cargo build
// ============================================================================

/// Dev-dependency: cargo test installs hooks, cargo build does not undo them.
#[test]
fn test_mixed_dev_dep_test_then_build() -> Result<(), Error> {
    let project = TestProject::new("mixed-dev-")?;
    project.add_husky_rs("dev-dependencies", false)?;
    project.create_hooks()?;

    // cargo test installs hooks (dev-dep build script runs for test targets)
    project.test()?;
    project.verify_hooks(true)?;

    // cargo build does NOT trigger dev-dep build script, but hooksPath is a
    // git config setting — it persists across commands regardless
    project.build()?;
    project.verify_hooks(true)?;

    // cargo test again — hooks still in place
    project.test()?;
    project.verify_hooks(true)
}

/// Regular dependency: cargo build installs hooks, cargo test keeps them.
#[test]
fn test_mixed_dep_build_then_test() -> Result<(), Error> {
    let project = TestProject::new("mixed-dep-")?;
    project.add_husky_rs("dependencies", false)?;
    project.create_hooks()?;

    // cargo build installs hooks
    project.build()?;
    project.verify_hooks(true)?;

    // cargo test (regular dep build script runs for all targets)
    project.test()?;
    project.verify_hooks(true)?;

    // cargo build again
    project.build()?;
    project.verify_hooks(true)
}

// ============================================================================
// Multiple Hooks
// ============================================================================

/// Install all supported hook types simultaneously.
#[test]
fn test_install_all_hook_types() -> Result<(), Error> {
    let project = TestProject::new("install-all-hooks-")?;
    project.add_husky_rs("dependencies", false)?;

    // Create all supported hooks
    let all_hooks = [
        "pre-commit",
        "prepare-commit-msg",
        "commit-msg",
        "post-commit",
        "pre-push",
        "pre-rebase",
        "post-rewrite",
        "post-checkout",
        "post-merge",
        "pre-auto-gc",
    ];

    for hook in &all_hooks {
        project.create_hook(hook, &format!("#!/bin/sh\necho '{}'\n", hook))?;
    }

    project.build()?;

    // Verify all hooks installed
    for hook in &all_hooks {
        project.assert_hook_installed(hook);
        project.assert_hook_contains(hook, hook);
    }
    Ok(())
}

// ============================================================================
// Hook Triggering Verification
// ============================================================================

/// pre-commit hook actually triggers during `git commit`.
#[test]
fn test_hook_triggers_pre_commit() -> Result<(), Error> {
    let project = TestProject::new("trigger-pre-")?;
    project.add_husky_rs("dependencies", false)?;

    let marker = project.path.join("hook_ran");
    project.create_hook(
        "pre-commit",
        &format!("#!/bin/sh\ntouch {}\nexit 0\n", marker.display()),
    )?;
    project.build()?;

    // Add a file so there's something to commit
    fs::write(project.path.join("foo.txt"), "bar")?;
    run_command_success("git", &["add", "foo.txt"], &project.path)?;

    let output = run_command(
        "git",
        &["commit", "-m", "test: trigger pre-commit"],
        &project.path,
    )?;

    assert!(output.success, "commit should succeed: {}", output.stderr);
    assert!(
        marker.exists(),
        "pre-commit hook should have created marker file"
    );

    Ok(())
}

/// pre-commit hook that exits non-zero correctly aborts the commit.
#[test]
fn test_hook_failure_aborts_commit() -> Result<(), Error> {
    let project = TestProject::new("trigger-fail-")?;
    project.add_husky_rs("dependencies", false)?;

    // Hook outputs a message then fails — both go to stderr
    project.create_hook(
        "pre-commit",
        "#!/bin/sh\necho 'REJECTED by hook' >&2\nexit 1\n",
    )?;
    project.build()?;

    fs::write(project.path.join("foo.txt"), "bar")?;
    run_command_success("git", &["add", "foo.txt"], &project.path)?;

    let output = run_command("git", &["commit", "-m", "should fail"], &project.path)?;

    assert!(!output.success, "commit should be aborted by failing hook");
    assert!(
        output.stderr.contains("REJECTED by hook"),
        "hook error message should appear in stderr: {:?}",
        output
    );

    Ok(())
}

/// commit-msg hook triggers and receives the commit message.
#[test]
fn test_hook_triggers_commit_msg() -> Result<(), Error> {
    let project = TestProject::new("trigger-msg-")?;
    project.add_husky_rs("dependencies", false)?;

    let marker = project.path.join("msg_ran");
    // $1 is the path to the commit message file, not the message itself
    project.create_hook(
        "commit-msg",
        &format!("#!/bin/sh\ncat \"$1\" > {}\nexit 0\n", marker.display()),
    )?;
    project.build()?;

    fs::write(project.path.join("foo.txt"), "bar")?;
    run_command_success("git", &["add", "foo.txt"], &project.path)?;

    let output = run_command(
        "git",
        &["commit", "-m", "feat: test commit-msg hook"],
        &project.path,
    )?;

    assert!(output.success, "commit should succeed: {}", output.stderr);
    assert!(
        marker.exists(),
        "commit-msg hook should have created marker file"
    );

    let content = fs::read_to_string(&marker)?;
    assert!(
        content.contains("feat: test commit-msg hook"),
        "hook should have received commit message, got: {}",
        content
    );

    Ok(())
}

/// post-commit hook triggers after a successful commit.
#[test]
fn test_hook_triggers_post_commit() -> Result<(), Error> {
    let project = TestProject::new("trigger-post-")?;
    project.add_husky_rs("dependencies", false)?;

    let marker = project.path.join("post_ran");
    project.create_hook(
        "post-commit",
        &format!("#!/bin/sh\ntouch {}\nexit 0\n", marker.display()),
    )?;
    project.build()?;

    fs::write(project.path.join("foo.txt"), "bar")?;
    run_command_success("git", &["add", "foo.txt"], &project.path)?;

    let output = run_command(
        "git",
        &["commit", "-m", "test: trigger post-commit"],
        &project.path,
    )?;

    assert!(output.success, "commit should succeed: {}", output.stderr);
    assert!(
        marker.exists(),
        "post-commit hook should have created marker file"
    );

    Ok(())
}

// ============================================================================
// Git Worktrees
// ============================================================================

/// Install in Git worktree.
#[test]
fn test_install_in_worktree() -> Result<(), Error> {
    let main_repo = create_temp_dir("install-wt-main-")?;

    // Setup main repo
    run_command_success("git", &["init"], &main_repo)?;
    run_command_success("git", &["config", "user.email", "t@t.com"], &main_repo)?;
    run_command_success("git", &["config", "user.name", "T"], &main_repo)?;
    run_command_success("cargo", &["init", "--bin"], &main_repo)?;
    run_command_success("git", &["add", "."], &main_repo)?;
    run_command_success("git", &["commit", "-m", "init"], &main_repo)?;

    // Create worktree
    let worktree = create_temp_dir("install-wt-tree-")?;
    let _ = fs::remove_dir_all(&worktree); // git worktree add needs non-existent dir

    let result = run_command(
        "git",
        &["worktree", "add", worktree.to_str().unwrap(), "-b", "wt"],
        &main_repo,
    )?;

    if !result.success {
        return Ok(()); // Skip if worktree not supported
    }

    // Verify .git is a file in worktree
    let git_path = worktree.join(".git");
    assert!(git_path.is_file(), ".git should be a file in worktree");

    // Add husky and build
    add_husky_dependency(&worktree.join("Cargo.toml"), &get_husky_rs_path())?;
    create_hook(&worktree, "pre-commit", "#!/bin/sh\necho 'wt'\n")?;

    let build = run_command("cargo", &["build"], &worktree)?;
    assert!(build.success);
    assert!(verify_hook_installed(&worktree, "pre-commit"));

    // Cleanup worktree registration
    let _ = run_command(
        "git",
        &["worktree", "remove", worktree.to_str().unwrap()],
        &main_repo,
    );

    Ok(())
}
