//! Integration tests for husky-rs hook installation.

mod common;

use common::{TestProject, HOOK_TYPES};
use std::fs;
use std::io::Error;

// ============================================================================
// Core Installation Tests
// ============================================================================

/// Test husky-rs works as a regular dependency using relative path.
#[test]
fn test_husky_rs_with_dep_rel() -> Result<(), Error> {
    let project = TestProject::new("husky-rs-dep-rel-test-")?;
    project.add_husky_rs("dependencies", false)?;
    project.create_hooks()?;
    project.build()?;

    for hook in HOOK_TYPES {
        project.assert_hook_installed(hook);
        project.assert_hook_contains(hook, "This hook was set by husky-rs");
        project.assert_hook_contains(hook, "This is a test hook");
    }

    Ok(())
}

/// Test husky-rs works as a regular dependency using absolute path.
#[test]
fn test_husky_rs_with_dep_abs() -> Result<(), Error> {
    let project = TestProject::new("husky-rs-dep-abs-test-")?;
    project.add_husky_rs("dependencies", true)?;
    project.create_hooks()?;
    project.build()?;
    project.verify_hooks(true)
}

/// Test husky-rs works correctly after a cargo clean.
#[test]
fn test_husky_rs_with_dep_after_cargo_clean() -> Result<(), Error> {
    let project = TestProject::new("husky-rs-clean-test-")?;
    project.add_husky_rs("dependencies", false)?;
    project.create_hooks()?;
    project.build()?;
    project.clean()?;
    project.build()?;
    project.verify_hooks(true)
}

// ============================================================================
// Dev Dependency Tests
// ============================================================================

/// Test husky-rs works as a dev dependency with cargo test (relative path).
#[test]
fn test_husky_rs_with_dev_dep_rel_and_cargo_test() -> Result<(), Error> {
    let project = TestProject::new("husky-rs-dev-dep-rel-test-")?;
    project.add_husky_rs("dev-dependencies", false)?;
    project.create_hooks()?;
    project.test()?;
    project.verify_hooks(true)
}

/// Test husky-rs works as a dev dependency with cargo test (absolute path).
#[test]
fn test_husky_rs_with_dev_dep_abs_and_cargo_test() -> Result<(), Error> {
    let project = TestProject::new("husky-rs-dev-dep-abs-test-")?;
    project.add_husky_rs("dev-dependencies", true)?;
    project.create_hooks()?;
    project.test()?;
    project.verify_hooks(true)
}

/// Test husky-rs with dev dependency and cargo build (hooks should NOT be installed).
#[test]
fn test_husky_rs_with_dev_dep_and_cargo_build() -> Result<(), Error> {
    let project = TestProject::new("husky-rs-dev-dep-build-test-")?;
    project.add_husky_rs("dev-dependencies", false)?;
    project.create_hooks()?;
    project.build()?;
    project.verify_hooks(false)
}

// ============================================================================
// Shebang and Hook Content Tests
// ============================================================================

/// Test various shebang scenarios and hook content verification.
#[test]
fn test_shebang_variations() -> Result<(), Error> {
    let project = TestProject::new("husky-rs-capture-output-")?;
    project.add_husky_rs("dependencies", false)?;
    project.create_hooks()?;

    let husky_hooks_dir = project.path.join(".husky").join("hooks");
    assert!(
        husky_hooks_dir.join("pre-commit").exists(),
        "User pre-commit should exist"
    );

    let output = project.cargo(&["build"])?;
    assert!(output.success, "cargo build should succeed");

    let installed_hook = project.path.join(".git").join("hooks").join("pre-commit");
    assert!(installed_hook.exists(), "pre-commit should be installed");

    let content = fs::read_to_string(&installed_hook)?;
    assert!(content.starts_with("#!/bin/sh"));
    assert!(content.contains("This hook was set by husky-rs"));
    assert!(content.contains("echo \"This is a test hook\""));

    Ok(())
}

// ============================================================================
// Edge Cases
// ============================================================================

/// Test that empty or whitespace-only hooks cause build failure.
#[test]
fn test_empty_user_hook_script() -> Result<(), Error> {
    let project = TestProject::new("husky-rs-empty-hook-")?;
    project.add_husky_rs("dependencies", false)?;

    let husky_hooks_dir = project.path.join(".husky").join("hooks");
    fs::create_dir_all(&husky_hooks_dir)?;
    fs::write(husky_hooks_dir.join("pre-commit"), "")?;
    fs::write(husky_hooks_dir.join("pre-push"), "   \n   \t  ")?;

    let build_result = project.build();
    assert!(build_result.is_err(), "Build should fail for empty hooks");

    let git_hooks_dir = project.path.join(".git").join("hooks");
    assert!(!git_hooks_dir.join("pre-commit").exists());
    assert!(!git_hooks_dir.join("pre-push").exists());

    Ok(())
}

/// Test that symbolic link hooks work correctly (Unix only).
#[test]
fn test_symbolic_link_hook() -> Result<(), Error> {
    let project = TestProject::new("husky-rs-symlink-abs-")?;
    project.add_husky_rs("dependencies", false)?;

    let husky_hooks_dir = project.path.join(".husky").join("hooks");
    fs::create_dir_all(&husky_hooks_dir)?;

    let target_script_path = fs::canonicalize(project.path.clone())?.join("actual_script.sh");
    let target_content = "#!/bin/sh\necho \"Actual script (via symlink) is running!\"\nexit 0";
    fs::write(&target_script_path, target_content)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::{symlink, PermissionsExt};

        fs::set_permissions(&target_script_path, fs::Permissions::from_mode(0o755))?;

        let symlink_path = husky_hooks_dir.join("pre-commit");
        symlink(&target_script_path, &symlink_path)?;

        assert!(symlink_path.exists());
        assert!(fs::symlink_metadata(&symlink_path)?
            .file_type()
            .is_symlink());

        let output = project.cargo(&["build"])?;
        assert!(output.success, "cargo build should succeed");

        let installed = project.path.join(".git").join("hooks").join("pre-commit");
        assert!(installed.exists());

        let content = fs::read_to_string(&installed)?;
        assert!(content.starts_with("#!/bin/sh"));
        assert!(content.contains("husky-rs"));
        assert!(content.contains("Actual script (via symlink)"));
    }

    #[cfg(not(unix))]
    {
        println!("Skipping symlink test on non-Unix platform");
    }

    Ok(())
}

/// Test no hooks are installed if NO_HUSKY_HOOKS is set.
#[test]
fn test_no_hooks_if_env_var_set() -> Result<(), Error> {
    let project = TestProject::new("husky-rs-no-hooks-env-var-")?;
    project.add_husky_rs("dependencies", false)?;
    project.create_hooks()?;

    // Run cargo build with NO_HUSKY_HOOKS set
    let output = std::process::Command::new("cargo")
        .arg("build")
        .current_dir(&project.path)
        .env("NO_HUSKY_HOOKS", "1")
        .output()?;

    assert!(output.status.success(), "Build should succeed");
    project.verify_hooks(false)
}
