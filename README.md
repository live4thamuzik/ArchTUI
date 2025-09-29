# ArchInstall TUI

A Rust/Bash implementation of the Arch Linux installer with a clean TUI interface.

## Quick Start

```bash
git clone https://github.com/your-username/archinstall.git
cd archinstall
./archinstall-tui
```

## Features

- **40+ configuration options** with real-time progress and smart validation
- **Zero dependencies** - pre-compiled binary for immediate use on live ISO
- **Interactive package selection** with Pacman and AUR search via API
- **ESP + XBOOTLDR partitioning** for dual-boot compatibility
- **Multiple filesystems**: ext4, xfs, btrfs with LVM and LUKS encryption support
- **Desktop environments**: GNOME, KDE, Hyprland, i3, XFCE with auto-configured display managers
- **Comprehensive validation** with UEFI detection and Secure Boot warnings

## Architecture

```
Frontend (Rust TUI)     Backend (Bash)
┌─────────────────┐     ┌─────────────────┐
│ User Interface  │────▶│ Installation    │
│ Configuration   │     │ Scripts         │
│ Validation      │     │ Package         │
│ Progress Display│     │ Management      │
└─────────────────┘     └─────────────────┘
```

## Usage

1. **Run**: `./archinstall-tui`
2. **Navigate**: Arrow keys, Enter to configure
3. **Install**: Press Space to start
4. **Monitor**: Watch real-time progress

## Technical Stack

- **Frontend**: Rust with ratatui/crossterm for TUI interface
- **Backend**: Bash scripts with modular functions and proper error handling
- **Testing**: Comprehensive Rust test suite with CI/CD pipeline

## Project Structure

```
src/                     # Rust TUI source
├── main.rs             # Entry point
├── app.rs              # Application logic
├── config.rs           # Configuration management
├── input.rs            # User input handling
├── ui.rs               # TUI rendering
└── package_utils.rs    # Package search utilities

scripts/                 # Bash installation backend
├── install.sh          # Main installation script
├── install_wrapper.sh  # TUI-friendly wrapper
├── utils.sh           # Utility functions
└── disk_strategies.sh # Partitioning strategies
```

## Development

```bash
# Development
git checkout dev
make build

# Testing
git checkout test
cargo test
```

## Requirements

- Arch Linux live ISO
- Root privileges
- Internet connection

## License

MIT License