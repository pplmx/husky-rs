use std::env;
use std::fs;
use std::io::Error;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

// --- Constants ---
const HOOK_TYPES: &[&str] = &[
    "pre-commit",
    "prepare-commit-msg",
    "commit-msg",
    "post-commit",
    "pre-push",
];
const HOOK_TEMPLATE: &str = "#!/bin/sh\necho \"This is a test hook\"\n";

// --- RAII Guard for Environment Variables ---

/// An RAII guard to safely and temporarily modify environment variables during tests.
///
/// It encapsulates `unsafe` calls to `env::set_var` and `env::remove_var`,
/// ensuring that the original state of the environment variable is restored
/// when the guard goes out of scope. This prevents tests from interfering with each other.
struct TempEnvVar {
    key: String,
    original_state: Option<String>,
}

impl TempEnvVar {
    /// Creates a new guard, temporarily setting the environment variable `key` to `value`.
    pub fn new(key: &str, value: &str) -> Self {
        let key = key.to_string();
        let original_state = env::var(&key).ok();
        // Unsafe operation is contained here.
        unsafe { env::set_var(&key, value) };
        Self {
            key,
            original_state,
        }
    }

    /// Creates a new guard, temporarily removing the environment variable `key`.
    pub fn new_removed(key: &str) -> Self {
        let key = key.to_string();
        let original_state = env::var(&key).ok();
        if original_state.is_some() {
            // Unsafe operation is contained here.
            unsafe { env::remove_var(&key) };
        }
        Self {
            key,
            original_state,
        }
    }
}

impl Drop for TempEnvVar {
    /// Restores the original environment variable state when the guard is dropped.
    fn drop(&mut self) {
        unsafe {
            match &self.original_state {
                Some(v) => env::set_var(&self.key, v),
                None => env::remove_var(&self.key),
            }
        }
    }
}

// --- Custom File and Path Utilities ---

/// Checks if a directory is writable by attempting to create a temporary file.
fn is_writable(path: &Path) -> bool {
    fs::File::create(path.join(".write_test"))
        .map(|_| fs::remove_file(path.join(".write_test")).is_ok())
        .unwrap_or(false)
}

/// Creates a temporary directory for a test project, preferring the parent of the current crate.
fn create_temp_dir(prefix: &str) -> Result<PathBuf, Error> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let current_crate_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    if let Some(parent) = current_crate_path.parent()
        && is_writable(parent)
    {
        let temp_dir = parent.join(format!("{}{}", prefix, timestamp));
        if fs::create_dir_all(&temp_dir).is_ok() {
            return Ok(temp_dir);
        }
    }
    // Fallback to the system's temporary directory.
    let temp_dir = env::temp_dir().join(format!("{}{}", prefix, timestamp));
    fs::create_dir_all(&temp_dir)?;
    Ok(temp_dir)
}

/// Calculates the relative path from one directory to another.
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
    let mut result = PathBuf::new();
    for _ in 0..(from_components.len() - common_prefix) {
        result.push("..");
    }
    for component in &to_components[common_prefix..] {
        result.push(component.as_os_str());
    }
    result
}

// --- TestProject Struct & Implementation ---

/// Represents an isolated, self-cleaning test project environment.
struct TestProject {
    path: PathBuf,
}
impl TestProject {
    /// Creates a new test project in a temporary directory and initializes it.
    pub fn new(prefix: &str) -> Result<Self, Error> {
        let path = create_temp_dir(prefix)?;
        println!("[TEST_SETUP] Project at: {}", path.display());
        let project = Self { path };
        project.init()?;
        Ok(project)
    }

    /// Initializes a new Cargo project in the test directory.
    fn init(&self) -> Result<(), Error> {
        Command::new("cargo")
            .args(["init", "--bin"])
            .current_dir(&self.path)
            .status()?;
        Ok(())
    }

    /// Adds husky-rs as a dependency to the project's Cargo.toml.
    pub fn add_husky_rs_to_toml(&self, dep_type: &str, use_abs: bool) -> Result<(), Error> {
        let cargo_toml_path = self.path.join("Cargo.toml");
        let mut cargo_toml = fs::read_to_string(&cargo_toml_path)?;
        let crate_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let local_path = if use_abs {
            crate_path
        } else {
            get_relative_path(&self.path, &crate_path)
        };
        let section = format!("[{}]", dep_type);
        let dep_line = format!("husky-rs = {{ path = {:?} }}\n", local_path);
        if let Some(pos) = cargo_toml.find(&section) {
            let insert_pos = cargo_toml[pos..]
                .find('\n')
                .map(|p| p + pos + 1)
                .unwrap_or(cargo_toml.len());
            cargo_toml.insert_str(insert_pos, &dep_line);
        } else {
            cargo_toml.push_str(&format!("\n{}\n{}", section, dep_line));
        }
        fs::write(cargo_toml_path, cargo_toml)
    }

