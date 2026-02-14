#!/bin/bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TARGET="$HOME/.local/bin/mutator"
mkdir -p "$HOME/.local/bin"

# Build release binary
echo "Building mutator..."
cd "$SCRIPT_DIR"
source "$HOME/.cargo/env" 2>/dev/null || true
cargo build --release

BINARY="$SCRIPT_DIR/target/release/mutator"
if [ ! -f "$BINARY" ]; then
    echo "Error: Build failed, binary not found at $BINARY"
    exit 1
fi

if [ -e "$TARGET" ] && [ ! -L "$TARGET" ]; then
    echo "Warning: $TARGET exists and is not a symlink. Skipping."
    exit 1
fi
ln -sf "$BINARY" "$TARGET"
echo "Installed: $TARGET -> $BINARY"
