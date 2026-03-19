#!/bin/bash
cd $(dirname "$0")
cargo test &> /dev/null
if [ "$?" -ne 0 ]; then
    echo "Tests failed"
    return 1
fi
cargo run --release
