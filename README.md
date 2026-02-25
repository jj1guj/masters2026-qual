# AHC Template

Template repository for AtCoder Heuristic Contest with integrated score visualizer and GitHub Actions support.

## Directory Structure

```
.
├── .vscode/              # VS Code settings, tasks, extensions
├── .github/workflows/    # GitHub Actions (score.yml)
├── solver/               # Your solver code (Rust/Go/C++)
├── config/
│   └── config.toml
├── tools/                # Downloaded from contest (git-ignored, via prepare_tools.sh)
├── results/              # Visualization output
├── scripts/
│   ├── setup.sh         # Initial setup (clone score_visualizer, prepare tools)
│   ├── prepare_tools.sh # Download and extract tools.zip
│   └── run_score.sh     # Run visualizer (local testing)
└── score_visualizer/     # Git submodule pointing to tool repo
```

⚠️ **Submodule must be initialized before running:**
```bash
git submodule update --init
```

## Setup

### Local Development

1. **Initialize submodules:**
   ```bash
   git submodule update --init
   ```

2. **Run setup script** (one-time):
   ```bash
   bash scripts/setup.sh
   ```
   This will:
   - Clone score_visualizer (if submodule not yet present)
   - Prompt you to edit `config/config.toml` with your tools.zip URL

3. **Edit configuration:**
   ```bash
   # config/config.toml
   tools_zip_url = "https://your-contest-url/tools.zip"
   ```
   ⚠️ `config/config.toml` is tracked by Git. Make sure to commit and push it so that CI can use the same configuration.

4. **Add your solver code** to `solver/` (Rust/Go/C++)

### Run Tests Locally

Two options:

**Option A: Terminal**
```bash
bash scripts/run_score.sh
```

**Option B: VS Code**
- `Ctrl+Shift+Enter` (runs "Run Visualizer" task)
- Terminal appears on the right side

### GitHub Actions (PR Trigger)

1. **Push to PR:**
   - The workflow automatically runs on PR creation/update
   - Tools are downloaded using the `tools_zip_url` in `config/config.toml` (must be committed and pushed beforehand)
   - Results appear as artifacts
   - Score comment is posted to the PR

## Configuration (config/config.toml)

```toml
# Tools artifact URL
tools_zip_url = "https://example.com/tools.zip"

# Directories
input_dir = "./tools/in"
visualizer_dir = "./tools"
html_output = "./results"

# Tester (scores your solution against test cases)
[tester]
command = "cargo run --release --bin tester --manifest-path {{script}}/Cargo.toml"
script = "./tools"
```

## Features

- ✅ **Streaming visualization**: Score calculated tests immediately visualized
- ✅ **VS Code integration**: Built-in terminal right-side, keyboard shortcut, progress bars
- ✅ **cargo-compete compatible**: Uses standard Rust Cargo.toml in solver/
- ✅ **Multi-language**: Supports Rust + Go + C++
- ✅ **GitHub Actions**: Auto-scores PRs, uploads artifacts, comments with results
- ✅ **Portable**: Submodule tool setup works locally and in CI

## References

- [score_visualizer repo](https://github.com/yourname/ahc-score-visualizer)
- [Supported external crates (cargo-compete)](https://github.com/taiki-e/cargo-compete)
