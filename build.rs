//! Build script for husky-rs
//!
//! This build script automatically configures Git to use hooks from `.husky/`
//! by setting `core.hooksPath`.
//!
//! ## How it works
//!
//! 1. Checks for the `NO_HUSKY_HOOKS` environment variable to skip installation
//! 2. Locates the `.git` directory
//! 3. Checks if `.husky/` exists
//! 4. Sets `git config core.hooksPath .husky`
//! 5. Ensures files in `.husky/` are executable (Unix-like systems)

use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug)]
enum HuskyError {
    GitDirNotFound(String),
    Io(io::Error),
    Env(env::VarError),
    GitConfigFailed(String),
}

impl std::fmt::Display for HuskyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HuskyError::GitDirNotFound(path) => write!(
                f,
                "Git directory not found in '{}' or its parent directories",
                path
            ),
            HuskyError::Io(err) => write!(f, "IO error: {}", err),
            HuskyError::Env(err) => write!(f, "Environment variable error: {}", err),
            HuskyError::GitConfigFailed(err) => write!(f, "Git config failed: {}", err),
        }
    }
}

impl std::error::Error for HuskyError {}

impl From<io::Error> for HuskyError {
    fn from(err: io::Error) -> Self {
        HuskyError::Io(err)
    }
}

impl From<env::VarError> for HuskyError {
    fn from(err: env::VarError) -> Self {
        HuskyError::Env(err)
    }
}

type Result<T> = std::result::Result<T, HuskyError>;

const HUSKY_DIR: &str = ".husky";

fn main() -> Result<()> {
    println!("cargo:rerun-if-env-changed=NO_HUSKY_HOOKS");

    if env::var_os("NO_HUSKY_HOOKS").is_some() {
        return Ok(());
    }

    install_hooks().or_else(|error| {
        match &error {
            HuskyError::GitDirNotFound(path) => {
                eprintln!(
                    "husky-rs: Unable to find .git directory starting from: {}",
                    path
                );
            }
            HuskyError::GitConfigFailed(e) => {
                eprintln!("husky-rs: Failed to set git config: {}", e);
            }
            HuskyError::Io(e) => {
                eprintln!("husky-rs: I/O error during hook installation: {}", e);
            }
            HuskyError::Env(e) => {
                eprintln!("husky-rs: Environment variable error: {}", e);
            }
        }

        // Tolerate GitDirNotFound and GitConfigFailed;
        // the user can always run `husky init` manually later.
        matches!(
            error,
            HuskyError::GitDirNotFound(_) | HuskyError::GitConfigFailed(_)
        )
        .then_some(())
        .ok_or(error)
    })
}

fn install_hooks() -> Result<()> {
    let git_dir = find_git_dir()?;
    let project_root = git_dir
        .parent()
        .ok_or_else(|| HuskyError::GitDirNotFound(git_dir.display().to_string()))?;
    let user_hooks_dir = project_root.join(HUSKY_DIR);

    // Tell Cargo to re-run when user's .husky directory changes
    println!("cargo:rerun-if-changed={}", user_hooks_dir.display());

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

    // Ensure all files in .husky are executable on Unix
    #[cfg(unix)]
    {
        for entry in fs::read_dir(&user_hooks_dir)? {
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

    Ok(())
}

fn find_git_dir() -> Result<PathBuf> {
    let start_dir = env::var("OUT_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| env::current_dir().expect("Failed to get current directory"));

    find_git_dir_from_path(&start_dir)
        .ok_or_else(|| HuskyError::GitDirNotFound(start_dir.display().to_string()))
}

fn find_git_dir_from_path(start_path: &Path) -> Option<PathBuf> {
    start_path.ancestors().find_map(|path| {
        let git_entry = path.join(".git");
        if git_entry.is_dir() {
            Some(git_entry)
        } else if git_entry.is_file() && is_valid_git_file(&git_entry) {
            // For submodules/worktrees, .git is a file; return the file path
            // so that .parent() gives the correct project root.
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
