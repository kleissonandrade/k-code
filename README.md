<p align="center">
  <img src="https://img.shields.io/badge/rust-2021-orange?logo=rust" alt="Rust 2021" />
  <img src="https://img.shields.io/badge/version-0.1.0-blue" alt="Version" />
  <img src="https://img.shields.io/badge/platform-macOS%20%7C%20Linux-lightgrey" alt="Platform" />
  <img src="https://img.shields.io/badge/license-MIT-green" alt="License" />
</p>

# k-code

A fast, vim-inspired terminal code editor written in Rust. Built for developers who prefer keyboard-driven workflows with integrated terminal, file tree, and git — all inside the terminal.

<p align="center">
  <img src=".github/assets/screenshot.png" alt="k-code screenshot" width="800" />
</p>

---

## Features

- **Vim-like modal editing** — Normal, Insert, Visual, V-Line, and Command modes
- **Syntax highlighting** — powered by [syntect](https://github.com/trishume/syntect) with broad language support
- **Fuzzy file finder** (`Ctrl+P`) — instant file navigation across the project
- **File tree** (`Ctrl+B`) — sidebar directory browser with expand/collapse
- **In-file search** (`Ctrl+F` or `/`) — floating search bar with match highlighting
- **Global search** (`Space+F`) — search across all project files with preview
- **Go to definition** (`gd`) — jump to symbol definitions
- **Git integration** (`Ctrl+G`) — status, branches, stash, log, diff viewer, and GitHub Actions
- **Integrated terminal** (`` Ctrl+` `` or `Space+J`) — full PTY with VTE emulation and multi-tab support
- **Multi-buffer tabs** — open and switch between multiple files
- **5 built-in themes** — Amethyst, Aureum, Dracula, Airo, and Monokai
- **Configurable** — `~/.config/k-code/config.toml`

---

## Installation

### Prerequisites

| Dependency | Version | How to install |
|---|---|---|
| **Rust toolchain** | 1.70+ | See below |
| **Git** | 2.x | Pre-installed on macOS; `sudo apt install git` on Linux |
| **C compiler** (for native deps) | — | Xcode CLT on macOS; `build-essential` on Linux |
| **pkg-config** | — | `brew install pkg-config` / `sudo apt install pkg-config` |
| **libgit2 dev headers** | — | Included via `git2` crate (compiled from source) |
| **cmake** (for oniguruma regex in syntect) | 3.x+ | `brew install cmake` / `sudo apt install cmake` |

### macOS

```bash
# 1. Install Rust (if not installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

# 2. Install system dependencies
brew install cmake pkg-config

# 3. Clone the repository
git clone https://github.com/kleissonvieira/k-code.git
cd k-code

# 4. Build in release mode
cargo build --release

# 5. Run
./target/release/k-code            # open in current directory
./target/release/k-code ~/projects  # open a specific directory
./target/release/k-code file.rs     # open a specific file
```

**Optional — add to PATH:**

```bash
cp ./target/release/k-code /usr/local/bin/
```

### Linux (Ubuntu/Debian)

```bash
# 1. Install Rust (if not installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

# 2. Install system dependencies
sudo apt update
sudo apt install -y build-essential cmake pkg-config libssl-dev

# 3. Clone the repository
git clone https://github.com/kleissonvieira/k-code.git
cd k-code

# 4. Build in release mode
cargo build --release

# 5. Run
./target/release/k-code
```

### Linux (Fedora/RHEL)

```bash
# 1. Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

# 2. Install system dependencies
sudo dnf groupinstall -y "Development Tools"
sudo dnf install -y cmake pkg-config openssl-devel

# 3. Clone, build, and run
git clone https://github.com/kleissonvieira/k-code.git
cd k-code
cargo build --release
./target/release/k-code
```

### Linux (Arch)

```bash
# 1. Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

# 2. Install system dependencies
sudo pacman -S --needed base-devel cmake pkg-config openssl

# 3. Clone, build, and run
git clone https://github.com/kleissonvieira/k-code.git
cd k-code
cargo build --release
./target/release/k-code
```

---

## Quick Start

```bash
# Open a project directory
k-code ~/my-project

# Open a single file
k-code src/main.rs

# Open current directory
k-code .
```

Press `?` inside the editor to open the built-in tutorial.

---

## Keyboard Shortcuts

### Navigation

| Key | Action |
|---|---|
| `h` `j` `k` `l` / Arrow keys | Move cursor |
| `w` / `b` | Next / previous word |
| `0` / `$` | Line start / end |
| `gg` / `G` | File start / end |
| `PageUp` / `PageDown` | Page scroll |
| `:<number>` | Go to line |

### Editing

| Key | Action |
|---|---|
| `i` | Insert mode |
| `I` | Insert at line start |
| `A` | Append at line end |
| `o` | New line below and insert |
| `x` | Delete character |
| `dd` | Delete line |
| `yy` | Yank (copy) line |
| `p` | Paste |
| `u` | Undo |
| `Ctrl+R` | Redo |
| `Esc` | Back to Normal mode |

### Files & Buffers

| Key | Action |
|---|---|
| `Ctrl+S` | Save |
| `Ctrl+T` | New file |
| `Ctrl+P` | Fuzzy file finder |
| `Ctrl+B` | Toggle file tree |
| `Tab` / `Shift+Tab` | Next / previous buffer |

### Search

| Key | Action |
|---|---|
| `/` or `Ctrl+F` | In-file search |
| `n` / `N` | Next / previous match |
| `Space+F` | Global search across files |
| `gd` | Go to definition |

### Git (`Ctrl+G`)

| Key | Action |
|---|---|
| `Tab` | Switch tabs (Status / Branches / Stash / Log / Actions) |
| `s` | Stage selected file |
| `a` | Stage all |
| `c` | Commit |
| `p` | Push |
| `Enter` | Checkout branch / Pop stash |
| `n` | New branch |
| `d` | View diff |

### Terminal

| Key | Action |
|---|---|
| `` Ctrl+` `` or `Space+J` | Toggle terminal |
| `Shift+Ctrl+N` | New terminal tab |

### UI

| Key | Action |
|---|---|
| `Space+T` | Cycle theme |
| `?` | Help / Tutorial |
| `q` | Quit |

### Command Mode (`:`)

| Command | Action |
|---|---|
| `:w` | Save |
| `:q` | Quit |
| `:wq` | Save and quit |
| `:q!` | Force quit |
| `:e <path>` | Open file |
| `:theme` | Change theme |
| `:goto <n>` | Go to line |

---

## Themes

Switch themes at any time with `Space+T`.

| Theme | Style |
|---|---|
| **Amethyst** | Purple tones (default) |
| **Aureum** | Warm gold |
| **Dracula** | Dark with vivid accents |
| **Airo** | Cyan / teal minimal |
| **Monokai** | Classic |

---

## Configuration

k-code reads its config from `~/.config/k-code/config.toml`. A file is created with defaults on first run.

```toml
theme = "Amethyst"

[editor]
tab_size = 4
use_spaces = true
show_line_numbers = true
highlight_current_line = true
scroll_padding = 5

[file_tree]
show_hidden = false
width = 40
show_icons = true
```

---

## Project Structure

```
k-code/
├── src/
│   ├── main.rs              # Entry point and event loop
│   ├── app.rs               # Application state
│   ├── action.rs            # Action definitions
│   ├── keymap.rs            # Keybinding resolution
│   ├── mode.rs              # Editor modes
│   ├── config.rs            # Configuration
│   ├── theme.rs             # Theme definitions
│   ├── layout/              # Layout engine
│   └── components/          # UI components
│       ├── editor.rs        # Text editor
│       ├── file_tree.rs     # File tree sidebar
│       ├── fuzzy_finder.rs  # Fuzzy search
│       ├── global_search.rs # Project-wide search
│       ├── git_panel.rs     # Git integration
│       ├── terminal_panel.rs# Integrated terminal
│       ├── search.rs        # In-file search
│       └── ...
├── crates/
│   ├── k-buffer/            # Text buffer (ropey)
│   ├── k-git/               # Git operations (git2)
│   ├── k-syntax/            # Syntax highlighting (syntect)
│   └── k-terminal/          # Terminal emulation (PTY + VTE)
└── Cargo.toml
```

---

## Building from Source

```bash
# Debug build (faster compile, slower runtime)
cargo build

# Release build (slower compile, optimized binary with LTO)
cargo build --release

# Run tests
cargo test --workspace

# Run with logging
RUST_LOG=debug cargo run -- .
```

Logs are written to `~/.local/share/k-code/k-code.log`.

---

## Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/my-feature`)
3. Commit your changes (`git commit -m 'Add my feature'`)
4. Push to the branch (`git push origin feature/my-feature`)
5. Open a Pull Request

---

## License

This project is licensed under the [MIT License](LICENSE).
