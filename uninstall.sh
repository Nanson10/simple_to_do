#!/bin/bash
set -euo pipefail

BIN_PATH="$HOME/.local/bin/simple_to_do"
ALIAS_PATH="$HOME/.local/bin/std"

rm -f "$BIN_PATH"
rm -f "$ALIAS_PATH"
echo "Uninstalled: $BIN_PATH"
echo "Uninstalled alias: $ALIAS_PATH"