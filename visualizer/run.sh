#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR/.."

echo "=== Building Visualizer ==="
cargo build --release --manifest-path visualizer/Cargo.toml

echo "=== Starting Visualizer ==="
echo "Open http://localhost:8088 in your browser"
./target/release/visualizer
