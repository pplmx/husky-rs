//! husky-rs CLI tool
//!
//! Optional command-line interface for managing Git hooks.
//! Install with: cargo install husky-rs --features=cli

use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::process;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_help();
        process::exit(0);
    }

    let result = match args[1].as_str() {
        "init" => init_husky(),
        "add" => {
            if args.len() < 3 {
                eprintln!("Error: 'add' command requires a hook name");
                eprintln!("Usage: husky add <hook-name>");
                process::exit(1);
            }
            add_hook(&args[2])
        }
        "list" => list_hooks(),
        "uninstall" => uninstall_husky(),
        "version" | "-v" | "--version" => {
            println!("husky-rs v{}", VERSION);
            Ok(())
        }
        "help" | "-h" | "--help" => {
            print_help();
            Ok(())
        }
        _ => {
            eprintln!("Unknown command: {}", args[1]);
            print_help();
            process::exit(1);
        }
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}

fn print_help() {
    println!("husky-rs v{}", VERSION);
    println!("Git hooks management for Rust projects");
    println!();
    println!("USAGE:");
    println!("    husky <COMMAND>");
    println!();
    println!("COMMANDS:");
    println!("    init              Create .husky directory");
    println!("    add <hook-name>   Create a new hook from template");
    println!("    list              List all hooks in .husky");
    println!("    uninstall         Unset core.hooksPath");
    println!("    help              Print this help message");
    println!("    version           Print version information");
    println!();
    println!("EXAMPLES:");
    println!("    husky init");
    println!("    husky add pre-commit");
    println!("    husky add commit-msg");
    println!("    husky list");
    println!();
    println!("For more information, visit: https://github.com/pplmx/husky-rs");
}

fn init_husky() -> io::Result<()> {
    let hooks_dir = Path::new(".husky");

    if hooks_dir.exists() {
        println!("✓ .husky already exists");
        return Ok(());
    }

    fs::create_dir_all(&hooks_dir)?;
    println!("✓ Created .husky directory");

    let status = process::Command::new("git")
        .args(["config", "core.hooksPath", ".husky"])
        .status();

    match status {
        Ok(s) if s.success() => println!("✓ Configured core.hooksPath to .husky"),
        Ok(_) => println!("⚠ Failed to configure core.hooksPath via git config"),
        Err(_) => println!("⚠ Git command not found, skipping configuration"),
    }

    let internal_dir = hooks_dir.join("_");
    if !internal_dir.exists() {
        fs::create_dir_all(&internal_dir)?;
    }

    let husky_sh = internal_dir.join("husky.sh");
    if !husky_sh.exists() {
        let content = r#"#!/bin/sh
if [ -z "$husky_skip_init" ]; then
  if [ "$HUSKY" = "0" ]; then
    exit 0
  fi
  readonly husky_skip_init=1
fi
"#;
        fs::write(&husky_sh, content)?;
    }

    println!();
    println!("Next steps:");
    println!("  1. Add husky-rs to your Cargo.toml:");
    println!("     cargo add husky-rs");
    println!("  2. Create hooks:");
    println!("     husky add pre-commit");
    println!("  3. Build your project to install hooks:");
    println!("     cargo build");

    Ok(())
}

fn add_hook(hook_name: &str) -> io::Result<()> {
    // Validate hook name
    if !husky_rs::is_valid_hook_name(hook_name) {
        eprintln!("Warning: '{}' is not a standard Git hook name", hook_name);
        eprintln!();
        eprintln!("Standard hooks include:");
        const HOOKS_PER_ROW: usize = 3;
        const HOOK_DISPLAY_WIDTH: usize = 25;
        for (i, hook) in husky_rs::SUPPORTED_HOOKS.iter().enumerate() {
            if i % HOOKS_PER_ROW == 0 {
                print!("  ");
            }
            print!("{:<width$}", hook, width = HOOK_DISPLAY_WIDTH);
            if (i + 1) % HOOKS_PER_ROW == 0 {
                println!();
            }
        }
        println!();
        print!("Continue anyway? [y/N]: ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Cancelled");
            return Ok(());
        }
    }

    let hooks_dir = Path::new(".husky");
    if !hooks_dir.exists() {
        fs::create_dir_all(&hooks_dir)?;
        println!("✓ Created .husky directory");
    }

    let hook_path = hooks_dir.join(hook_name);
    if hook_path.exists() {
        eprintln!("Error: Hook '{}' already exists at:", hook_name);
        eprintln!("  {}", hook_path.display());
        eprintln!();
        eprintln!("Edit the file directly or remove it first.");
        process::exit(1);
    }

    let template = get_hook_template(hook_name);
    fs::write(&hook_path, template)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&hook_path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&hook_path, perms)?;
    }

    println!("✓ Created hook: {}", hook_name);
    println!("  {}", hook_path.display());
    println!();
    println!("Edit the hook file to customize it, then run:");
    println!("  cargo build");

    Ok(())
}

