#!/bin/bash
cd $(dirname "$0")
echo "Building for testing (1/3)"
cargo build
if [ "$?" -ne 0 ]; then
    echo "Build failed"
    return 1
fi
clear
echo "Running Tests (2/3)"
cargo test
if [ "$?" -ne 0 ]; then
    echo "Tests failed"
    return 1
fi
clear
echo "Setting up Application (3/3)"
cargo build --release &&
clear
./target/release/simple_to_do
