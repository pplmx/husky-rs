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
    add_husky_dependency, create_hook, create_temp_dir, get_hook_content, get_husky_rs_path, run_command,
    run_command_success, verify_hook_installed, TestProject, HOOK_TYPES,
};
use std::env;
use std::fs;
use std::io::Error;

fn path_with_prepend(directory: &std::path::Path) -> Result<std::ffi::OsString, Error> {
    let current_path = env::var_os("PATH").unwrap_or_default();
    env::join_paths(std::iter::once(directory.to_path_buf()).chain(env::split_paths(&current_path)))
        .map_err(Error::other)
}

fn create_fake_prek(directory: &std::path::Path, exit_code: i32) -> Result<(), Error> {
    fs::create_dir_all(directory)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let executable = directory.join("prek");
        fs::write(
            &executable,
            format!(
                "#!/bin/sh\nprintf '%s\\n' \"$PWD\" \"$@\" >> \"$PREK_INVOCATION_LOG\"\n\
                 [ \"$1\" = \"validate-config\" ] && exit 0\nexit {exit_code}\n"
            ),
        )?;
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o755))?;
    }

    #[cfg(windows)]
    {
        let source = directory.join("fake_prek.rs");
        fs::write(
            &source,
            format!(
                r#"use std::env;
use std::fs::OpenOptions;
use std::io::Write;

fn main() {{
    let args: Vec<_> = env::args().skip(1).collect();
    let mut lines = vec![env::current_dir().unwrap().display().to_string()];
    lines.extend(args.iter().cloned());
    let mut log = OpenOptions::new()
        .create(true)
        .append(true)
        .open(env::var_os("PREK_INVOCATION_LOG").unwrap())
        .unwrap();
    writeln!(log, "{{}}", lines.join("\n")).unwrap();
    if args.first().map(String::as_str) == Some("validate-config") {{
        std::process::exit(0);
    }}
    std::process::exit({exit_code});
}}
"#
            ),
        )?;
        run_command_success(
            "rustc",
            &[
                source.to_str().unwrap(),
                "-o",
                directory.join("prek.exe").to_str().unwrap(),
            ],
            directory,
        )?;
    }

    Ok(())
}

fn create_failing_git(directory: &std::path::Path) -> Result<(), Error> {
    fs::create_dir_all(directory)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let executable = directory.join("git");
        fs::write(&executable, "#!/bin/sh\nexit 1\n")?;
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o755))?;
    }

    #[cfg(windows)]
    {
        let source = directory.join("fake_git.rs");
        fs::write(&source, "fn main() { std::process::exit(1); }\n")?;
        run_command_success(
            "rustc",
            &[
                source.to_str().unwrap(),
                "-o",
                directory.join("git.exe").to_str().unwrap(),
            ],
            directory,
        )?;
    }

    Ok(())
}

#[cfg(unix)]
fn write_executable(path: &std::path::Path, content: &str) -> Result<(), Error> {
    use std::os::unix::fs::PermissionsExt;

    fs::write(path, content)?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o755))
}

fn real_prek_e2e_enabled() -> bool {
    env::var_os("PREK_E2E").is_some()
}

#[cfg(unix)]
fn path_without_prek(project_path: &std::path::Path) -> Result<(std::ffi::OsString, std::path::PathBuf), Error> {
    use std::os::unix::fs::symlink;

    let current_path = env::var_os("PATH").unwrap_or_default();
    let path_entries: Vec<_> = env::split_paths(&current_path).collect();
    let tool_bin = project_path.join("tool-bin");
    fs::create_dir_all(&tool_bin)?;

    // Preserve Rust tools if prek was installed into the same directory as Cargo.
    for tool in ["cargo", "rustc"] {
        let executable = path_entries
            .iter()
            .map(|directory| directory.join(tool))
            .find(|candidate| candidate.is_file())
            .ok_or_else(|| Error::other(format!("{tool} not found on PATH")))?;
        symlink(fs::canonicalize(executable)?, tool_bin.join(tool))?;
    }

    let path = env::join_paths(
        std::iter::once(tool_bin.clone()).chain(
            path_entries
                .into_iter()
                .filter(|directory| !directory.join("prek").exists()),
        ),
    )
    .map_err(Error::other)?;

    Ok((path, tool_bin))
}

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