fn list_hooks() -> io::Result<()> {
    let hooks_dir = Path::new(".husky");

    if !hooks_dir.exists() {
        println!("No .husky directory found.");
        println!("Run 'husky init' to create it.");
        return Ok(());
    }

    let entries: Vec<_> = fs::read_dir(&hooks_dir)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().is_file())
        .collect();

    if entries.is_empty() {
        println!("No hooks found in .husky/");
        println!("Run 'husky add <hook-name>' to create one.");
        return Ok(());
    }

    println!("Hooks in .husky/:");
    for entry in entries {
        let name = entry.file_name();

        let is_valid = husky_rs::is_valid_hook_name(&name.to_string_lossy());
        let indicator = if is_valid { "✓" } else { "⚠" };

        println!("  {} {}", indicator, name.to_string_lossy());

        if !is_valid {
            println!("     (not a standard Git hook name)");
        }
    }

    Ok(())
}

fn uninstall_husky() -> io::Result<()> {
    let status = process::Command::new("git")
        .args(["config", "--unset", "core.hooksPath"])
        .status()?;

    if status.success() {
        println!("✓ Unset core.hooksPath");
    } else {
        println!("i core.hooksPath was not set or failed to unset");
    }

    Ok(())
}

fn get_hook_template(hook_name: &str) -> String {
    match hook_name {
        "pre-commit" => {
            r#"#!/bin/sh
# . "$(dirname "$0")/_/husky.sh"

echo "Running pre-commit checks..."

# Run tests
cargo test --quiet || {
    echo "❌ Tests failed!"
    exit 1
}

# Check formatting
cargo fmt --check || {
    echo "❌ Code is not formatted. Run: cargo fmt"
    exit 1
}

echo "✓ Pre-commit checks passed"
"#
        }
        "commit-msg" => {
            r#"#!/bin/sh
# . "$(dirname "$0")/_/husky.sh"

commit_msg_file="$1"
commit_msg=$(cat "$commit_msg_file")

# Example: Check minimum message length
if [ ${#commit_msg} -lt 10 ]; then
    echo "❌ Commit message too short (minimum 10 characters)"
    exit 1
fi

# Example: Check for conventional commits format
# Uncomment the following to enforce conventional commits:
# if ! echo "$commit_msg" | grep -qE "^(feat|fix|docs|style|refactor|test|chore):"; then
#     echo "❌ Commit message must follow conventional commits format"
#     echo "   Examples: feat: add feature, fix: bug fix"
#     exit 1
# fi

echo "✓ Commit message valid"
"#
        }
        "pre-push" => {
            r#"#!/bin/sh
# . "$(dirname "$0")/_/husky.sh"

echo "Running pre-push checks..."

# Run comprehensive checks before pushing
set -e

echo "1/3 Checking format..."
cargo fmt --check

echo "2/3 Running linter..."
cargo clippy --all-targets --all-features -- -D warnings

echo "3/3 Running tests..."
cargo test

echo "✅ All checks passed! Pushing..."
"#
        }
        _ => {
            r#"#!/bin/sh
# . "$(dirname "$0")/_/husky.sh"

echo "Running hook..."

# Add your hook logic here
# Exit with non-zero status to abort the Git operation

echo "✓ Hook completed"
"#
        }
    }
    .to_string()
}
