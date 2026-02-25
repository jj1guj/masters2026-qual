#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR/.." || exit 1

echo "=== Running score visualizer ==="

# Check prerequisites
if [ ! -d "ahc_score_visualizer" ]; then
    echo "[ERROR] ahc_score_visualizer directory not found."
    echo "Run: bash scripts/setup.sh" >&2
    exit 1
fi

if [ ! -f "config/config.toml" ]; then
    echo "[ERROR] config/config.toml not found."
    echo "Run: bash scripts/setup.sh" >&2
    exit 1
fi

if [ ! -d "tools" ] || [ -z "$(ls -A tools 2>/dev/null || true)" ]; then
    echo "[INFO] tools directory not found. Running prepare_tools.sh..."
    bash scripts/prepare_tools.sh
fi

echo "[INFO] Building solver..."
cargo build --release --manifest-path solver/Cargo.toml

echo "[INFO] Building tools..."
cargo build --release --manifest-path tools/Cargo.toml

echo "[INFO] Building ahc_score_visualizer..."
cargo build --release --manifest-path ahc_score_visualizer/Cargo.toml

echo "[INFO] Running visualizer..."
./target/release/score_visualizer --config config/config.toml
