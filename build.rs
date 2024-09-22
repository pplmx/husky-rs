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
const HUSKY_HEADER: &str = "This hook was set by husky-rs";

fn main() -> Result<()> {
    if skip_hook_installation() {
        return Ok(());
    }

    install_hooks().or_else(handle_installation_error)
}

fn skip_hook_installation() -> bool {
    if env::var_os("CARGO_HUSKY_DONT_INSTALL_HOOKS").is_some() {
        println!("CARGO_HUSKY_DONT_INSTALL_HOOKS is set, skipping hook installation");
        return true;
    }
    false
}

fn handle_installation_error(error: HuskyError) -> Result<()> {
    eprintln!("Error during hook installation: {}", error);
    if let HuskyError::GitDirNotFound(_) = error {
        Ok(()) // Ignore missing git directories
    } else {
        Err(error)
    }
}

fn install_hooks() -> Result<()> {
    let git_dir = find_git_dir()?;
    let project_root = git_dir
        .parent()
        .ok_or_else(|| HuskyError::GitDirNotFound(git_dir.display().to_string()))?;
    let user_hooks_dir = project_root.join(HUSKY_DIR).join(HUSKY_HOOKS_DIR);
    let git_hooks_dir = git_dir.join("hooks");

    if !user_hooks_dir.exists() {
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
    let start_dir = env::var("OUT_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| env::current_dir().expect("Failed to get current directory"));
    find_git_dir_from_path(&start_dir)
        .ok_or_else(|| HuskyError::GitDirNotFound(start_dir.display().to_string()))
}

fn find_git_dir_from_path(start_path: &Path) -> Option<PathBuf> {
    start_path.ancestors().find_map(|path| {
        let git_dir = path.join(".git");
        if git_dir.is_dir() {
            Some(git_dir)
        } else if git_dir.is_file() {
            read_git_submodule(&git_dir).ok()
        } else {
            None
        }
    })
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

    let content = read_file_lines(src)?;
    if content.is_empty() {
        return Err(HuskyError::EmptyUserHook(src.to_owned()));
    }

    let content_with_header = add_husky_header(content);
    write_executable_file(&dst, &content_with_header)
}

fn hook_exists(hook: &Path) -> bool {
    fs::read_to_string(hook)
        .map(|content| content.contains(HUSKY_HEADER))
        .unwrap_or(false)
}

fn read_file_lines(path: &Path) -> Result<Vec<String>> {
    let file = File::open(path)?;
    BufReader::new(file)
        .lines()
        .collect::<io::Result<_>>()
        .map_err(HuskyError::from)
}

fn add_husky_header(mut content: Vec<String>) -> Vec<String> {
    let header = r#"
#!/bin/sh
# This hook was set by husky-rs
# v{version}: {homepage}
"#;
    let formatted_header = header
        .replace("{version}", env!("CARGO_PKG_VERSION"))
        .replace("{homepage}", env!("CARGO_PKG_HOMEPAGE"));

    content.insert(0, formatted_header);
    content
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
    std::fs::OpenOptions::new()
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
