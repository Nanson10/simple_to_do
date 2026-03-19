#!/bin/bash
set -euo pipefail

cd "$(dirname "$0")"

echo "Building for testing (1/3)"
cargo build

clear
echo "Running Tests (2/3)"
cargo test

clear
echo "Running Application (3/3)"
./target/debug/simple_to_do
clear