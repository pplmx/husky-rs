#!/bin/sh

#━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
# Colors and Icons
#━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
MAGENTA='\033[0;35m'
NC='\033[0m'

print_header() {
    icon="$1"
    title="$2"
    echo ""
    echo "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo "${CYAN}  $icon $title${NC}"
    echo "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo ""
}

print_step() { echo "${BLUE}→${NC} $1"; }
print_success() { echo "  ${GREEN}✓${NC} $1"; }
print_error() { echo "  ${RED}✗${NC} $1"; }
print_warning() { echo "  ${YELLOW}⚠${NC} $1"; }
print_info() { echo "  ${CYAN}ℹ${NC}  $1"; }

# run_check <step_name> <command> <error_msg> [hint] [non_blocking]
run_check() {
    step_name="$1"
    command="$2"
    error_msg="$3"
    hint="$4"
    non_blocking="$5"

    print_step "$step_name"

    # Temporary file for output
    tmp_file=$(mktemp)

    if eval "$command" > "$tmp_file" 2>&1; then
        print_success "Passed"
        # Optional: Print summary if it's a test
        if grep -q "test result:" "$tmp_file"; then
             grep "test result:" "$tmp_file" | while read -r line; do
                print_info "$line"
            done
        fi
        rm -f "$tmp_file"
        return 0
    else
        echo ""
        if [ "$non_blocking" = "true" ]; then
            print_warning "$error_msg (non-blocking)"
        else
            print_error "$error_msg"
        fi

        if [ -n "$hint" ]; then
            print_info "Fix: ${YELLOW}$hint${NC}"
        fi

        if [ -s "$tmp_file" ]; then
            echo ""
            print_warning "Details:"
            if [ "$step_name" = "🧪 Running complete test suite..." ]; then
                # Show more for tests
                echo "${MAGENTA}$(tail -30 "$tmp_file")${NC}"
            else
                echo "${MAGENTA}$(head -20 "$tmp_file")${NC}"
            fi
        fi
        echo ""
        rm -f "$tmp_file"

        if [ "$non_blocking" = "true" ]; then
            return 0
        fi
        return 1
    fi
}
