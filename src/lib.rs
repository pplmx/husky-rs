//! # husky-rs
//!
//! Git hooks management for Rust projects.
//!
//! ## Zero-Config Usage
//!
//! Just add `husky-rs` as a dependency and create hooks in `.husky/hooks/`.
//! The hooks will be automatically installed when you build your project:
//!
//! ```sh
//! cargo add husky-rs
//! mkdir -p .husky/hooks
//! echo '#!/bin/sh\necho "Running pre-commit hook"' > .husky/hooks/pre-commit
//! cargo build  # Hooks installed automatically!
//! ```
//!
//! That's it! No configuration needed, no manual installation steps.
//!
//! ## How It Works
//!
//! `husky-rs` uses a build script (`build.rs`) to automatically copy hooks from
//! `.husky/hooks/` to `.git/hooks/` during the build process. This means:
//!
//! - Works with both `dependencies` and `dev-dependencies`
//! - Hooks are installed automatically on `cargo build` or `cargo test`
//! - Changes to hooks trigger automatic reinstallation (no `cargo clean` needed!)
//! - Cross-platform support (Unix-like systems and Windows)
//!
//! ## Skipping Hook Installation
//!
//! Set the `NO_HUSKY_HOOKS` environment variable to skip hook installation:
//!
//! ```sh
//! NO_HUSKY_HOOKS=1 cargo build
//! ```
//!
//! ## Optional Utilities
//!
//! The functions and constants below are **completely optional** - you don't need
//! them for basic usage. They're provided for advanced use cases like custom
//! tooling or programmatic hook management.

use std::path::{Path, PathBuf};

/// The default directory name for husky configuration (`.husky`)
pub const HUSKY_DIR: &str = ".husky";

/// The default hooks subdirectory name (`hooks`)
pub const HUSKY_HOOKS_DIR: &str = "hooks";

/// Returns the standard husky hooks directory path for a given project root.
///
/// This is a convenience function that returns `.husky/hooks` relative to
/// the provided project root.
///
/// # Examples
///
/// ```
/// use husky_rs::hooks_dir;
/// use std::path::Path;
///
/// let project_root = Path::new("/path/to/project");
/// let hooks_path = hooks_dir(project_root);
/// assert_eq!(hooks_path, Path::new("/path/to/project/.husky/hooks"));
/// ```
pub fn hooks_dir(project_root: impl AsRef<Path>) -> PathBuf {
    project_root.as_ref().join(HUSKY_DIR).join(HUSKY_HOOKS_DIR)
}

/// Checks if hook installation should be skipped based on the `NO_HUSKY_HOOKS`
/// environment variable.
///
/// Returns `true` if the environment variable is set (regardless of value),
/// `false` otherwise.
///
/// # Examples
///
/// ```
/// use husky_rs::should_skip_installation;
///
/// // In your build script or tool:
/// if should_skip_installation() {
///     println!("Skipping hook installation");
///     return;
/// }
/// ```
pub fn should_skip_installation() -> bool {
    std::env::var_os("NO_HUSKY_HOOKS").is_some()
}

/// List of all Git hooks supported by husky-rs.
///
/// NOTE: Keep in sync with `VALID_HOOK_NAMES` in `build.rs`.
///
/// This includes all standard Git hooks as documented in the
/// [Git documentation](https://git-scm.com/docs/githooks).
pub const SUPPORTED_HOOKS: &[&str] = &[
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

/// Checks if a given hook name is valid/supported by Git.
///
/// # Examples
///
/// ```
/// use husky_rs::is_valid_hook_name;
///
/// assert!(is_valid_hook_name("pre-commit"));
/// assert!(is_valid_hook_name("pre-push"));
/// assert!(!is_valid_hook_name("invalid-hook"));
/// ```
pub fn is_valid_hook_name(name: &str) -> bool {
    SUPPORTED_HOOKS.contains(&name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_hooks_dir() {
        let path = hooks_dir("/tmp/project");
        assert_eq!(path, PathBuf::from("/tmp/project/.husky/hooks"));
    }

    #[test]
    fn test_hooks_dir_with_trailing_slash() {
        let path = hooks_dir("/tmp/project/");
        assert_eq!(path, PathBuf::from("/tmp/project/.husky/hooks"));
    }

    #[test]
    fn test_hooks_dir_relative_path() {
        let path = hooks_dir(".");
        assert!(path.ends_with(".husky/hooks"));
    }

    #[test]
    fn test_is_valid_hook_name() {
        assert!(is_valid_hook_name("pre-commit"));
        assert!(is_valid_hook_name("commit-msg"));
        assert!(is_valid_hook_name("pre-push"));
        assert!(!is_valid_hook_name("not-a-hook"));
        assert!(!is_valid_hook_name(""));
    }

    #[test]
    fn test_is_valid_hook_name_all_supported() {
        // Verify all hooks in SUPPORTED_HOOKS are valid
        for hook in SUPPORTED_HOOKS {
            assert!(is_valid_hook_name(hook), "Hook '{}' should be valid", hook);
        }
    }

    #[test]
    fn test_is_valid_hook_name_case_sensitive() {
        assert!(is_valid_hook_name("pre-commit"));
        assert!(!is_valid_hook_name("PRE-COMMIT"));
        assert!(!is_valid_hook_name("Pre-Commit"));
    }

    #[test]
    fn test_supported_hooks_count() {
        assert_eq!(SUPPORTED_HOOKS.len(), 27);
    }

    #[test]
    fn test_supported_hooks_no_duplicates() {
        use std::collections::HashSet;
        let set: HashSet<_> = SUPPORTED_HOOKS.iter().collect();
        assert_eq!(
            set.len(),
            SUPPORTED_HOOKS.len(),
            "SUPPORTED_HOOKS should not contain duplicates"
        );
    }

    use std::sync::Mutex;
    use std::sync::OnceLock;

    // Mutex to properly serialize tests that modify environment variables.
    // This prevents race conditions when running `cargo test` which runs tests in parallel by default.
    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    #[test]
    fn test_should_skip_installation_not_set() {
        let _guard = env_lock().lock().unwrap();
        // Ensure clean state
        env::remove_var("NO_HUSKY_HOOKS");
        // Give it a moment to ensure var is actually removed
        assert!(
            env::var_os("NO_HUSKY_HOOKS").is_none(),
            "Env var should be None"
        );
        assert!(
            !should_skip_installation(),
            "Should not skip when env var not set"
        );
    }

    #[test]
    fn test_should_skip_installation_set_to_1() {
        let _guard = env_lock().lock().unwrap();
        env::set_var("NO_HUSKY_HOOKS", "1");
        assert!(should_skip_installation(), "Should skip when set to 1");
        env::remove_var("NO_HUSKY_HOOKS");
    }

    #[test]
    fn test_should_skip_installation_set_to_any_value() {
        let _guard = env_lock().lock().unwrap();
        env::set_var("NO_HUSKY_HOOKS", "anything");
        assert!(should_skip_installation());
        env::remove_var("NO_HUSKY_HOOKS");
    }

    #[test]
    fn test_should_skip_installation_set_to_empty() {
        let _guard = env_lock().lock().unwrap();
        env::set_var("NO_HUSKY_HOOKS", "");
        assert!(should_skip_installation(), "Empty value should still skip");
        env::remove_var("NO_HUSKY_HOOKS");
    }

    #[test]
    fn test_constants_match() {
        assert_eq!(HUSKY_DIR, ".husky");
        assert_eq!(HUSKY_HOOKS_DIR, "hooks");
    }
}
