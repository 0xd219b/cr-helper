# cr-helper

> An interactive code review tool designed for Agent CLIs, enabling AI Agents to efficiently understand and respond to code reviews.

[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

## Features

- **Agent-Friendly**: Designed for Agent CLIs like Claude Code, with token-optimized structured output
- **High Performance**: Built with Rust and [ratatui](https://github.com/ratatui/ratatui) for blazing fast TUI
- **Vim-Style Navigation**: Familiar j/k navigation with inline comments
- **Lazy Loading**: On-demand file loading for fast startup with large projects
- **Deep Integration**: Seamless integration with Claude Code via Skills + Hooks

## Quick Start

### Installation

```bash
# Install from source
git clone https://github.com/0xd219b/cr-helper.git
cd cr-helper
cargo install --path crates/cr-cli

# Or build directly
cargo build --release
./target/release/cr-helper --help
```

### Basic Usage

```bash
# Review working tree changes
cr-helper review

# Review staged changes
cr-helper review --staged

# Review specific commit
cr-helper review --commit HEAD~1

# Include untracked files (new files)
cr-helper review --untracked
cr-helper review -u

# Create session without starting TUI
cr-helper review --no-tui

# Export review results
cr-helper export -s <session-id>
cr-helper export -s <session-id> --format json
```

## TUI Interface

After launching the TUI, you'll see a Vim-style code review interface:

```
┌─────────────────────────────────────────────────────────────┐
│ ~ src/main.rs [1/5]                                         │  <- Title bar
├─────────────────────────────────────────────────────────────┤
│   1    fn main() {                                          │
│   2 +      println!("Hello, world!");                       │  <- Current line highlighted
│         │ INFO: This is a comment                           │  <- Inline comment
│   3    }                                                    │
├─────────────────────────────────────────────────────────────┤
│ NORMAL | L2 | 1 comments | 20260101-abc123                  │  <- Status bar
└─────────────────────────────────────────────────────────────┘
```

### Keybindings

| Key | Action |
|-----|--------|
| `j` / `↓` | Move cursor down |
| `k` / `↑` | Move cursor up |
| `g` | Go to top of file |
| `G` | Go to bottom of file |
| `Ctrl-u` | Page up |
| `Ctrl-d` | Page down |
| `n` | Next file |
| `N` | Previous file |
| `]` | Jump to next comment |
| `[` | Jump to previous comment |
| `c` | Add line comment |
| `C` | Add file-level comment |
| `x` | Delete comment on current line |
| `s` | Save session |
| `?` | Show help |
| `q` | Quit |

### Adding Comments

1. Navigate to target line with `j`/`k`
2. Press `c` to enter insert mode
3. Type your comment
4. Press `Enter` to confirm, `Esc` to cancel

## Export Formats

### Markdown Format

```bash
cr-helper export -s <session-id>
```

Example output:

```markdown
# Code Review Report

**Session:** `20260101-abc123`
**Date:** 2026-01-01 12:00:00 UTC

## Summary

- **Total Comments:** 2
- **Files Reviewed:** 5
- 1 Critical Issues
- 1 Info

## Critical Issues

### `src/auth.rs:42`

SQL injection vulnerability detected

> **Line 42:** `let query = format!("SELECT * FROM users WHERE id = {}", user_id);`

```rust
  40  fn get_user(user_id: &str) -> User {
  41 +    let db = connect();
  42 +    let query = format!("SELECT * FROM users WHERE id = {}", user_id); <<<
  43 +    db.query(&query)
  44  }
```

---
```

### JSON Format (Token-Optimized)

```bash
cr-helper export -s <session-id> --format json
```

Outputs compact JSON suitable for AI Agent parsing.

## Session Management

```bash
# List all sessions
cr-helper session list

# View session details
cr-helper session info <session-id>

# Resume session for review
cr-helper review -s <session-id>
```

## Configuration

### Initialize Project Configuration

```bash
cr-helper init
```

This creates a `.cr-helper/config.toml` configuration file.

### Example Configuration

```toml
[review]
auto_save = true
context_lines = 3

[export]
default_format = "markdown"
include_diff = true
```

### .gitignore Configuration

cr-helper uses `.gitignore` to exclude files from review. If you have too many files when using `--untracked`, ensure your `.gitignore` includes:

```gitignore
target/
node_modules/
dist/
build/
__pycache__/
.venv/
```

## Claude Code Integration

### Install Integration

```bash
# One-click install for Claude Code
cr-helper install --claude-code

# Verify installation
cr-helper doctor --claude-code
```

### Workflow

```
User: "Refactor this module"
  ↓
Agent: Modifies code
  ↓
Agent: Calls cr-helper review
  ↓
cr-helper: Generates review report
  ↓
Agent: Reads report, fixes issues
  ↓
Done
```

## Project Structure

```
cr-helper/
├── crates/
│   ├── cr-cli/           # CLI entry point
│   ├── cr-core/          # Core business logic
│   │   ├── diff/         # Diff parsing and navigation
│   │   ├── comment/      # Comment management
│   │   ├── session/      # Session lifecycle
│   │   └── export/       # Multi-format export
│   ├── cr-ui/            # TUI interface (ratatui)
│   ├── cr-integration/   # Agent adapters
│   └── cr-storage/       # Persistence layer
└── docs/                 # Documentation
```

## Development

```bash
# Clone repository
git clone https://github.com/0xd219b/cr-helper.git
cd cr-helper

# Build
cargo build

# Run tests
cargo test

# Run (debug mode is slow, use release)
cargo run --release -- review
```

## Tech Stack

| Component | Technology | Description |
|-----------|------------|-------------|
| Core | Rust 1.70+ | High performance, cross-platform |
| TUI | ratatui 0.26 | Modern terminal UI |
| CLI | clap 4.5 | Command-line argument parsing |
| Serialization | serde + serde_json | JSON processing |

## FAQ

### Q: Why Rust?

1. **Performance**: Smooth rendering for large diff files, fast startup
2. **Distribution**: Single binary, no runtime dependencies
3. **Cross-platform**: Native support for macOS/Linux/Windows

### Q: Why is the debug build so slow?

Debug builds have no optimizations and can be 100x slower than release builds. Always use:

```bash
cargo build --release
./target/release/cr-helper review
```

### Q: How to handle large number of untracked files?

1. Ensure `.gitignore` is properly configured
2. cr-helper warns when file count exceeds 100
3. Uses lazy loading - files are only loaded when navigated to

## License

MIT License - See [LICENSE](LICENSE)

## Acknowledgments

- [ratatui](https://github.com/ratatui/ratatui) - Excellent TUI framework
- [Claude Code](https://claude.ai/code) - AI coding tool that inspired this project

---

**Star this project to support development!**