fn assert_prek_config_runs_default_install(prefix: &str, config_name: &str, config: &str) -> Result<(), Error> {
    let project = TestProject::new(prefix)?;
    project.add_husky_rs("dependencies", false)?;
    fs::write(project.path.join(config_name), config)?;

    let fake_bin = project.path.join("fake-bin");
    let invocation_log = project.path.join("prek-invocation.log");
    create_fake_prek(&fake_bin, 0)?;

    let output = std::process::Command::new("cargo")
        .arg("build")
        .current_dir(&project.path)
        .env("PATH", path_with_prepend(&fake_bin)?)
        .env("PREK_INVOCATION_LOG", &invocation_log)
        .output()?;

    assert!(
        output.status.success(),
        "cargo build failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let invocation = fs::read_to_string(&invocation_log)?;
    let lines: Vec<_> = invocation.lines().collect();
    assert_eq!(
        lines.len(),
        5,
        "expected validation followed by installation: {invocation}"
    );
    assert_eq!(fs::canonicalize(lines[0])?, fs::canonicalize(&project.path)?);
    assert_eq!(lines[1], "validate-config");
    assert_eq!(
        fs::canonicalize(lines[2])?,
        fs::canonicalize(project.path.join(config_name))?
    );
    assert_eq!(fs::canonicalize(lines[3])?, fs::canonicalize(&project.path)?);
    assert_eq!(lines[4], "install");
    assert!(
        !project.path.join(".husky").exists(),
        "husky-rs should not create .husky in prek mode"
    );

    Ok(())
}

/// A pre-commit YAML config delegates installation to prek.
#[test]
fn test_pre_commit_yaml_runs_default_install() -> Result<(), Error> {
    assert_prek_config_runs_default_install("install-prek-yaml-", ".pre-commit-config.yaml", "repos: []\n")
}

/// A pre-commit YML config delegates installation to prek.
#[test]
fn test_pre_commit_yml_runs_default_install() -> Result<(), Error> {
    assert_prek_config_runs_default_install("install-prek-yml-", ".pre-commit-config.yml", "repos: []\n")
}

/// A native prek TOML config delegates installation to prek.
#[test]
fn test_prek_toml_runs_default_install() -> Result<(), Error> {
    assert_prek_config_runs_default_install(
        "install-prek-toml-",
        "prek.toml",
        "[[repos]]\nrepo = \"builtin\"\nhooks = [{ id = \"check-toml\" }]\n",
    )
}

/// A prek config takes over hook management even when .husky already exists.
#[test]
fn test_prek_config_takes_over_existing_husky_mode() -> Result<(), Error> {
    let project = TestProject::new("install-prek-coexist-")?;
    project.add_husky_rs("dependencies", false)?;
    project.create_hook("pre-commit", "#!/bin/sh\necho 'standalone'\n")?;
    fs::write(project.path.join(".pre-commit-config.yaml"), "repos: []\n")?;

    let fake_bin = project.path.join("fake-bin");
    let invocation_log = project.path.join("prek-invocation.log");
    create_fake_prek(&fake_bin, 0)?;

    let output = std::process::Command::new("cargo")
        .arg("build")
        .current_dir(&project.path)
        .env("PATH", path_with_prepend(&fake_bin)?)
        .env("PREK_INVOCATION_LOG", &invocation_log)
        .output()?;

    assert!(
        output.status.success(),
        "cargo build failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(invocation_log.exists(), "prek config should select prek mode");
    assert!(!verify_hook_installed(&project.path, "pre-commit"));
    assert!(project.get_hook_content("pre-commit")?.contains("standalone"));

    Ok(())
}

/// Stable standalone inputs keep the husky-rs build script cached.
#[test]
fn test_standalone_mode_is_cached() -> Result<(), Error> {
    let project = TestProject::new("install-standalone-cache-")?;
    project.add_husky_rs("dependencies", false)?;
    project.create_hook("pre-commit", "#!/bin/sh\necho 'standalone'\n")?;
    project.build()?;

    let second = project.cargo(&["build", "-vv"])?;
    assert!(second.success, "second cargo build failed: {}", second.stderr);
    assert!(
        !second.stderr.contains("Dirty husky-rs"),
        "unchanged standalone inputs should not rerun husky-rs build.rs"
    );

    Ok(())
}

/// A prek execution failure stops the Cargo build with an actionable error.
#[test]
fn test_prek_install_failure_fails_build() -> Result<(), Error> {
    let project = TestProject::new("install-prek-fail-")?;
    project.add_husky_rs("dependencies", false)?;
    fs::write(project.path.join(".pre-commit-config.yaml"), "repos: []\n")?;

    let fake_bin = project.path.join("fake-bin");
    let invocation_log = project.path.join("prek-invocation.log");
    create_fake_prek(&fake_bin, 42)?;

    let output = std::process::Command::new("cargo")
        .arg("build")
        .current_dir(&project.path)
        .env("PATH", path_with_prepend(&fake_bin)?)
        .env("PREK_INVOCATION_LOG", &invocation_log)
        .output()?;

    assert!(!output.status.success(), "prek failure should fail cargo build");
    assert!(invocation_log.exists(), "fake prek should have been invoked");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("prek install failed with status"),
        "expected actionable prek failure error, got: {stderr}"
    );

    Ok(())
}

/// A missing prek executable stops the build with installation guidance.
#[cfg(unix)]
#[test]
fn test_prek_missing_fails_build() -> Result<(), Error> {
    let project = TestProject::new("install-prek-missing-")?;
    project.add_husky_rs("dependencies", false)?;
    fs::write(project.path.join(".pre-commit-config.yaml"), "repos: []\n")?;

    let (path_without_prek, _) = path_without_prek(&project.path)?;

    let output = std::process::Command::new("cargo")
        .arg("build")
        .current_dir(&project.path)
        .env("PATH", path_without_prek)
        .output()?;

    assert!(!output.status.success(), "missing prek should fail cargo build");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("cargo install prek"),
        "expected prek installation guidance, got: {stderr}"
    );

    Ok(())
}

