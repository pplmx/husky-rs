//! Build script for husky-rs
//!
//! Automatically configures Git hooks. Supports two modes:
//!
//! ## Mode 1 — standalone (no prek-supported config)
//!
//! Husky-rs sets `core.hooksPath` to `.husky/`. Users create and manage
//! their own hook scripts there. Hooks are made executable on install.
//!
//! ## Mode 2 — prek integration
//!
//! Husky-rs detects `prek.toml`, `.pre-commit-config.yaml`, or
//! `.pre-commit-config.yml` and delegates hook management completely to prek.
//! If `prek` is available on PATH, `prek install` is run automatically using
//! prek's configuration-compatible hook type defaults.
//!
//! If prek is unavailable, the config is invalid, or installation fails, the
//! build fails. Set `NO_HUSKY_HOOKS` to skip hook installation explicitly.

use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{self, Command, Output, Stdio};

#[derive(Debug)]
enum HuskyError {
    GitDirNotFound(String),
    Io(io::Error),
    GitConfigFailed(String),
    PrekInstallFailed(String),
}

impl std::fmt::Display for HuskyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HuskyError::GitDirNotFound(path) => {
                write!(f, "Git directory not found in '{path}' or its parent directories")
            }
            HuskyError::Io(err) => write!(f, "IO error: {err}"),
            HuskyError::GitConfigFailed(err) => write!(f, "Git config failed: {err}"),
            HuskyError::PrekInstallFailed(err) => write!(f, "prek installation failed: {err}"),
        }
    }
}

impl std::error::Error for HuskyError {}

impl From<io::Error> for HuskyError {
    fn from(err: io::Error) -> Self {
        HuskyError::Io(err)
    }
}

type Result<T> = std::result::Result<T, HuskyError>;

const HUSKY_DIR: &str = ".husky";
const PREK_CONFIGS: &[&str] = &["prek.toml", ".pre-commit-config.yaml", ".pre-commit-config.yml"];

fn main() {
    if let Err(error) = run() {
        eprintln!("husky-rs: {error}");
        process::exit(1);
    }
}

fn run() -> Result<()> {
    println!("cargo:rerun-if-env-changed=NO_HUSKY_HOOKS");

    if env::var_os("NO_HUSKY_HOOKS").is_some() {
        return Ok(());
    }

    install_hooks().or_else(|error| match error {
        HuskyError::GitDirNotFound(path) => {
            eprintln!("husky-rs: Unable to find .git directory starting from: {path}");
            Ok(())
        }
        HuskyError::GitConfigFailed(error) => {
            eprintln!("husky-rs: Failed to set git config: {error}");
            Ok(())
        }
        error @ (HuskyError::Io(_) | HuskyError::PrekInstallFailed(_)) => Err(error),
    })
}

fn install_hooks() -> Result<()> {
    let git_dir = find_git_dir()?;
    let project_root = git_dir
        .parent()
        .ok_or_else(|| HuskyError::GitDirNotFound(git_dir.display().to_string()))?;

    let user_hooks_dir = project_root.join(HUSKY_DIR);
    let prek_config = PREK_CONFIGS
        .iter()
        .map(|config| project_root.join(config))
        .find(|config| config.is_file());

    if let Some(prek_config) = prek_config {
        println!("cargo:rerun-if-changed={}", prek_config.display());
        install_prek_mode(project_root, &prek_config)
    } else {
        println!("cargo:rerun-if-changed={}", user_hooks_dir.display());
        install_standalone_mode(project_root)
    }
}

/// Mode 2: prek integration.
///
/// When a prek-supported config exists, delegate everything to prek.
/// prek installs hooks natively into `.git/hooks/` — no `.husky/` directory needed.
/// Any prior `core.hooksPath` (e.g. `.husky`) is cleared.
fn install_prek_mode(project_root: &Path, config_path: &Path) -> Result<()> {
    // 1. Validate config before making any changes.
    let validation = Command::new("prek")
        .arg("validate-config")
        .arg(config_path)
        .current_dir(project_root)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .output();
    ensure_prek_command_succeeds("prek validate-config", validation, config_path)?;

    // 2. Warn if .husky/ exists — it will be ignored in prek mode.
    let husky_dir = project_root.join(HUSKY_DIR);
    if husky_dir.is_dir() {
        println!(
            "cargo:warning=husky-rs: {} detected — .husky/ will be ignored (prek manages hooks via .git/hooks/)",
            config_path.file_name().unwrap_or_default().to_string_lossy()
        );
    }

    // 3. Clear any prior core.hooksPath (e.g. leftover from standalone `.husky` mode).
    //    In prek mode, hooks live natively in `.git/hooks/` — the git default.
    let current_hooks_path = Command::new("git")
        .args(["config", "core.hooksPath"])
        .current_dir(project_root)
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();
    if !current_hooks_path.is_empty() {
        let status = Command::new("git")
            .args(["config", "--unset", "core.hooksPath"])
            .current_dir(project_root)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        if let Ok(s) = status {
            if s.success() {
                println!("cargo:warning=husky-rs: Cleared core.hooksPath (was \"{current_hooks_path}\")");
            }
        }
    }

    // 4. Install prek hooks natively into `.git/hooks/`.
    let installation = Command::new("prek")
        .arg("install")
        .arg("--git-dir")
        .arg(project_root.join(".git"))
        .current_dir(project_root)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .output();
    ensure_prek_command_succeeds("prek install --git-dir .git", installation, config_path)?;

    println!("cargo:warning=husky-rs: prek hooks active (native mode — .git/hooks/)");
    Ok(())
}

