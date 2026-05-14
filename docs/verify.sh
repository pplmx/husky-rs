#!/usr/bin/env bash
# End-to-end verification for husky-rs hook installation.
#
# Two scenarios:
#   1. Standard: .husky/ exists before `cargo test` → hooksPath set → commit triggers
#   2. Lazy:     cargo test before .husky/ created → second cargo test re-runs
#                build script → hooksPath set → commit triggers
#
# Usage:
#   ./scripts/verify.sh              # auto-detect husky-rs repo root
#   ./scripts/verify.sh /path/to/repo

set -e

if [ -n "$1" ]; then
    HUSKY_RS_PATH="$1"
else
    HUSKY_RS_PATH="$(cd "$(dirname "$0")/.." && pwd)"
fi

if [ ! -f "$HUSKY_RS_PATH/Cargo.toml" ]; then
    echo "Error: $HUSKY_RS_PATH is not a valid husky-rs repo (no Cargo.toml)"
    echo "Usage: $0 [/path/to/husky-rs]"
    exit 1
fi

TMPDIR="/tmp/husky-rs-verify"
rm -rf "$TMPDIR"

run_scenario() {
    local name="$1"
    local dir="$TMPDIR/$name"
    mkdir -p "$dir" && cd "$dir"

    git init -q && git config user.email "t@t.com" && git config user.name "T"
    cargo init --bin -q
    cargo add --dev --path "$HUSKY_RS_PATH" husky-rs -q
}

# ============================================================
# Scenario 1: Standard flow
# ============================================================
echo "=============================================="
echo " Scenario 1: Standard (.husky/ before cargo test)"
echo "=============================================="

run_scenario scenario1

mkdir -p .husky
cat > .husky/pre-commit << 'HOOK'
#!/bin/sh
echo ">>> PRE-COMMIT HOOK TRIGGERED <<<"
exit 0
HOOK
chmod +x .husky/pre-commit

echo "--- cargo test ---"
cargo test -q 2>&1

echo "--- core.hooksPath (expect .husky) ---"
git config core.hooksPath

echo "--- git commit (expect hook output above) ---"
touch foo && git add foo && git commit -qm "test: scenario1"

# ============================================================
# Scenario 2: Lazy flow
# ============================================================
echo ""
echo "=============================================="
echo " Scenario 2: Lazy (.husky/ AFTER first cargo test)"
echo "=============================================="

run_scenario scenario2

echo "--- cargo test BEFORE .husky/ exists ---"
cargo test -q 2>&1
echo "--- core.hooksPath (expect empty) ---"
git config core.hooksPath || echo "(not set)"

mkdir -p .husky
cat > .husky/pre-commit << 'HOOK'
#!/bin/sh
echo ">>> LAZY HOOK FIRED <<<"
exit 0
HOOK
chmod +x .husky/pre-commit

echo "--- cargo test AGAIN (must recompile, not Fresh) ---"
cargo test -q 2>&1
echo "--- core.hooksPath (expect .husky) ---"
git config core.hooksPath

echo "--- git commit (expect LAZY HOOK FIRED) ---"
touch bar && git add bar && git commit -qm "test: scenario2"

rm -rf "$TMPDIR"
echo ""
echo "=============================================="
echo " PASS: both scenarios verified"
echo "=============================================="
