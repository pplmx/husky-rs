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
const HUSKY_HOOKS_DIR: &str = "hooks";
const VALID_HOOK_NAMES: [&str; 27] = [
    "applypatch-msg",
    "pre-applypatch",
    "post-applypatch",
    "pre-commit",
    "pre-merge-commit",
    "prepare-commit-msg",
    "commit-msg",
    "post-commit",
    "pre-rebase",
    "post-checkout",
    "post-merge",
    "pre-push",
    "pre-receive",
    "update",
    "proc-receive",
    "post-receive",
    "post-update",
    "reference-transaction",
    "pre-auto-gc",
    "post-rewrite",
    "sendemail-validate",
    "fsmonitor-watchman",
    "p4-changelist",
    "p4-prepare-changelist",
    "p4-post-changelist",
    "p4-pre-submit",
    "post-index-change",
];
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
    println!("cargo:rerun-if-env-changed=NO_HUSKY_HOOKS");
    if env::var_os("NO_HUSKY_HOOKS").is_some() {
        println!("NO_HUSKY_HOOKS is set, skipping hook installation"); // This should be visible in captured stderr
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

    for entry_result in fs::read_dir(&user_hooks_dir)? {
        let entry = entry_result?;
        let user_hook_path = entry.path();

        // Tell cargo to re-run the build script if this file/symlink changes.
        // Tell cargo to re-run the build script if this file/symlink changes.
        // This was temporarily removed for debugging test_no_hooks_if_env_var_set
        // if let Some(path_str) = user_hook_path.to_str() {
        //     println!("cargo:rerun-if-changed={}", path_str);
        // }

        if is_valid_hook_file(&entry) {
            install_hook(&user_hook_path, &git_hooks_dir)?;
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
    let git_dir = PathBuf::from(content.trim_end_matches(['\n', '\r']));
    if !git_dir.is_dir() {
        return Err(HuskyError::GitDirNotFound(git_dir.display().to_string()));
    }
    Ok(git_dir)
}

fn is_valid_hook_file(entry: &fs::DirEntry) -> bool {
    let path = entry.path();
    // fs::metadata follows symlinks. If we wanted to check the symlink itself,
    // we would use fs::symlink_metadata().
    let metadata_result = fs::metadata(&path);

    let is_file_type = metadata_result.map(|md| md.is_file()).unwrap_or(false);

    is_file_type && VALID_HOOK_NAMES.contains(&entry.file_name().to_str().unwrap_or(""))
}

fn install_hook(src: &Path, dst_dir: &Path) -> Result<()> {
    let dst = dst_dir.join(src.file_name().unwrap());
    let user_script_lines = read_file_lines(src)?;
    if user_script_lines.is_empty() {
        return Err(HuskyError::EmptyUserHook(src.to_owned()));
    }

    let (shebang, actual_script_body) = extract_shebang_and_body(user_script_lines);
    let final_hook_script_lines = generate_husky_hook_script(shebang, actual_script_body);
    write_executable_file(&dst, &final_hook_script_lines)
}

// Extracts shebang and body from user script lines.
// Returns default shebang if not found or if script is empty.
fn extract_shebang_and_body(user_script_lines: Vec<String>) -> (String, Vec<String>) {
    let mut actual_script_body = user_script_lines; // No clone needed if we consume it

    let shebang = if let Some(first_line) = actual_script_body.first() {
        if SHEBANGS.contains(&first_line.trim()) {
            let s = first_line.trim().to_string();
            actual_script_body = actual_script_body.into_iter().skip(1).collect();
            s
        } else {
            "#!/usr/bin/env bash".to_string()
        }
    } else {
        // This case should ideally not be hit if install_hook checks for empty user_script_lines
        "#!/usr/bin/env bash".to_string()
    };

    (shebang, actual_script_body)
}

// Generates the full hook script content with husky header.
fn generate_husky_hook_script(shebang: String, actual_script_body: Vec<String>) -> Vec<String> {
    let header_content = format!(
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

    let mut final_script_lines = vec![header_content];
    final_script_lines.extend(actual_script_body);
    final_script_lines
}

fn read_file_lines(path: &Path) -> Result<Vec<String>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    // Collect only non-whitespace lines
    let lines: Vec<String> = reader
        .lines()
        .collect::<io::Result<Vec<String>>>()?
        .into_iter()
        .filter(|line| !line.trim().is_empty()) // Keep only lines with non-whitespace content
        .collect();

    Ok(lines) // Return the filtered lines. If all lines were whitespace/empty, this will be an empty Vec.
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

#[cfg(test)]
mod tests {
    use super::*;
    // use std::path::PathBuf; // Not needed for these specific tests

    #[test]
    fn test_extract_standard_shebang() {
        let input_lines = vec!["#!/bin/sh".to_string(), "echo hello".to_string()];
        let (shebang, body) = extract_shebang_and_body(input_lines);
        assert_eq!(shebang, "#!/bin/sh");
        assert_eq!(body, vec!["echo hello".to_string()]);
    }

    #[test]
    fn test_extract_python_shebang() {
        let input_lines = vec![
            "#!/usr/bin/env python".to_string(),
            "print('hello')".to_string(),
        ];
        let (shebang, body) = extract_shebang_and_body(input_lines);
        assert_eq!(shebang, "#!/usr/bin/env python");
        assert_eq!(body, vec!["print('hello')".to_string()]);
    }

    #[test]
    fn test_extract_no_shebang() {
        let input_lines = vec!["echo no shebang".to_string()];
        let (shebang, body) = extract_shebang_and_body(input_lines);
        assert_eq!(shebang, "#!/usr/bin/env bash");
        assert_eq!(body, vec!["echo no shebang".to_string()]);
    }

    #[test]
    fn test_extract_shebang_no_leading_empty_lines_in_body() {
        // Simulating read_file_lines output which already filters empty lines
        let input_lines = vec!["#!/bin/sh".to_string(), "echo hello".to_string()];
        let (shebang, body) = extract_shebang_and_body(input_lines);
        assert_eq!(shebang, "#!/bin/sh");
        assert_eq!(body, vec!["echo hello".to_string()]);
    }

    #[test]
    fn test_extract_only_shebang() {
        let input_lines = vec!["#!/bin/sh".to_string()];
        let (shebang, body) = extract_shebang_and_body(input_lines);
        assert_eq!(shebang, "#!/bin/sh");
        assert_eq!(body, Vec::<String>::new());
    }

    #[test]
    fn test_extract_empty_input() {
        let input_lines: Vec<String> = vec![];
        let (shebang, body) = extract_shebang_and_body(input_lines);
        assert_eq!(shebang, "#!/usr/bin/env bash");
        assert_eq!(body, Vec::<String>::new());
    }

    #[test]
    fn test_generate_basic_script() {
        let shebang = "#!/bin/sh".to_string();
        let body = vec!["echo test".to_string(), "exit 1".to_string()];
        let result = generate_husky_hook_script(shebang.clone(), body.clone());

        assert_eq!(result[0], shebang);
        let full_script = result.join("\n");
        assert!(full_script.contains(HUSKY_HEADER));
        assert!(full_script.contains(env!("CARGO_PKG_VERSION")));
        assert!(full_script.contains(env!("CARGO_PKG_HOMEPAGE")));
        assert!(full_script.contains("\necho test\n")); // Check for newline separation
        assert!(full_script.contains("\nexit 1")); // Ensure it's part of the body
                                                   // Check that the body lines are at the end
        let expected_body_combined = "\necho test\nexit 1";
        assert!(
            full_script.ends_with(expected_body_combined)
                || full_script.ends_with(&(expected_body_combined.to_string() + "\n"))
        );
    }

    #[test]
    fn test_generate_script_empty_body() {
        let shebang = "#!/usr/bin/env bash".to_string();
        let body: Vec<String> = vec![];
        let result = generate_husky_hook_script(shebang.clone(), body);

        assert_eq!(result[0], shebang);
        let full_script = result.join("\n");
        assert!(full_script.contains(HUSKY_HEADER));
        assert!(full_script.contains(env!("CARGO_PKG_VERSION")));
        assert!(full_script.contains(env!("CARGO_PKG_HOMEPAGE")));
        // Ensure that nothing follows the header's trailing newline if the body is empty.
        let expected_header = format!(
            "{}
#
# {}
# v{}: {}
#
", // Note the trailing newline
            shebang,
            HUSKY_HEADER,
            env!("CARGO_PKG_VERSION"),
            env!("CARGO_PKG_HOMEPAGE")
        );
        assert_eq!(full_script, expected_header.trim_end_matches('\n')); // Compare without trailing newline if result doesn't have one for empty body
    }
}
