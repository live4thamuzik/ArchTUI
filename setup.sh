#!/bin/bash
# setup.sh — Download and verify the ArchTUI binary from the latest release
set -euo pipefail

REPO="live4thamuzik/ArchTUI"
BASE_URL="https://github.com/${REPO}/releases/latest/download"

echo "Downloading ArchTUI binary..."
if ! curl -fsSL "${BASE_URL}/archtui" -o archtui; then
    echo "Error: failed to download binary. Check your network connection." >&2
    exit 1
fi

if ! curl -fsSL "${BASE_URL}/archtui.sha256" -o archtui.sha256; then
    echo "Error: failed to download checksum." >&2
    rm -f archtui
    exit 1
fi

echo "Verifying SHA256 checksum..."
if ! sha256sum -c archtui.sha256; then
    echo "Error: checksum verification failed. Binary may be corrupted." >&2
    rm -f archtui archtui.sha256
    exit 1
fi

rm -f archtui.sha256
chmod +x archtui
echo "Ready. Run ./archtui to start."
