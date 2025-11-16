#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

SRC="$ROOT/tree-sitter/queries"
DEST="$ROOT/editors/zed/queries/patchwork"

mkdir -p "$DEST"
cp "$SRC"/*.scm "$DEST"/

echo "Synced queries from $SRC to $DEST"
