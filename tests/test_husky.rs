use std::env;
use std::fs::{self, File};
use std::io::{Error, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn create_temp_dir(prefix: Option<&str>) -> Result<PathBuf, Error> {
    let prefix = prefix.unwrap_or("tmp-");
    let time_since_epoch = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    let temp_dir = env::temp_dir().join(format!("{}{}", prefix, time_since_epoch.as_secs()));

    fs::create_dir_all(&temp_dir)?;
    Ok(temp_dir)
}

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

fn create_hook(dir: &Path, name: &str, content: &str) -> std::io::Result<()> {
    let path = dir.join(name);
    let mut file = File::create(&path)?;
    file.write_all(content.as_bytes())?;
    Ok(())
}

struct TestProject {
    path: PathBuf,
}

impl TestProject {
    fn new(prefix: &str) -> Result<Self, Error> {
        let temp_dir = create_temp_dir(Some(prefix))?;
        let project = TestProject { path: temp_dir };
        project.init()?;
        Ok(project)
    }

    fn init(&self) -> Result<(), Error> {
        Command::new("cargo")
            .args(["init", "--bin"])
            .current_dir(&self.path)
            .status()?;
        Ok(())
    }

    fn add_husky_rs_to_toml(&self, dependencies_type: &str) -> Result<(), Error> {
        let cargo_toml_path = self.path.join("Cargo.toml");
        let mut cargo_toml = fs::read_to_string(&cargo_toml_path)?;
        let current_crate_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let relative_crate_path = get_relative_path(&self.path, &current_crate_path);

        // Check if [dependencies] or [dev-dependencies] exists
        if !cargo_toml.contains(&format!("[{}]", dependencies_type)) {
            cargo_toml.push_str(&format!("\n[{}]\n", dependencies_type));
        }

        // Add husky-rs to the suitable section
        let husky_rs_dep = format!("husky-rs = {{ path = {:?} }}\n", relative_crate_path);
        if let Some(pos) = cargo_toml.find(&format!("[{}]", dependencies_type)) {
            let insert_pos = cargo_toml[pos..]
                .find('\n')
                .map(|p| p + pos + 1)
                .unwrap_or(cargo_toml.len());
            cargo_toml.insert_str(insert_pos, &husky_rs_dep);
        }

        fs::write(&cargo_toml_path, cargo_toml)?;
        Ok(())
    }

    fn create_hooks(&self, content: &str) -> Result<(), Error> {
        let husky_dir = self.path.join(".husky").join("hooks");
        fs::create_dir_all(&husky_dir)?;
        for hook in &["pre-commit", "commit-msg", "pre-push"] {
            create_hook(&husky_dir, hook, content)?;
        }
        Ok(())
    }

    fn run_cargo_command(&self, command: &str) -> Result<(), Error> {
        Command::new("cargo")
            .arg(command)
            .current_dir(&self.path)
            .status()?;
        Ok(())
    }

    fn verify_hooks(&self) -> Result<(), Error> {
        let git_hooks_dir = self.path.join(".git").join("hooks");
        for hook in &["pre-commit", "commit-msg", "pre-push"] {
            let hook_path = git_hooks_dir.join(hook);
            assert!(hook_path.exists(), "Hook {} was not created", hook);

            let hook_content = fs::read_to_string(&hook_path)?;
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
        }
        Ok(())
    }
}

impl Drop for TestProject {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[test]
fn test_husky_rs_with_dependencies() -> Result<(), Error> {
    let project = TestProject::new("husky-rs-dep-test-")?;
    project.add_husky_rs_to_toml("dependencies")?;
    project.create_hooks("#!/bin/sh\necho \"This is a test hook\"\n")?;
    project.run_cargo_command("build")?;
    project.verify_hooks()?;
    Ok(())
}

#[test]
fn test_husky_rs_with_dev_dependencies() -> Result<(), Error> {
    let project = TestProject::new("husky-rs-dev-dep-test-")?;
    project.add_husky_rs_to_toml("dev-dependencies")?;
    project.create_hooks("#!/bin/sh\necho \"This is a test hook\"\n")?;
    project.run_cargo_command("test")?;
    project.verify_hooks()?;
    Ok(())
}

#[test]
fn test_husky_rs_after_cargo_clean() -> Result<(), Error> {
    let project = TestProject::new("husky-rs-clean-test-")?;
    project.add_husky_rs_to_toml("dependencies")?;
    project.create_hooks("#!/bin/sh\necho \"This is a test hook\"\n")?;
    project.run_cargo_command("build")?;
    project.run_cargo_command("clean")?;
    project.run_cargo_command("build")?;
    project.verify_hooks()?;
    Ok(())
}