/// NO_HUSKY_HOOKS explicitly skips required prek installation.
#[test]
fn test_prek_install_skipped_with_env_var() -> Result<(), Error> {
    let project = TestProject::new("install-prek-skip-")?;
    project.add_husky_rs("dependencies", false)?;
    fs::write(project.path.join(".pre-commit-config.yaml"), "repos: []\n")?;

    let fake_bin = project.path.join("fake-bin");
    let invocation_log = project.path.join("prek-invocation.log");
    create_fake_prek(&fake_bin, 0)?;

    let output = std::process::Command::new("cargo")
        .arg("build")
        .current_dir(&project.path)
        .env("NO_HUSKY_HOOKS", "1")
        .env("PATH", path_with_prepend(&fake_bin)?)
        .env("PREK_INVOCATION_LOG", &invocation_log)
        .output()?;

    assert!(
        output.status.success(),
        "explicit skip should keep cargo build successful"
    );
    assert!(
        !invocation_log.exists(),
        "prek should not run when NO_HUSKY_HOOKS is set"
    );

    Ok(())
}

/// A failed build retries successfully after prek appears on the same PATH.
#[cfg(unix)]
#[test]
fn test_prek_install_retries_after_binary_appears() -> Result<(), Error> {
    let project = TestProject::new("install-prek-retry-")?;
    project.add_husky_rs("dependencies", false)?;
    fs::write(project.path.join(".pre-commit-config.yaml"), "repos: []\n")?;

    let (path, tool_bin) = path_without_prek(&project.path)?;
    let invocation_log = project.path.join("prek-invocation.log");

    let first = std::process::Command::new("cargo")
        .arg("build")
        .current_dir(&project.path)
        .env("PATH", &path)
        .output()?;
    assert!(!first.status.success(), "build should fail before prek is installed");

    create_fake_prek(&tool_bin, 0)?;
    let second = std::process::Command::new("cargo")
        .arg("build")
        .current_dir(&project.path)
        .env("PATH", &path)
        .env("PREK_INVOCATION_LOG", &invocation_log)
        .output()?;
    assert!(second.status.success(), "build should retry after prek is installed");
    assert!(invocation_log.exists(), "newly installed prek should run");

    Ok(())
}

