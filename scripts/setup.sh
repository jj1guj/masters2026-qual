#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR/.." || exit 1

echo "=== Setting up contest template ==="

# Initialize submodules (ahc_score_visualizer)
echo "[INFO] Initializing git submodules..."
git submodule update --init --recursive

# Prepare tools
bash scripts/prepare_tools.sh

# Setup solver (Rust boilerplate if needed)
if [ ! -f "solver/Cargo.toml" ] && [ -z "$(ls -A solver 2>/dev/null || true)" ]; then
    echo "[INFO] Creating basic Rust solver project..."
    cargo init solver
fi

echo ""
echo "[✓] Template setup complete!"
echo ""
echo "Next steps:"
echo "  1. Edit config/config.toml with your tools.zip URL"
echo "  2. Add your solver code to solver/"
echo "  3. Run: bash scripts/run_score.sh"
echo "     or: Ctrl+Shift+Enter in VS Code"
