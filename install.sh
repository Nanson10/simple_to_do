#!/bin/bash
set -euo pipefail

cd "$(dirname "$0")"

BIN_DIR="$HOME/.local/bin"
BIN_PATH="$BIN_DIR/simple_to_do"
ALIAS_PATH="$BIN_DIR/std"

echo "Building for testing (1/4)"
cargo build
clear

echo "Running Tests (2/4)"
cargo test
clear

echo "Building Application for release (3/4)"
cargo build --release
clear

echo "Installing Application (4/4)"
mkdir -p "$BIN_DIR"
install -m 755 target/release/simple_to_do "$BIN_PATH"
ln -sfn "$BIN_PATH" "$ALIAS_PATH"
clear

"$BIN_PATH"