/// Stable prek inputs keep the husky-rs build script cached.
#[test]
fn test_prek_mode_is_cached() -> Result<(), Error> {
    let project = TestProject::new("install-prek-cache-")?;
    project.add_husky_rs("dependencies", false)?;
    fs::write(project.path.join(".pre-commit-config.yaml"), "repos: []\n")?;

    let fake_bin = project.path.join("fake-bin");
    let invocation_log = project.path.join("prek-invocation.log");
    create_fake_prek(&fake_bin, 0)?;
    let path = path_with_prepend(&fake_bin)?;

    let first = std::process::Command::new("cargo")
        .arg("build")
        .current_dir(&project.path)
        .env("PATH", &path)
        .env("PREK_INVOCATION_LOG", &invocation_log)
        .output()?;
    assert!(first.status.success(), "first cargo build failed");
    assert!(invocation_log.exists(), "first prek executable should run");
    fs::remove_file(&invocation_log)?;

    let second = std::process::Command::new("cargo")
        .arg("build")
        .current_dir(&project.path)
        .env("PATH", &path)
        .env("PREK_INVOCATION_LOG", &invocation_log)
        .output()?;
    assert!(second.status.success(), "second cargo build failed");
    assert!(
        !invocation_log.exists(),
        "unchanged prek inputs should not rerun husky-rs build.rs"
    );

    Ok(())
}

/// Real prek installs configured hook types and runs a local pre-commit hook.
#[cfg(unix)]
#[test]
fn test_real_prek_new_project_installs_and_runs_hooks() -> Result<(), Error> {
    if !real_prek_e2e_enabled() {
        return Ok(());
    }

    let project = TestProject::new("real-prek-new-")?;
    project.add_husky_rs("dependencies", false)?;
    write_executable(&project.path.join("prek-marker.sh"), "#!/bin/sh\ntouch prek-hook-ran\n")?;
    fs::write(
        project.path.join(".pre-commit-config.yaml"),
        r#"default_install_hook_types: [pre-commit, commit-msg, pre-push]
repos:
  - repo: local
    hooks:
      - id: prek-marker
        name: prek marker
        entry: ./prek-marker.sh
        language: system
        pass_filenames: false
"#,
    )?;

    project.build()?;

    for hook_type in ["pre-commit", "commit-msg", "pre-push"] {
        assert!(
            project.path.join(".git").join("hooks").join(hook_type).is_file(),
            "real prek should install the {hook_type} shim"
        );
    }

    fs::write(project.path.join("tracked.txt"), "content\n")?;
    run_command_success("git", &["add", "tracked.txt"], &project.path)?;
    run_command_success("git", &["commit", "-m", "test: real prek hook"], &project.path)?;
    assert!(
        project.path.join("prek-hook-ran").exists(),
        "real prek hook should run during git commit"
    );

    Ok(())
}

/// Real prek migrates an existing standalone hook and chains its legacy script.
#[cfg(unix)]
#[test]
fn test_real_prek_migrates_standalone_hook() -> Result<(), Error> {
    if !real_prek_e2e_enabled() {
        return Ok(());
    }

    let project = TestProject::new("real-prek-upgrade-")?;
    project.add_husky_rs("dependencies", false)?;
    project.create_hook("pre-commit", "#!/bin/sh\ntouch legacy-hook-ran\n")?;
    project.build()?;
    assert!(verify_hook_installed(&project.path, "pre-commit"));

    write_executable(&project.path.join("prek-marker.sh"), "#!/bin/sh\ntouch prek-hook-ran\n")?;
    fs::write(
        project.path.join(".pre-commit-config.yaml"),
        r#"repos:
  - repo: local
    hooks:
      - id: prek-marker
        name: prek marker
        entry: ./prek-marker.sh
        language: system
        pass_filenames: false
"#,
    )?;

    project.clean()?;
    project.build()?;

    assert!(project.path.join(".husky").join("pre-commit").is_file());
    assert!(project.path.join(".husky").join("pre-commit.legacy").is_file());

    fs::write(project.path.join("tracked.txt"), "content\n")?;
    run_command_success("git", &["add", "tracked.txt"], &project.path)?;
    run_command_success("git", &["commit", "-m", "test: prek migration"], &project.path)?;
    assert!(
        project.path.join("prek-hook-ran").exists(),
        "configured prek hook should run"
    );
    assert!(
        project.path.join("legacy-hook-ran").exists(),
        "legacy standalone hook should run"
    );

    Ok(())
}

/// Real prek config errors fail the Cargo build instead of leaving hooks absent.
#[cfg(unix)]
#[test]
fn test_real_prek_invalid_config_fails_build() -> Result<(), Error> {
    if !real_prek_e2e_enabled() {
        return Ok(());
    }

    let project = TestProject::new("real-prek-invalid-")?;
    project.add_husky_rs("dependencies", false)?;
    fs::write(project.path.join(".pre-commit-config.yaml"), "repos: [\n")?;

    let output = project.cargo(&["build"])?;
    assert!(!output.success, "invalid prek config should fail cargo build");
    assert!(
        output.stderr.contains("prek installation failed"),
        "expected prek failure details, got: {}",
        output.stderr
    );

    Ok(())
}

