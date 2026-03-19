#!/bin/bash
set -euo pipefail

cd "$(dirname "$0")"

BIN_DIR="$HOME/.local/bin"
BIN_PATH="$BIN_DIR/simple_to_do"
ALIAS_PATH="$BIN_DIR/std"

echo "Building for testing (1/3)"
cargo build

clear
echo "Running Tests (2/3)"
cargo test

clear
echo "Installing Application (3/3)"
cargo build --release
mkdir -p "$BIN_DIR"
install -m 755 target/release/simple_to_do "$BIN_PATH"
ln -sfn "$BIN_PATH" "$ALIAS_PATH"
clear
"$BIN_PATH"
clear