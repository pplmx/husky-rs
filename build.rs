use std::env;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
enum HuskyError {
    #[error("Git directory not found in '{0}' or its parent directories")]
    GitDirNotFound(String),
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    #[error("Environment variable error: {0}")]
    Env(#[from] env::VarError),
    #[error("User hook script is empty: '{0}'")]
    EmptyUserHook(PathBuf),
}

type Result<T> = std::result::Result<T, HuskyError>;

const HUSKY_DIR: &str = ".husky";
const HUSKY_HOOKS_DIR: &str = "hooks";
const VALID_HOOK_NAMES: [&str; 3] = ["pre-commit", "commit-msg", "pre-push"];

fn main() -> Result<()> {
    if env::var_os("CARGO_HUSKY_DONT_INSTALL_HOOKS").is_some() {
        println!("CARGO_HUSKY_DONT_INSTALL_HOOKS is set, skipping hook installation");
        return Ok(());
    }

    install_hooks().or_else(|e| match e {
        HuskyError::GitDirNotFound(_) => {
            eprintln!("Warning: Git directory not found. Skipping husky hook installation.");
            Ok(())
        }
        e => Err(e),
    })
}

fn install_hooks() -> Result<()> {
    let git_dir = find_git_dir()?;
    let project_root = git_dir
        .parent()
        .ok_or_else(|| HuskyError::GitDirNotFound(git_dir.display().to_string()))?;
    let user_hooks_dir = project_root.join(HUSKY_DIR).join(HUSKY_HOOKS_DIR);
    let git_hooks_dir = git_dir.join("hooks");

    if !user_hooks_dir.is_dir() {
        eprintln!("Warning: .husky/hooks directory not found. Skipping husky hook installation.");
        return Ok(());
    }

    fs::create_dir_all(&git_hooks_dir)?;

    for entry in fs::read_dir(&user_hooks_dir)? {
        let entry = entry?;
        if is_valid_hook_file(&entry) {
            install_hook(&entry.path(), &git_hooks_dir)?;
        }
    }

    Ok(())
}

fn find_git_dir() -> Result<PathBuf> {
    // First, try to use OUT_DIR if available
    if let Ok(out_dir) = env::var("OUT_DIR") {
        let out_path = PathBuf::from(out_dir);
        if let Some(git_dir) = find_git_dir_from_path(&out_path) {
            return Ok(git_dir);
        }
    }

    // If OUT_DIR is not set or doesn't contain a .git directory, fall back to the current directory
    let current_dir = env::current_dir()?;
    find_git_dir_from_path(&current_dir)
        .ok_or_else(|| HuskyError::GitDirNotFound(current_dir.display().to_string()))
}

fn find_git_dir_from_path(start_path: &Path) -> Option<PathBuf> {
    let mut current_dir = start_path.to_path_buf();
    loop {
        let git_dir = current_dir.join(".git");
        if git_dir.is_dir() {
            return Some(git_dir);
        }
        if git_dir.is_file() {
            return read_git_submodule(&git_dir).ok();
        }
        if !current_dir.pop() {
            return None;
        }
    }
}

fn read_git_submodule(git_file: &Path) -> Result<PathBuf> {
    let content = fs::read_to_string(git_file)?;
    let git_dir = PathBuf::from(content.trim_end_matches(&['\n', '\r']));
    if !git_dir.is_dir() {
        return Err(HuskyError::GitDirNotFound(git_dir.display().to_string()));
    }
    Ok(git_dir)
}

fn is_valid_hook_file(entry: &fs::DirEntry) -> bool {
    entry.file_type().map(|ft| ft.is_file()).unwrap_or(false)
        && is_executable_file(entry)
        && VALID_HOOK_NAMES.contains(&entry.file_name().to_str().unwrap_or(""))
}

#[cfg(unix)]
fn is_executable_file(entry: &fs::DirEntry) -> bool {
    use std::os::unix::fs::PermissionsExt;
    entry
        .metadata()
        .map(|m| m.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn is_executable_file(_entry: &fs::DirEntry) -> bool {
    true // On Windows, we consider all files as potentially executable
}

fn install_hook(src: &Path, dst_dir: &Path) -> Result<()> {
    let dst = dst_dir.join(src.file_name().unwrap());
    if hook_exists(&dst) {
        return Ok(());
    }

    let mut content = read_file_lines(src)?;
    if content.is_empty() {
        return Err(HuskyError::EmptyUserHook(src.to_owned()));
    }

    add_husky_header(&mut content);
    write_executable_file(&dst, &content)
}

fn hook_exists(hook: &Path) -> bool {
    fs::read_to_string(hook)
        .map(|content| {
            content
                .lines()
                .any(|line| line.contains("This hook was set by husky-rs"))
        })
        .unwrap_or(false)
}

fn read_file_lines(path: &Path) -> Result<Vec<String>> {
    let file = File::open(path)?;
    BufReader::new(file)
        .lines()
        .collect::<io::Result<_>>()
        .map_err(HuskyError::from)
}

fn add_husky_header(content: &mut Vec<String>) {
    if !content[0].starts_with("#!") {
        content.insert(0, "#".to_string());
    }
    content.insert(1, "#".to_string());
    content.insert(
        2,
        format!(
            "# This hook was set by husky-rs v{}: {}",
            env!("CARGO_PKG_VERSION"),
            env!("CARGO_PKG_HOMEPAGE")
        ),
    );
}

fn write_executable_file(path: &Path, content: &[String]) -> Result<()> {
    let mut file = create_executable_file(path)?;
    for line in content {
        writeln!(file, "{}", line)?;
    }
    Ok(())
}

#[cfg(unix)]
fn create_executable_file(path: &Path) -> io::Result<File> {
    use std::os::unix::fs::OpenOptionsExt;
    OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o755)
        .open(path)
}

#[cfg(not(unix))]
fn create_executable_file(path: &Path) -> io::Result<File> {
    File::create(path)
}
