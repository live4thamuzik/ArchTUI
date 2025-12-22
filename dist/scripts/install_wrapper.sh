#!/bin/bash
# install_wrapper.sh - TUI-friendly wrapper for the main installation script
# This script ensures clean output for the TUI by redirecting all output properly

set -euo pipefail

# Get the directory where this script is located
SCRIPT_DIR="$(dirname "${BASH_SOURCE[0]}")"

# Set environment variables for clean output
export TERM=dumb
export LANG=C
export LC_ALL=C

# Redirect stderr to stdout so all output goes through the same pipe
exec 2>&1

# Run the main installation script
exec bash "$SCRIPT_DIR/install.sh" "$@"
