# ArchInstall TUI

A Rust/Bash implementation of the Arch Linux installer with a clean TUI interface.

## Overview

This is a complete rewrite of the Arch Linux installer using:
- **Rust TUI** frontend for user interaction
- **Bash backend** following Arch Wiki installation guide
- **Pre-compiled binary** for immediate use on live ISO

## Quick Start

```bash
git clone https://github.com/your-username/archinstall.git
cd archinstall
sudo ./archinstall-tui
```

## Features

### Core Functionality
- **40+ configuration options** covering all installation aspects
- **Real-time progress** with live installation feedback
- **Smart validation** to prevent configuration errors
- **Zero dependencies** - pre-compiled binary included

### Partitioning
- **ESP + XBOOTLDR** setup (Arch Wiki recommended)
- **Multiple filesystems**: ext4, xfs, btrfs
- **LVM support** for complex partitioning
- **LUKS encryption** support
- **UUID-based** device management

### Package Management
- **Interactive package selection** with terminal-like interface
- **AUR integration** via API (no paru/yay required)
- **Dependency checking** before installation
- **Pacman package search** with real-time results

### System Configuration
- **Desktop environments**: GNOME, KDE, Hyprland, i3, XFCE
- **Display managers** auto-configured based on DE selection
- **Plymouth themes** and GRUB customization
- **Network configuration** and locale setup

## Architecture

```
Frontend (Rust TUI)     Backend (Bash)
┌─────────────────┐     ┌─────────────────┐
│ User Interface  │────▶│ Installation    │
│ Configuration   │     │ Scripts         │
│ Validation      │     │ Arch Wiki       │
│ Progress Display│     │ Compliance      │
└─────────────────┘     └─────────────────┘
```

## Usage

1. **Run the installer**: `sudo ./archinstall-tui`
2. **Navigate**: Arrow keys to move, Enter to configure
3. **Configure**: Set installation options through TUI dialogs
4. **Install**: Press Space to start installation
5. **Monitor**: Watch real-time progress and logs

## Configuration Options

### Boot Setup
- Boot Mode (Auto/UEFI/BIOS)
- Secure Boot (with UEFI validation)
- Bootloader (GRUB/systemd-boot)

### System Setup
- Disk selection and partitioning
- Filesystem selection (ext4/xfs/btrfs)
- Encryption configuration
- Swap and home partition options

### Localization
- Timezone and region selection
- Locale configuration
- Keymap selection

### Software
- Desktop environment selection
- Display manager configuration
- Additional packages (Pacman + AUR)
- AUR helper installation

## Technical Details

### Rust Frontend
- **ratatui** for TUI interface
- **crossterm** for terminal control
- **Modular design** with separate concerns
- **Type-safe configuration** management

### Bash Backend
- **Arch Wiki compliant** installation process
- **Error handling** with proper exit codes
- **Logging** for troubleshooting
- **Dependency management** for required packages

### Validation & Safety
- **UEFI detection** for boot mode validation
- **Secure Boot warnings** for proper setup
- **Dependency checking** before operations
- **Configuration validation** before installation

## Development

### Project Structure
```
src/
├── main.rs          # Application entry point
├── app.rs           # Main application logic
├── config.rs        # Configuration management
├── input.rs         # User input handling
├── ui.rs            # TUI rendering
└── package_utils.rs # Package search utilities

scripts/
├── install.sh           # Main installation script
├── install_wrapper.sh   # TUI-friendly wrapper
├── utils.sh            # Utility functions
└── disk_strategies.sh  # Partitioning strategies
```

### Building
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
- Internet connection (for packages)

## License

MIT License

---

**Simple, clean, and effective.** Just like Arch Linux itself.