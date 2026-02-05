#!/bin/bash
# Generate Rust documentation for ArchTUI
#
# Usage: ./scripts/generate_docs.sh [--open]
#
# Options:
#   --open    Open generated docs in browser after building

set -euo pipefail

echo "Generating documentation..."

# Generate docs with private items documented
cargo doc --no-deps --document-private-items --no-default-features

echo "Documentation generated at: target/doc/archtui/index.html"

# Open in browser if requested
if [[ "${1:-}" == "--open" ]]; then
    if command -v xdg-open &>/dev/null; then
        xdg-open target/doc/archtui/index.html
    elif command -v open &>/dev/null; then
        open target/doc/archtui/index.html
    else
        echo "Could not detect browser opener. Open manually: target/doc/archtui/index.html"
    fi
fi