/// Real prek refuses an external global hooksPath and propagates the failure.
#[cfg(unix)]
#[test]
fn test_real_prek_global_hooks_path_fails_build() -> Result<(), Error> {
    if !real_prek_e2e_enabled() {
        return Ok(());
    }

    let project = TestProject::new("real-prek-global-hooks-")?;
    project.add_husky_rs("dependencies", false)?;
    fs::write(project.path.join(".pre-commit-config.yaml"), "repos: []\n")?;

    let global_hooks = project.path.join("global-hooks");
    let global_config = project.path.join("global.gitconfig");
    fs::write(
        &global_config,
        format!("[core]\n\thooksPath = {}\n", global_hooks.display()),
    )?;

    let output = std::process::Command::new("cargo")
        .arg("build")
        .current_dir(&project.path)
        .env("GIT_CONFIG_GLOBAL", &global_config)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()?;

    assert!(
        !output.status.success(),
        "external global hooksPath should fail cargo build"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Refusing to install hooks"),
        "expected prek hooksPath refusal, got: {stderr}"
    );

    Ok(())
}

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

    let fake_dir = project.path.join("fake-bin");
    create_failing_git(&fake_dir)?;

    let output = std::process::Command::new("cargo")
        .arg("build")
        .current_dir(&project.path)
        .env("PATH", path_with_prepend(&fake_dir)?)
        .output()?;

    assert!(output.status.success(), "build should succeed even if git config fails");

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
    run_command_success("git", &["config", "core.hooksPath", "custom-hooks"], &project.path)?;

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
    // Git Bash on Windows needs forward slashes
    let marker_str = marker.display().to_string().replace('\\', "/");
    project.create_hook("pre-commit", &format!("#!/bin/sh\ntouch {}\nexit 0\n", marker_str))?;
    project.build()?;

    // Add a file so there's something to commit
    fs::write(project.path.join("foo.txt"), "bar")?;
    run_command_success("git", &["add", "foo.txt"], &project.path)?;

    let output = run_command("git", &["commit", "-m", "test: trigger pre-commit"], &project.path)?;

    assert!(output.success, "commit should succeed: {}", output.stderr);
    assert!(marker.exists(), "pre-commit hook should have created marker file");

    Ok(())
}

/// pre-commit hook that exits non-zero correctly aborts the commit.
#[test]
fn test_hook_failure_aborts_commit() -> Result<(), Error> {
    let project = TestProject::new("trigger-fail-")?;
    project.add_husky_rs("dependencies", false)?;

    // Hook outputs a message then fails — both go to stderr
    project.create_hook("pre-commit", "#!/bin/sh\necho 'REJECTED by hook' >&2\nexit 1\n")?;
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
    let marker_str = marker.display().to_string().replace('\\', "/");
    // $1 is the path to the commit message file, not the message itself
    project.create_hook(
        "commit-msg",
        &format!("#!/bin/sh\ncat \"$1\" > {}\nexit 0\n", marker_str),
    )?;
    project.build()?;

    fs::write(project.path.join("foo.txt"), "bar")?;
    run_command_success("git", &["add", "foo.txt"], &project.path)?;

    let output = run_command("git", &["commit", "-m", "feat: test commit-msg hook"], &project.path)?;

    assert!(output.success, "commit should succeed: {}", output.stderr);
    assert!(marker.exists(), "commit-msg hook should have created marker file");

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
    let marker_str = marker.display().to_string().replace('\\', "/");
    project.create_hook("post-commit", &format!("#!/bin/sh\ntouch {}\nexit 0\n", marker_str))?;
    project.build()?;

    fs::write(project.path.join("foo.txt"), "bar")?;
    run_command_success("git", &["add", "foo.txt"], &project.path)?;

    let output = run_command("git", &["commit", "-m", "test: trigger post-commit"], &project.path)?;

    assert!(output.success, "commit should succeed: {}", output.stderr);
    assert!(marker.exists(), "post-commit hook should have created marker file");

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
    let _ = run_command("git", &["worktree", "remove", worktree.to_str().unwrap()], &main_repo);

    Ok(())
}
