#!/bin/bash
# run_tests.sh - Test runner for bash script tests
#
# Usage:
#   ./run_tests.sh              # Run all tests
#   ./run_tests.sh utils        # Run only utils tests
#   ./run_tests.sh --verbose    # Run with verbose output
#   ./run_tests.sh --tap        # Run with TAP output format

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$(dirname "$SCRIPT_DIR")")"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Default options
VERBOSE=false
TAP_OUTPUT=false
SPECIFIC_TEST=""

# Parse arguments
while [[ $# -gt 0 ]]; do
    case "$1" in
        --verbose|-v)
            VERBOSE=true
            shift
            ;;
        --tap)
            TAP_OUTPUT=true
            shift
            ;;
        --help|-h)
            echo "Usage: $0 [OPTIONS] [TEST_FILE]"
            echo ""
            echo "Options:"
            echo "  --verbose, -v    Show verbose test output"
            echo "  --tap            Output in TAP format"
            echo "  --help, -h       Show this help message"
            echo ""
            echo "Test files:"
            echo "  utils            Run utils.bats tests"
            echo "  disk_utils       Run disk_utils.bats tests"
            echo "  config_loader    Run config_loader.bats tests"
            echo ""
            echo "Examples:"
            echo "  $0               # Run all tests"
            echo "  $0 utils         # Run only utils tests"
            echo "  $0 -v            # Run all tests with verbose output"
            exit 0
            ;;
        *)
            SPECIFIC_TEST="$1"
            shift
            ;;
    esac
done

# Check if bats is installed
check_bats() {
    if ! command -v bats &> /dev/null; then
        echo -e "${YELLOW}bats (Bash Automated Testing System) is not installed.${NC}"
        echo ""
        echo "Install bats using one of these methods:"
        echo ""
        echo "  Arch Linux:    sudo pacman -S bash-bats"
        echo "  Ubuntu/Debian: sudo apt-get install bats"
        echo "  macOS:         brew install bats-core"
        echo "  Manual:        git clone https://github.com/bats-core/bats-core.git && cd bats-core && sudo ./install.sh /usr/local"
        echo ""
        exit 1
    fi
}

# Print header
print_header() {
    echo ""
    echo -e "${BLUE}╔═══════════════════════════════════════════════════════════╗${NC}"
    echo -e "${BLUE}║       Archinstall TUI - Bash Script Test Suite            ║${NC}"
    echo -e "${BLUE}╚═══════════════════════════════════════════════════════════╝${NC}"
    echo ""
}

# Run tests
run_tests() {
    local test_files=()
    local bats_opts=()

    # Build bats options
    if [[ "$VERBOSE" == "true" ]]; then
        bats_opts+=("--verbose-run")
    fi

    if [[ "$TAP_OUTPUT" == "true" ]]; then
        bats_opts+=("--tap")
    fi

    # Determine which tests to run
    if [[ -n "$SPECIFIC_TEST" ]]; then
        case "$SPECIFIC_TEST" in
            utils)
                test_files+=("$SCRIPT_DIR/utils.bats")
                ;;
            disk_utils)
                test_files+=("$SCRIPT_DIR/disk_utils.bats")
                ;;
            config_loader)
                test_files+=("$SCRIPT_DIR/config_loader.bats")
                ;;
            *)
                if [[ -f "$SCRIPT_DIR/${SPECIFIC_TEST}.bats" ]]; then
                    test_files+=("$SCRIPT_DIR/${SPECIFIC_TEST}.bats")
                elif [[ -f "$SPECIFIC_TEST" ]]; then
                    test_files+=("$SPECIFIC_TEST")
                else
                    echo -e "${RED}Error: Unknown test file: $SPECIFIC_TEST${NC}"
                    exit 1
                fi
                ;;
        esac
    else
        # Run all tests
        test_files+=("$SCRIPT_DIR"/*.bats)
    fi

    # Check if any test files exist
    if [[ ${#test_files[@]} -eq 0 ]]; then
        echo -e "${YELLOW}No test files found.${NC}"
        exit 1
    fi

    echo -e "${GREEN}Running tests...${NC}"
    echo ""

    # Run bats
    if bats "${bats_opts[@]}" "${test_files[@]}"; then
        echo ""
        echo -e "${GREEN}╔═══════════════════════════════════════════════════════════╗${NC}"
        echo -e "${GREEN}║                    All tests passed!                       ║${NC}"
        echo -e "${GREEN}╚═══════════════════════════════════════════════════════════╝${NC}"
        return 0
    else
        echo ""
        echo -e "${RED}╔═══════════════════════════════════════════════════════════╗${NC}"
        echo -e "${RED}║                    Some tests failed!                      ║${NC}"
        echo -e "${RED}╚═══════════════════════════════════════════════════════════╝${NC}"
        return 1
    fi
}

# Main
main() {
    print_header
    check_bats
    run_tests
}

main "$@"
