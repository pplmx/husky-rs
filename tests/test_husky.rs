use std::env;
use std::fs::{self};
use std::io::Error;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

const HOOK_TYPES: &[&str] = &[
    "pre-commit",
    "prepare-commit-msg",
    "commit-msg",
    "post-commit",
    "pre-push",
];
const HOOK_TEMPLATE: &str = "#!/bin/sh\necho \"This is a test hook\"\n";

// Check if a directory is writable by attempting to create a test file
fn is_writable(path: &Path) -> bool {
    let test_file = path.join(".write_test");
    match fs::File::create(&test_file) {
        Ok(_) => {
            let _ = fs::remove_file(&test_file);
            true
        }
        Err(_) => false,
    }
}

// Create temporary directory, preferring parent directory of current crate, fallback to system temp
fn create_temp_dir(prefix: &str) -> Result<PathBuf, Error> {
    let time_since_epoch = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    let timestamp = time_since_epoch.as_secs();

    // Try parent directory of current crate first
    let current_crate_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    if let Some(parent_path) = current_crate_path.parent() {
        if is_writable(parent_path) {
            let temp_dir = parent_path.join(format!("{}{}", prefix, timestamp));
            if fs::create_dir_all(&temp_dir).is_ok() {
                return Ok(temp_dir);
            }
        }
    }

    // Fallback to system temp directory
    let temp_dir = env::temp_dir().join(format!("{}{}", prefix, timestamp));
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
    fn create_hooks(&self) -> Result<(), Error> {
        let husky_dir = self.path.join(".husky").join("hooks");
        fs::create_dir_all(&husky_dir)?;
        for hook in HOOK_TYPES {
            let path = husky_dir.join(hook);
            fs::write(&path, HOOK_TEMPLATE)?;
        }
        Ok(())
    }

    // Runs a cargo command (e.g., build, test, clean) in the project directory
    fn run_cargo_command(&self, command: &str) -> Result<(), Error> {
        let status = Command::new("cargo")
            .arg(command)
            .current_dir(&self.path)
            .status()?;
        if status.success() {
            Ok(())
        } else {
            Err(Error::other(
                // Changed here
                format!(
                    "Cargo command `cargo {}` failed with status: {}",
                    command, status
                ),
            ))
        }
    }

    // Runs a cargo command and returns its output (stdout, stderr, success_status)
    fn run_cargo_command_with_output(
        &self,
        command_args: &[&str],
    ) -> Result<(String, String, bool), Error> {
        let output = Command::new("cargo")
            .args(command_args)
            .current_dir(&self.path)
            .output()?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let success = output.status.success();

        // No longer returns Err directly on failure, success status is returned instead.
        // The caller can decide how to handle non-success.
        Ok((stdout, stderr, success))
    }

    // Verifies the existence and content of Git hooks
    fn verify_hooks(&self, expect_hooks: bool) -> Result<(), Error> {
        let git_hooks_dir = self.path.join(".git").join("hooks");
        for hook in HOOK_TYPES {
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
    project.create_hooks()?;
    project.run_cargo_command("build")?;
    project.verify_hooks(true)
}

// Test: Debugging hook installation, using project.create_hooks() and capturing build output
#[test]
fn test_shebang_variations() -> Result<(), Error> {
    env::remove_var("NO_HUSKY_HOOKS"); // Explicitly unset NO_HUSKY_HOOKS
    println!("[TEST] Ensured NO_HUSKY_HOOKS is not set.");

    let project = TestProject::new("husky-rs-capture-output-")?;
    println!("[TEST] TestProject path: {}", project.path.display());

    project.add_husky_rs_to_toml("dependencies")?;
    println!("[TEST] Added husky-rs to Cargo.toml");

    project.create_hooks()?;
    println!("[TEST] Ran project.create_hooks()");

    let husky_hooks_dir = project.path.join(".husky").join("hooks");
    let user_pre_commit_path = husky_hooks_dir.join("pre-commit");
    assert!(
        user_pre_commit_path.exists(),
        "[TEST_ASSERT] User pre-commit hook file should exist at {} after project.create_hooks().",
        user_pre_commit_path.display()
    );

    println!("[TEST] Listing files in .husky/hooks before build:");
    if husky_hooks_dir.exists() {
        for entry in fs::read_dir(&husky_hooks_dir)?.flatten() {
            println!("[TEST]   - {}", entry.path().display());
        }
    } else {
        println!("[TEST] .husky/hooks directory does NOT exist before build (unexpected after create_hooks).");
    }

    println!("[TEST] Attempting to run cargo build and capture output...");
    let (build_stdout, build_stderr, build_success) =
        project.run_cargo_command_with_output(&["build"])?;

    println!("[TEST] --- Cargo Build STDOUT ---");
    println!("{}", build_stdout);
    println!("[TEST] --- End Cargo Build STDOUT ---");

    println!("[TEST] --- Cargo Build STDERR ---");
    println!("{}", build_stderr);
    println!("[TEST] --- End Cargo Build STDERR ---");

    if build_success {
        println!("[TEST] cargo build reported success.");
    } else {
        println!(
            "[TEST] cargo build reported FAILURE. Test might behave unexpectedly or fail early."
        );
        // Even if build reports failure (e.g. due to husky-rs build script erroring),
        // we still want to check the filesystem state as per the original test intent.
        // However, if the build of the test project itself failed, it's a significant issue.
        // For now, we'll proceed to check hooks, but this indicates a problem.
    }

    // Verification
    let git_hooks_dir = project.path.join(".git").join("hooks");
    println!(
        "[TEST] Expected Git hooks directory: {}",
        git_hooks_dir.display()
    );

    // List files in .git/hooks
    if git_hooks_dir.exists() {
        println!("[TEST] Listing files in .git/hooks after build:");
        for entry in fs::read_dir(&git_hooks_dir)?.flatten() {
            println!("[TEST]   - {}", entry.path().display());
        }
    } else {
        println!("[TEST] .git/hooks directory does NOT exist after build!");
    }

    let installed_pre_commit_path = git_hooks_dir.join("pre-commit");
    println!(
        "[TEST] Checking for installed hook at: {}",
        installed_pre_commit_path.display()
    );

    assert!(
        installed_pre_commit_path.exists(),
        "[TEST_ASSERT] pre-commit hook was not installed at {} by husky-rs build script.",
        installed_pre_commit_path.display()
    );

    let content = fs::read_to_string(&installed_pre_commit_path)?;
    // HOOK_TEMPLATE uses "#!/bin/sh"
    assert!(
        content.starts_with("#!/bin/sh"),
        "[TEST_ASSERT] Installed pre-commit hook does not start with #!/bin/sh (from HOOK_TEMPLATE)."
    );
    assert!(
        content.contains("This hook was set by husky-rs"),
        "[TEST_ASSERT] Installed pre-commit hook does not contain husky header."
    );
    assert!(
        content.contains("echo \"This is a test hook\""), // from HOOK_TEMPLATE
        "[TEST_ASSERT] Installed pre-commit hook does not contain original content from HOOK_TEMPLATE."
    );

    Ok(())
}

// Test: Verify that empty or whitespace-only user hook scripts are not installed
#[test]
fn test_empty_user_hook_script() -> Result<(), Error> {
    env::remove_var("NO_HUSKY_HOOKS"); // Ensure clean env state for this test too

    let project = TestProject::new("husky-rs-empty-hook-")?;
    project.add_husky_rs_to_toml("dependencies")?;

    let husky_hooks_dir = project.path.join(".husky").join("hooks");
    fs::create_dir_all(&husky_hooks_dir)?;

    // Create an empty pre-commit hook
    fs::write(husky_hooks_dir.join("pre-commit"), "")?;

    // Create a pre-push hook with only whitespace
    fs::write(husky_hooks_dir.join("pre-push"), "   \n   \t  ")?;

    // Expect the build to fail because husky-rs build script should error out
    let build_result = project.run_cargo_command("build");
    assert!(
        build_result.is_err(),
        "Build should fail for project with empty/whitespace user hooks. Result was: {:?}",
        build_result
    );

    if let Err(e) = &build_result {
        println!("[TEST_EMPTY_HOOK] Build failed as expected: {}", e);
        // Further check if the error message contains specific text from husky-rs build script if desired
        // e.g., assert!(e.to_string().contains("User hook script is empty"));
    }

    // Assertions that hooks were NOT installed (these should still hold)
    let git_hooks_dir = project.path.join(".git").join("hooks");
    assert!(
        !git_hooks_dir.join("pre-commit").exists(),
        "Empty pre-commit hook script should not have been installed"
    );
    assert!(
        !git_hooks_dir.join("pre-push").exists(),
        "Whitespace-only pre-push hook script should not have been installed"
    );

    Ok(()) // If assertions pass and build failed as expected
}

// Test: Verify husky-rs correctly installs a hook when the user's hook is a symbolic link (Unix-only)
#[test]
fn test_symbolic_link_hook() -> Result<(), Error> {
    env::remove_var("NO_HUSKY_HOOKS");
    println!("[TEST_SYMLINK] Ensured NO_HUSKY_HOOKS is not set.");

    let project = TestProject::new("husky-rs-symlink-abs-")?; // New prefix for fresh project
    println!(
        "[TEST_SYMLINK] TestProject path: {}",
        project.path.display()
    );

    project.add_husky_rs_to_toml("dependencies")?;
    println!("[TEST_SYMLINK] Added husky-rs to Cargo.toml");

    let husky_hooks_dir = project.path.join(".husky").join("hooks");
    fs::create_dir_all(&husky_hooks_dir)?;
    println!(
        "[TEST_SYMLINK] Created .husky/hooks directory at: {}",
        husky_hooks_dir.display()
    );

    // Create the actual hook script target
    let target_script_name = "actual_pre_commit_script.sh";
    // Ensure target_script_path is absolute for symlink creation robustness
    let target_script_path = fs::canonicalize(project.path.clone())?.join(target_script_name);
    let target_script_content =
        "#!/bin/sh\necho \"Actual script (via symlink) is running!\"\nexit 0";

    fs::write(&target_script_path, target_script_content.as_bytes())?; // Use as_bytes() for clarity
    println!(
        "[TEST_SYMLINK] Wrote target script to: {}",
        target_script_path.display()
    );

    #[cfg(unix)]
    {
        use std::os::unix::fs::{symlink, PermissionsExt};

        // Pre-assertion: Target script exists and is a file
        assert!(
            target_script_path.exists(),
            "[TEST_SYMLINK_PRE_ASSERT] Target script file should exist at {}.",
            target_script_path.display()
        );
        assert!(
            target_script_path.is_file(),
            "[TEST_SYMLINK_PRE_ASSERT] Target script path should be a file."
        );

        fs::set_permissions(&target_script_path, fs::Permissions::from_mode(0o755))?;
        println!("[TEST_SYMLINK] Set target script executable (Unix).");
        // Pre-assertion: Target script is executable
        let perms = fs::metadata(&target_script_path)?.permissions();
        assert_eq!(
            perms.mode() & 0o111,
            0o111,
            "[TEST_SYMLINK_PRE_ASSERT] Target script should be executable."
        );

        // Create the symbolic link in .husky/hooks using absolute paths
        let symlink_path = husky_hooks_dir.join("pre-commit");
        symlink(&target_script_path, &symlink_path)?; // std::os::unix::fs::symlink

        println!(
            "[TEST_SYMLINK] Created symlink at: {} -> {}",
            symlink_path.display(),
            target_script_path.display()
        );

        // Detailed Symlink Pre-Assertions
        assert!(
            symlink_path.exists(),
            "[TEST_SYMLINK_PRE_ASSERT] Symlink path {} should exist.",
            symlink_path.display()
        );
        let symlink_meta = fs::symlink_metadata(&symlink_path)?;
        assert!(
            symlink_meta.file_type().is_symlink(),
            "[TEST_SYMLINK_PRE_ASSERT] Path at {} should be a symlink.",
            symlink_path.display()
        );

        let link_target = fs::read_link(&symlink_path)?;
        assert_eq!(
            link_target, target_script_path,
            "[TEST_SYMLINK_PRE_ASSERT] Symlink target should match expected target path."
        );
        println!(
            "[TEST_SYMLINK] Symlink at {} correctly points to {}.",
            symlink_path.display(),
            link_target.display()
        );

        let symlink_content_before_build = fs::read_to_string(&symlink_path)?;
        assert_eq!(symlink_content_before_build, target_script_content, "[TEST_SYMLINK_PRE_ASSERT] Content read via symlink should match target script content.");
        println!(
            "[TEST_SYMLINK] Content read via symlink matches target script content before build."
        );

        // Run cargo build using the method that captures output
        println!("[TEST_SYMLINK] Attempting to run cargo build and capture output...");
        let (build_stdout, build_stderr, build_success) =
            project.run_cargo_command_with_output(&["build"])?;

        println!("[TEST_SYMLINK] --- Cargo Build STDOUT (Symlink Test) ---");
        println!("{}", build_stdout);
        println!("[TEST_SYMLINK] --- End Cargo Build STDOUT (Symlink Test) ---");

        println!("[TEST_SYMLINK] --- Cargo Build STDERR (Symlink Test) ---");
        println!("{}", build_stderr);
        println!("[TEST_SYMLINK] --- End Cargo Build STDERR (Symlink Test) ---");

        if !build_success {
            // It's important to list files even if build fails, to see the state
            // list_files_in_git_hooks(&project.path.join(".git").join("hooks"));
            return Err(Error::other(
                // Changed here
                format!(
                    "Cargo build failed for symlink test. Stderr: {}",
                    build_stderr
                ),
            ));
        }
        println!("[TEST_SYMLINK] cargo build reported success.");

        // Verification (Post-Build)
        let installed_hook_path = project.path.join(".git").join("hooks").join("pre-commit");
        println!(
            "[TEST_SYMLINK] Checking for installed hook at: {}",
            installed_hook_path.display()
        );

        // Listing files in .git/hooks for diagnostics
        let git_hooks_dir_for_listing = project.path.join(".git").join("hooks");
        if git_hooks_dir_for_listing.exists() {
            println!("[TEST_SYMLINK] Listing files in .git/hooks after build:");
            for entry in fs::read_dir(&git_hooks_dir_for_listing)?.flatten() {
                println!("[TEST_SYMLINK]   - {}", entry.path().display());
            }
        } else {
            println!("[TEST_SYMLINK] .git/hooks directory does NOT exist after build!");
        }

        assert!(
            installed_hook_path.exists(),
            "[TEST_SYMLINK_ASSERT] Installed pre-commit hook (from symlink) was not found at {}.",
            installed_hook_path.display()
        );

        let content = fs::read_to_string(&installed_hook_path)?;
        assert!(
            content.starts_with("#!/bin/sh"),
            "[TEST_SYMLINK_ASSERT] Installed hook does not start with #!/bin/sh."
        );
        assert!(
            content.contains("This hook was set by husky-rs"),
            "[TEST_SYMLINK_ASSERT] Installed hook does not contain husky header."
        );
        assert!(
            content.contains("echo \"Actual script (via symlink) is running!\""),
            "[TEST_SYMLINK_ASSERT] Installed hook does not contain content from target script."
        );
        println!("[TEST_SYMLINK] Symlink hook test completed and verified (Unix).");
    }
    #[cfg(not(unix))]
    {
        println!("[TEST_SYMLINK] Skipping symlink creation and verification on non-Unix platform.");
    }

    Ok(())
}

// Test: Verify no hooks are installed if NO_HUSKY_HOOKS is set
#[test]
fn test_no_hooks_if_env_var_set() -> Result<(), Error> {
    std::env::set_var("NO_HUSKY_HOOKS", "1"); // Set for current process (less critical but good for belt-and-suspenders)
    println!("[TEST NO_HOOKS_ENV_VAR] Set NO_HUSKY_HOOKS=1 for current process.");

    let project = TestProject::new("husky-rs-no-hooks-env-var-")?;
    println!(
        "[TEST NO_HOOKS_ENV_VAR] TestProject path: {}",
        project.path.display()
    );
    project.add_husky_rs_to_toml("dependencies")?;
    project.create_hooks()?; // Creates dummy hooks in .husky/hooks/
    println!("[TEST NO_HOOKS_ENV_VAR] Created dummy hooks in .husky/hooks/");

    println!("[TEST NO_HOOKS_ENV_VAR] Running cargo build with NO_HUSKY_HOOKS=1 explicitly set for the command...");
    let build_output = std::process::Command::new("cargo")
        .arg("build")
        .current_dir(&project.path)
        .env("NO_HUSKY_HOOKS", "1") // Explicitly set for this command
        .output()?;

    println!(
        "[TEST NO_HOOKS_ENV_VAR] cargo build STDOUT:\n{}",
        String::from_utf8_lossy(&build_output.stdout)
    );
    println!(
        "[TEST NO_HOOKS_ENV_VAR] cargo build STDERR:\n{}",
        String::from_utf8_lossy(&build_output.stderr)
    );

    assert!(
        build_output.status.success(),
        "[TEST NO_HOOKS_ENV_VAR] Cargo build should succeed even with NO_HUSKY_HOOKS set. Stderr: {}",
        String::from_utf8_lossy(&build_output.stderr)
    );
    println!("[TEST NO_HOOKS_ENV_VAR] Cargo build command reported success.");

    // Key verification: husky-rs build script should have seen NO_HUSKY_HOOKS and skipped installation
    let verify_result = project.verify_hooks(false);

    std::env::remove_var("NO_HUSKY_HOOKS"); // Clean up env var for current process
    println!("[TEST NO_HOOKS_ENV_VAR] Removed NO_HUSKY_HOOKS from current process environment.");

    verify_result
}

// Test: Verify husky-rs works as a dev dependency with cargo test
#[test]
fn test_husky_rs_with_dev_dependencies_and_cargo_test() -> Result<(), Error> {
    let project = TestProject::new("husky-rs-dev-dep-test-")?;
    project.add_husky_rs_to_toml("dev-dependencies")?;
    project.create_hooks()?;
    project.run_cargo_command("test")?;
    project.verify_hooks(true)
}

// Test: Verify husky-rs works as a dev dependency with cargo build, no hooks expected
#[test]
fn test_husky_rs_with_dev_dependencies_and_cargo_build() -> Result<(), Error> {
    let project = TestProject::new("husky-rs-dev-dep-build-test-")?;
    project.add_husky_rs_to_toml("dev-dependencies")?;
    project.create_hooks()?;
    project.run_cargo_command("build")?;
    project.verify_hooks(false)
}

// Test: Verify husky-rs works correctly after a cargo clean
#[test]
fn test_husky_rs_after_cargo_clean() -> Result<(), Error> {
    let project = TestProject::new("husky-rs-clean-test-")?;
    project.add_husky_rs_to_toml("dependencies")?;
    project.create_hooks()?;
    project.run_cargo_command("build")?;
    project.run_cargo_command("clean")?;
    project.run_cargo_command("build")?;
    project.verify_hooks(true)
}
