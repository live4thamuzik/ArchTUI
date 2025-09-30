# ArchInstall TUI

A professional, production-ready Arch Linux installer with a clean TUI interface. Built with Rust and Bash for maximum reliability and user experience.

## Quick Start

```bash
git clone https://github.com/your-username/archinstall.git
cd archinstall
./archinstall-tui
```

## Features

### 🎯 **User Experience**
- **40+ configuration options** with real-time progress and smart validation
- **Zero dependencies** - pre-compiled binary for immediate use on live ISO
- **Interactive package selection** with Pacman and AUR search via API
- **Dynamic UI** that adapts to terminal size with proper scrolling
- **Comprehensive validation** with UEFI detection and Secure Boot warnings

### 🔒 **Security & Reliability**
- **Bulletproof security** - all command injection vulnerabilities eliminated
- **Robust error handling** - graceful failure recovery throughout
- **Input validation** - prevents invalid configurations before installation
- **Safe shell scripting** - `set -euo pipefail` on all Bash scripts

### 💾 **Advanced Partitioning**
- **ESP + XBOOTLDR standard** for optimal dual-boot compatibility
- **Multiple filesystems**: ext4, xfs, btrfs with full LVM and LUKS support
- **RAID support** with automatic array creation and management
- **Manual partitioning** with guided setup and validation

### 🖥️ **System Configuration**
- **Desktop environments**: GNOME, KDE, Hyprland, i3, XFCE with auto-configured display managers
- **Bootloaders**: GRUB (BIOS/UEFI) and systemd-boot (UEFI only)
- **Secure Boot** support with proper UEFI validation
- **Localization** with timezone and keymap configuration

## Auto-Partitioning Layouts

The installer offers comprehensive auto-partitioning strategies designed for different use cases:

### 🏠 **Simple Layouts**
- **Simple**: Basic ESP + XBOOTLDR + Root partition
- **Simple + LUKS**: Same as simple but with full disk encryption

### 🔧 **LVM Layouts**
- **LVM**: ESP + XBOOTLDR + LVM with flexible volume management
- **LVM + LUKS**: LVM on top of encrypted partition for maximum flexibility

### 🛡️ **RAID Layouts**
- **RAID**: Software RAID1/RAID5 with ESP + XBOOTLDR
- **RAID + LUKS**: RAID arrays with full disk encryption
- **RAID + LVM**: RAID with logical volume management
- **RAID + LVM + LUKS**: Ultimate setup with RAID, LVM, and encryption

### 📐 **Layout Details**

| Strategy | ESP | XBOOTLDR | Root | Encryption | RAID | LVM | Use Case |
|----------|-----|----------|------|------------|------|-----|----------|
| Simple | ✅ | ✅ | ✅ | ❌ | ❌ | ❌ | Basic installation |
| Simple + LUKS | ✅ | ✅ | ✅ | ✅ | ❌ | ❌ | Encrypted single disk |
| LVM | ✅ | ✅ | ✅ | ❌ | ❌ | ✅ | Flexible partitioning |
| LVM + LUKS | ✅ | ✅ | ✅ | ✅ | ❌ | ✅ | Encrypted LVM |
| RAID | ✅ | ✅ | ✅ | ❌ | ✅ | ❌ | Multi-disk redundancy |
| RAID + LUKS | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ | Encrypted RAID |
| RAID + LVM | ✅ | ✅ | ✅ | ❌ | ✅ | ✅ | RAID with LVM flexibility |
| RAID + LVM + LUKS | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | Enterprise-grade setup |
| Manual | ✅ | ✅ | ✅ | User choice | User choice | User choice | Full control |

### 🔍 **What Each Layout Provides**

**ESP (EFI System Partition)**:
- FAT32 filesystem
- 512MB size
- Mounted at `/efi`
- Required for UEFI boot

