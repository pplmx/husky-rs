use std::env;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

#[derive(Debug)]
enum HuskyError {
    GitDirNotFound(String),
    Io(io::Error),
    Env(env::VarError),
    EmptyUserHook(PathBuf),
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
            HuskyError::EmptyUserHook(path) => {
                write!(f, "User hook script is empty: '{}'", path.display())
            }
        }
    }
}

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
const HUSKY_HOOKS_DIR: &str = "hooks";
const VALID_HOOK_NAMES: [&str; 3] = ["pre-commit", "commit-msg", "pre-push"];
const HUSKY_HEADER: &str = "This hook was set by husky-rs";
const SHEBANGS: [&str; 8] = [
    "#!/bin/sh",
    "#!/usr/bin/env sh",
    "#!/usr/bin/env bash",
    "#!/usr/bin/env python",
    "#!/usr/bin/env python3",
    "#!/usr/bin/env ruby",
    "#!/usr/bin/env node",
    "#!/usr/bin/env perl",
];

fn main() -> Result<()> {
    if env::var_os("NO_HUSKY_HOOKS").is_some() {
        println!("NO_HUSKY_HOOKS is set, skipping hook installation");
        return Ok(());
    }

    install_hooks().or_else(|error| {
        eprintln!("Error during hook installation: {}", error);
        matches!(error, HuskyError::GitDirNotFound(_))
            .then_some(())
            .ok_or(error)
    })
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
        && VALID_HOOK_NAMES
            .iter()
            .any(|&name| entry.file_name() == name)
}

fn install_hook(src: &Path, dst_dir: &Path) -> Result<()> {
    let dst = dst_dir.join(src.file_name().unwrap());

    let content = read_file_lines(src)?;
    if content.is_empty() {
        return Err(HuskyError::EmptyUserHook(src.to_owned()));
    }

    let content_with_header = add_husky_header(content);
    write_executable_file(&dst, &content_with_header)
}

fn read_file_lines(path: &Path) -> Result<Vec<String>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    let mut lines: Vec<String> = reader.lines().collect::<io::Result<_>>()?;

    // Remove leading empty lines
    while lines.first().map_or(false, |line| line.trim().is_empty()) {
        lines.remove(0);
    }

    // Remove trailing empty lines
    while lines.last().map_or(false, |line| line.trim().is_empty()) {
        lines.pop();
    }

    // Ensure the last line is empty if no other lines exist
    if lines.is_empty() || !lines.last().map_or(false, |line| line.trim().is_empty()) {
        lines.push(String::new());
    }

    Ok(lines)
}

fn add_husky_header(mut content: Vec<String>) -> Vec<String> {
    let shebang = content
        .first()
        .filter(|line| SHEBANGS.contains(&line.trim()))
        .map(|line| line.trim().to_string())
        .unwrap_or_else(|| "#!/usr/bin/env bash".to_string());

    content = content
        .into_iter()
        .skip_while(|line| SHEBANGS.contains(&line.trim()) || line.trim().is_empty())
        .collect();

    let header = format!(
        "{}
#
# {}
# v{}: {}
#
",
        shebang,
        HUSKY_HEADER,
        env!("CARGO_PKG_VERSION"),
        env!("CARGO_PKG_HOMEPAGE")
    );

    let mut result = vec![header];
    result.extend(content);
    result
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