    /// Creates user-defined git hooks in the .husky/hooks directory.
    pub fn create_hooks(&self) -> Result<(), Error> {
        let dir = self.husky_hooks_dir();
        fs::create_dir_all(&dir)?;
        for hook in HOOK_TYPES {
            fs::write(dir.join(hook), HOOK_TEMPLATE)?;
        }
        Ok(())
    }

    /// Runs a cargo command and returns an error if it fails.
    pub fn run_cargo_command(&self, cmd: &str) -> Result<(), Error> {
        let status = Command::new("cargo")
            .arg(cmd)
            .current_dir(&self.path)
            .status()?;
        if status.success() {
            Ok(())
        } else {
            Err(Error::other(format!("`cargo {}` failed: {}", cmd, status)))
        }
    }

    /// Runs a cargo command and captures its stdout, stderr, and success status.
    pub fn run_cargo_command_with_output(
        &self,
        args: &[&str],
    ) -> Result<(String, String, bool), Error> {
        let out = Command::new("cargo")
            .args(args)
            .current_dir(&self.path)
            .output()?;
        Ok((
            String::from_utf8_lossy(&out.stdout).into(),
            String::from_utf8_lossy(&out.stderr).into(),
            out.status.success(),
        ))
    }

    /// Verifies whether git hooks were installed as expected.
    pub fn verify_hooks(&self, expect: bool) -> Result<(), Error> {
        for hook in HOOK_TYPES {
            let path = self.git_hooks_dir().join(hook);
            let exists = path.exists();
            let content = if exists {
                fs::read_to_string(&path)?
            } else {
                String::new()
            };
            if expect {
                assert!(exists, "Hook {} was not created", hook);
                assert!(
                    content.contains("This hook was set by husky-rs"),
                    "Hook {} is missing the husky-rs header",
                    hook
                );
                assert!(
                    content.contains("This is a test hook"),
                    "Hook {} is missing the original content",
                    hook
                );
            } else {
                assert!(
                    !exists || !content.contains("This hook was set by husky-rs"),
                    "Hook {} was unexpectedly created or modified",
                    hook
                );
            }
        }
        Ok(())
    }

    // --- Path helpers ---
    pub fn git_hooks_dir(&self) -> PathBuf {
        self.path.join(".git").join("hooks")
    }
    pub fn husky_hooks_dir(&self) -> PathBuf {
        self.path.join(".husky").join("hooks")
    }
}