fn ensure_prek_command_succeeds(command: &str, result: io::Result<Output>, config_path: &Path) -> Result<()> {
    match result {
        Ok(output) if output.status.success() => Ok(()),
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let details = stderr.trim();
            let message = if details.is_empty() {
                format!(
                    "{command} failed with status {} (config: {})",
                    output.status,
                    config_path.display()
                )
            } else {
                format!(
                    "{command} failed with status {} (config: {}): {details}",
                    output.status,
                    config_path.display()
                )
            };
            Err(HuskyError::PrekInstallFailed(message))
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => Err(HuskyError::PrekInstallFailed(format!(
            "{} found but prek is not installed; run `cargo install prek`",
            config_path.display()
        ))),
        Err(error) => Err(HuskyError::PrekInstallFailed(format!(
            "failed to execute prek: {error}"
        ))),
    }
}

/// Mode 1: standalone — set core.hooksPath to .husky.
fn install_standalone_mode(project_root: &Path) -> Result<()> {
    let user_hooks_dir = project_root.join(HUSKY_DIR);

    if !user_hooks_dir.exists() {
        return Ok(());
    }

    let current_hooks_path = Command::new("git")
        .args(["config", "core.hooksPath"])
        .current_dir(project_root)
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();

    if current_hooks_path != ".husky" {
        let status = Command::new("git")
            .args(["config", "core.hooksPath", ".husky"])
            .current_dir(project_root)
            .status()
            .map_err(|e| {
                if e.kind() == io::ErrorKind::NotFound {
                    HuskyError::GitConfigFailed("git command not found".to_string())
                } else {
                    HuskyError::Io(e)
                }
            })?;

        if !status.success() {
            return Err(HuskyError::GitConfigFailed(
                "git config core.hooksPath .husky failed".to_string(),
            ));
        }
        println!("cargo:warning=husky-rs: Configured core.hooksPath to .husky");
    }

    #[cfg(unix)]
    {
        match fs::read_dir(&user_hooks_dir) {
            Ok(entries) => {
                for entry in entries {
                    let entry = entry?;
                    let path = entry.path();
                    if path.is_file() {
                        use std::os::unix::fs::PermissionsExt;
                        let mut perms = fs::metadata(&path)?.permissions();
                        if perms.mode() & 0o111 == 0 {
                            perms.set_mode(perms.mode() | 0o111);
                            fs::set_permissions(&path, perms)?;
                        }
                    }
                }
            }
            Err(e) if e.to_string().contains("not a directory") || e.to_string().contains("Not a directory") => {
                return Err(HuskyError::Io(io::Error::new(
                    e.kind(),
                    format!(
                        "{} exists but is not a directory; remove it or replace it with a directory",
                        user_hooks_dir.display()
                    ),
                )));
            }
            Err(e) => return Err(HuskyError::Io(e)),
        }
    }

    Ok(())
}

fn find_git_dir() -> Result<PathBuf> {
    let start_dir = env::var("OUT_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| env::current_dir().expect("Failed to get current directory"));

    find_git_dir_from_path(&start_dir).ok_or_else(|| HuskyError::GitDirNotFound(start_dir.display().to_string()))
}

fn find_git_dir_from_path(start_path: &Path) -> Option<PathBuf> {
    start_path.ancestors().find_map(|path| {
        let git_entry = path.join(".git");
        // Keep the .git file path for worktrees/submodules so its parent remains the project root.
        if git_entry.is_dir() || (git_entry.is_file() && is_valid_git_file(&git_entry)) {
            Some(git_entry)
        } else {
            None
        }
    })
}

fn is_valid_git_file(git_file: &Path) -> bool {
    let parent = git_file.parent().unwrap_or(Path::new("."));
    fs::read_to_string(git_file)
        .ok()
        .and_then(|content| {
            let line = content.trim_end_matches(['\n', '\r']);
            line.strip_prefix("gitdir: ").map(PathBuf::from)
        })
        .map(|resolved| parent.join(resolved).is_dir())
        .unwrap_or(false)
}
