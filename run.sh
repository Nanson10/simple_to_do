#!/bin/bash
cd $(dirname "$0")
cargo build
if [ "$?" -ne 0 ]; then
    echo "Build failed"
    return 1
fi
clear
cargo test
if [ "$?" -ne 0 ]; then
    echo "Tests failed"
    return 1
fi
clear
cargo build --release &&
clear
./target/release/simple_to_do