impl Drop for TestProject {
    /// Ensures the temporary project directory is cleaned up after the test.
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

// --- Test Fixtures ---

/// Base fixture: Creates a project and adds husky-rs as a dependency.
/// This fixture does not create any user hooks, allowing for custom hook setup in tests.
fn with_project_setup<F>(name: &str, dep_type: &str, use_abs: bool, logic: F) -> Result<(), Error>
where
    F: FnOnce(&TestProject) -> Result<(), Error>,
{
    let project = TestProject::new(&format!("husky-rs-{}-", name))?;
    project.add_husky_rs_to_toml(dep_type, use_abs)?;
    logic(&project)
}

/// Standard fixture: Creates a project, adds husky-rs as a `[dependencies]`, and creates standard hooks.
fn with_project<F>(name: &str, use_abs: bool, logic: F) -> Result<(), Error>
where
    F: FnOnce(&TestProject) -> Result<(), Error>,
{
    with_project_setup(name, "dependencies", use_abs, |project| {
        project.create_hooks()?;
        logic(project)
    })
}

/// Dev dependency fixture: Creates a project, adds husky-rs as a `[dev-dependencies]`, and creates standard hooks.
fn with_dev_project<F>(name: &str, use_abs: bool, logic: F) -> Result<(), Error>
where
    F: FnOnce(&TestProject) -> Result<(), Error>,
{
    with_project_setup(name, "dev-dependencies", use_abs, |project| {
        project.create_hooks()?;
        logic(project)
    })
}

// --- Test Cases ---

#[test]
fn test_husky_rs_with_dep_rel() -> Result<(), Error> {
    with_project("dep-rel", false, |p| {
        p.run_cargo_command("build")?;
        p.verify_hooks(true)
    })
}
#[test]
fn test_husky_rs_with_dep_abs() -> Result<(), Error> {
    with_project("dep-abs", true, |p| {
        p.run_cargo_command("build")?;
        p.verify_hooks(true)
    })
}
#[test]
fn test_husky_rs_with_dep_after_cargo_clean() -> Result<(), Error> {
    with_project("clean", false, |p| {
        p.run_cargo_command("build")?;
        p.verify_hooks(true)?;
        p.run_cargo_command("clean")?;
        p.run_cargo_command("build")?;
        p.verify_hooks(true)
    })
}
#[test]
fn test_husky_rs_with_dev_dep_rel_and_cargo_test() -> Result<(), Error> {
    with_dev_project("dev-rel-test", false, |p| {
        p.run_cargo_command("test")?;
        p.verify_hooks(true)
    })
}
#[test]
fn test_husky_rs_with_dev_dep_abs_and_cargo_test() -> Result<(), Error> {
    with_dev_project("dev-abs-test", true, |p| {
        p.run_cargo_command("test")?;
        p.verify_hooks(true)
    })
}
#[test]
fn test_husky_rs_with_dev_dep_and_cargo_build() -> Result<(), Error> {
    with_dev_project("dev-build", false, |p| {
        p.run_cargo_command("build")?;
        p.verify_hooks(false)
    })
}

#[test]
fn test_shebang_variations() -> Result<(), Error> {
    let _env_guard = TempEnvVar::new_removed("NO_HUSKY_HOOKS");
    with_project("shebang", false, |p| {
        let (out, err, success) = p.run_cargo_command_with_output(&["build"])?;
        println!("---STDOUT---\n{}\n---STDERR---\n{}", out, err);
        assert!(success, "Build failed when it should have succeeded");
        let hook_path = p.git_hooks_dir().join("pre-commit");
        assert!(hook_path.exists(), "Hook was not installed");
        let content = fs::read_to_string(hook_path)?;
        assert!(
            content.starts_with("#!/bin/sh"),
            "Installed hook has an incorrect shebang"
        );
        Ok(())
    })
}

#[test]
fn test_empty_user_hook_script() -> Result<(), Error> {
    let _env_guard = TempEnvVar::new_removed("NO_HUSKY_HOOKS");
    // Use the base fixture to allow for custom hook creation.
    with_project_setup("empty-hook", "dependencies", false, |p| {
        fs::create_dir_all(p.husky_hooks_dir())?;
        // Create an empty and a whitespace-only hook.
        fs::write(p.husky_hooks_dir().join("pre-commit"), "")?;
        fs::write(p.husky_hooks_dir().join("pre-push"), "   \n\t  ")?;
        assert!(
            p.run_cargo_command("build").is_err(),
            "Build should fail for projects with empty user hooks"
        );
        p.verify_hooks(false)
    })
}

#[test]
#[cfg(unix)]
fn test_symbolic_link_hook() -> Result<(), Error> {
    let _env_guard = TempEnvVar::new_removed("NO_HUSKY_HOOKS");
    // Use the base fixture to allow for custom symlink creation.
    with_project_setup("symlink", "dependencies", false, |p| {
        let hooks_dir = p.husky_hooks_dir();
        fs::create_dir_all(&hooks_dir)?;
        let target = p.path.join("actual.sh");
        let content = "#!/bin/sh\necho \"symlink target\"";
        fs::write(&target, content)?;
        // Create the symlink pointing to the target script.
        std::os::unix::fs::symlink(&target, hooks_dir.join("pre-commit"))?;
        p.run_cargo_command("build")?;
        let installed = p.git_hooks_dir().join("pre-commit");
        assert!(installed.exists(), "Symlinked hook was not installed");
        assert!(
            fs::read_to_string(installed)?.contains("symlink target"),
            "Installed hook does not contain content from symlinked target"
        );
        Ok(())
    })
}

#[test]
fn test_no_hooks_if_env_var_set() -> Result<(), Error> {
    // Set the environment variable for the duration of this test.
    let _env_guard = TempEnvVar::new("NO_HUSKY_HOOKS", "1");
    with_project("no-hooks-env", false, |p| {
        // Explicitly pass the env var to the command for a more robust test,
        // ensuring the build process itself sees the variable.
        let out = Command::new("cargo")
            .arg("build")
            .current_dir(&p.path)
            .env("NO_HUSKY_HOOKS", "1")
            .output()?;
        assert!(
            out.status.success(),
            "Build should succeed even when hooks are disabled"
        );
        p.verify_hooks(false)
    })
}
