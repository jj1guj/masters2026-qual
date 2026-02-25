#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR/.." || exit 1

echo "=== Preparing tools ==="

# Check if config exists
if [ ! -f "config/config.toml" ]; then
    echo "[ERROR] config/config.toml not found. Please create it with your tools.zip URL." >&2
    exit 1
fi

# Read tools_zip_url from config using grep
TOOLS_ZIP_URL=$(grep '^tools_zip_url\s*=' config/config.toml | head -1 | cut -d'=' -f2 | xargs)

if [ -z "$TOOLS_ZIP_URL" ] || [ "$TOOLS_ZIP_URL" = "https://example.com/tools.zip" ]; then
    echo "[ERROR] tools_zip_url not set or still using example URL in config/config.toml" >&2
    exit 1
fi

# Download and extract
echo "[INFO] Downloading tools from: $TOOLS_ZIP_URL"
wget -q --show-progress -O /tmp/tools.zip "$TOOLS_ZIP_URL"
echo "[INFO] Extracting tools..."
unzip -q -o /tmp/tools.zip
rm /tmp/tools.zip

echo "[INFO] ✓ Tools prepared successfully"