**XBOOTLDR (Extended Boot Loader Partition)**:
- ext4 filesystem
- 1GB size
- Mounted at `/boot`
- Stores kernel and initramfs

**Root Partition**:
- User-selectable filesystem (ext4, xfs, btrfs)
- Remaining disk space
- Mounted at `/`
- Contains the entire system

**Encryption (LUKS)**:
- AES-256 encryption with SHA-512 hashing
- Password-based unlocking
- Full disk encryption when enabled

**RAID Arrays**:
- RAID1 for 2 disks (mirroring)
- RAID5 for 3+ disks (parity)
- Automatic array creation and configuration
- Built-in redundancy and performance

**LVM (Logical Volume Manager)**:
- Flexible volume sizing
- Easy resizing and management
- Snapshots support (btrfs)
- Multiple logical volumes per physical volume

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
2. **Configure**: Navigate with arrow keys, Enter to configure options
3. **Validate**: System checks configuration before allowing installation
4. **Install**: Press Space to start installation
5. **Monitor**: Watch real-time progress with detailed status updates

### Navigation
- **Arrow Keys**: Navigate configuration options
- **Enter**: Configure selected option
- **Space**: Start installation (when on green button)
- **Esc**: Cancel/return from dialogs
- **Q**: Quit installer

## Technical Stack

- **Frontend**: Rust with ratatui/crossterm for responsive TUI interface
- **Backend**: Modular Bash scripts with comprehensive error handling
- **Package Management**: Native pacman integration + AUR API via curl
- **Testing**: Comprehensive Rust test suite with CI/CD pipeline

## Project Structure

```
src/                     # Rust TUI source
├── main.rs             # Entry point with error handling
├── app.rs              # Application logic and state management
├── config.rs           # Configuration management and validation
├── input.rs            # User input handling and dialogs
├── ui.rs               # TUI rendering and layout management
├── package_utils.rs    # Package search utilities (Pacman + AUR)
├── scrolling.rs        # Reusable scrolling logic
└── error.rs            # Custom error types and handling

scripts/                 # Bash installation backend
├── install.sh          # Main installation orchestrator
├── install_wrapper.sh  # TUI-friendly output wrapper
├── utils.sh           # Common utility functions
├── disk_utils.sh      # Partitioning utilities and constants
├── disk_strategies.sh # Partitioning strategy dispatcher
└── strategies/        # Individual partitioning strategies
    ├── simple.sh      # Basic partitioning
    ├── simple_luks.sh # Encrypted partitioning
    ├── lvm.sh         # LVM partitioning
    ├── lvm_luks.sh    # Encrypted LVM
    ├── raid.sh        # RAID partitioning
    ├── raid_luks.sh   # Encrypted RAID
    ├── raid_lvm.sh    # RAID + LVM
    ├── raid_lvm_luks.sh # RAID + LVM + Encryption
    └── manual.sh      # Guided manual partitioning

Source/                 # Plymouth themes
├── arch-glow/         # Arch-themed boot splash
└── arch-mac-style/    # macOS-inspired theme
```

## Development

```bash
# Development branch
git checkout dev
make build

# Testing branch
git checkout test
cargo test

# Production branch
git checkout main
```

## Requirements

- **Arch Linux live ISO** (latest recommended)
- **Root privileges** (installer will request)
- **Internet connection** (for package downloads and AUR access)
- **Minimum 8GB RAM** (recommended for smooth operation)
- **UEFI or BIOS** (both supported with automatic detection)

## Security Features

- **Input sanitization** prevents command injection
- **Path validation** ensures safe file operations
- **UUID-based mounting** for reliable partition identification
- **Secure password handling** with proper validation
- **Error isolation** prevents cascade failures

## License

MIT License - See LICENSE file for details

## Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## Support

- **Issues**: Report bugs and request features via GitHub Issues
- **Discussions**: Join community discussions for help and ideas
- **Documentation**: Check the wiki for detailed guides and troubleshooting