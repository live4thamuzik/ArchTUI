# Arch Linux Toolkit

A comprehensive, production-ready Arch Linux installer and system administration toolkit. Built with Rust TUI frontend and modular Bash backend for maximum reliability, user experience, and functionality.

## 🚀 Quick Start

### Installation
```bash
git clone https://github.com/your-username/archinstall.git
cd archinstall
./archinstall-tui
```

### Usage
```bash
# Interactive TUI installer
./archinstall-tui

# Automated installation from config
./archinstall-tui install --config my_config.json

# System administration tools
./archinstall-tui tools disk format --device /dev/sda1 --filesystem ext4
./archinstall-tui tools system services --action status --service sshd
./archinstall-tui tools user security --action full
```

## 🎯 Features Overview

### 📦 **Dual-Purpose Design**
- **Guided Installer**: Beginner-friendly TUI for Arch Linux installation
- **System Toolkit**: Comprehensive administration tools for power users
- **Zero Dependencies**: Pre-compiled binary works immediately on live ISO
- **Scriptable**: Full CLI access for automation and scripting

### 🔧 **System Administration Toolkit (19 Tools)**

#### **💾 Disk & Filesystem Tools (5 tools)**
- **Manual Partitioning**: Interactive cfdisk integration
- **Format Partitions**: Support for ext4, xfs, btrfs, fat32, ntfs
- **Secure Disk Wiping**: Zero, random, and secure erase methods
- **Disk Health Monitoring**: SMART diagnostics and health checks
- **Mount Management**: Mount/unmount partitions with filesystem detection

#### **⚙️ System & Boot Tools (5 tools)**
- **Bootloader Management**: Install/repair GRUB and systemd-boot
- **fstab Generation**: Automatic filesystem table creation
- **System Chroot**: Access installed systems for maintenance
- **Service Management**: Enable/disable systemd services
- **System Information**: Comprehensive hardware and software details

#### **👥 User & Security Tools (5 tools)**
- **User Management**: Create accounts with full configuration
- **Password Reset**: Secure password recovery functionality
- **Group Management**: Add/remove users from groups
- **SSH Configuration**: Server setup with security options
- **Security Auditing**: Comprehensive system security assessment

#### **🌐 Network Tools (4 tools)**
- **Network Configuration**: Interface setup with IP/gateway options
- **Connectivity Testing**: Ping, DNS, and HTTP connectivity tests
- **Firewall Management**: iptables and UFW configuration
- **Network Diagnostics**: Comprehensive network troubleshooting

### 🎨 **User Experience**
- **Intuitive TUI**: Clean, responsive interface with keyboard navigation
- **Parameter Dialogs**: Interactive configuration for complex tools
- **Real-time Output**: Live progress monitoring during operations
- **Smart Validation**: Prevents invalid configurations and dangerous operations
- **Comprehensive Help**: Built-in documentation for all tools

### 🔒 **Security & Reliability**
- **Input Sanitization**: Prevents command injection vulnerabilities
- **Path Validation**: Ensures safe file operations and prevents directory traversal
- **Error Isolation**: Graceful failure recovery prevents cascade failures
- **Secure Scripting**: `set -euo pipefail` on all Bash scripts
- **UUID-based Operations**: Reliable partition identification

## 🏗️ Installation Features

### **Advanced Partitioning**
- **ESP + XBOOTLDR Standard**: Optimal dual-boot compatibility
- **Multiple Filesystems**: ext4, xfs, btrfs with full LVM and LUKS support
- **RAID Support**: Automatic array creation and management
- **Manual Partitioning**: Guided setup with validation

### **Auto-Partitioning Strategies**

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

### **System Configuration**
- **Desktop Environments**: GNOME, KDE, Hyprland, i3, XFCE with auto-configured display managers
- **Bootloaders**: GRUB (BIOS/UEFI) and systemd-boot (UEFI only)
- **Secure Boot**: Support with proper UEFI validation
- **Localization**: Timezone and keymap configuration
- **Package Management**: Interactive Pacman and AUR package selection

## 🛠️ Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Rust TUI Frontend                        │
├─────────────────────────────────────────────────────────────┤
│  Main Menu    │  Guided Installer  │  System Tools          │
│  - Installer  │  - Configuration   │  - Disk Tools          │
│  - Tools      │  - Validation      │  - System Tools        │
│  - Quit       │  - Installation    │  - User Tools          │
│               │                    │  - Network Tools       │
└─────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────┐
│                    Bash Backend                             │
├─────────────────────────────────────────────────────────────┤
│  Installation Scripts    │  System Administration Scripts   │
│  - install.sh            │  - scripts/tools/                │
│  - disk_strategies.sh    │  - 19 specialized tools          │
│  - chroot_config.sh      │  - Full CLI integration          │
│  - Package management    │  - Real-time output              │
└─────────────────────────────────────────────────────────────┘
```

## 📖 Usage Guide

### **TUI Navigation**
- **Arrow Keys**: Navigate menus and options
- **Enter**: Select/configure options
- **Space**: Start operations (when available)
- **Esc**: Cancel/return from dialogs
- **Q**: Quit application

### **CLI Usage**
```bash
# Installation
./archinstall-tui install --config config.json
./archinstall-tui install --save-config config.json

# System Tools
./archinstall-tui tools disk format --device /dev/sda1 --filesystem ext4
./archinstall-tui tools system services --action enable --service sshd
./archinstall-tui tools user add --username newuser --full-name "New User"
./archinstall-tui tools network test --action full --timeout 10

# Help and Documentation
./archinstall-tui tools --help
./archinstall-tui tools disk --help
./archinstall-tui tools disk format --help
```

## 📁 Project Structure

```
archinstall/
├── archinstall-tui          # Main binary (pre-compiled)
├── src/                     # Rust TUI source code
│   ├── main.rs             # Entry point and CLI handling
│   ├── app.rs              # Application logic and state management
│   ├── config.rs           # Configuration management
│   ├── input.rs            # User input and dialogs
│   ├── ui.rs               # TUI rendering and layout
│   ├── cli.rs              # CLI argument definitions
│   ├── config_file.rs      # JSON configuration handling
│   ├── package_utils.rs    # Package search utilities
│   ├── installer.rs        # Installation orchestration
│   ├── scrolling.rs        # Reusable scrolling logic
│   └── error.rs            # Error handling
│
├── scripts/                 # Bash backend scripts
│   ├── install.sh          # Main installation orchestrator
│   ├── install_wrapper.sh  # TUI-friendly output wrapper
│   ├── utils.sh           # Common utility functions
│   ├── disk_utils.sh      # Partitioning utilities
│   ├── disk_strategies.sh # Partitioning strategy dispatcher
│   ├── chroot_config.sh   # Chroot configuration
│   ├── config_loader.sh   # JSON configuration loader
│   ├── strategies/        # Individual partitioning strategies
│   │   ├── simple.sh      # Basic partitioning
│   │   ├── simple_luks.sh # Encrypted partitioning
│   │   ├── lvm.sh         # LVM partitioning
│   │   ├── lvm_luks.sh    # Encrypted LVM
│   │   ├── raid.sh        # RAID partitioning
│   │   ├── raid_luks.sh   # Encrypted RAID
│   │   ├── raid_lvm.sh    # RAID + LVM
│   │   ├── raid_lvm_luks.sh # RAID + LVM + Encryption
│   │   └── manual.sh      # Guided manual partitioning
│   │
│   ├── desktops/          # Desktop environment scripts
│   │   ├── gnome.sh       # GNOME installation
│   │   ├── kde.sh         # KDE installation
│   │   ├── hyprland.sh    # Hyprland installation
│   │   ├── i3.sh          # i3 installation
│   │   ├── xfce.sh        # XFCE installation
│   │   └── none.sh        # No desktop environment
│   │
│   └── tools/             # System administration tools
│       ├── manual_partition.sh     # Manual partitioning
│       ├── format_partition.sh     # Partition formatting
│       ├── wipe_disk.sh           # Secure disk wiping
│       ├── check_disk_health.sh   # Disk health monitoring
│       ├── mount_partitions.sh    # Mount management
│       ├── install_bootloader.sh  # Bootloader management
│       ├── generate_fstab.sh      # fstab generation
│       ├── chroot_system.sh       # System chroot access
│       ├── manage_services.sh     # Service management
│       ├── system_info.sh         # System information
│       ├── add_user.sh           # User management
│       ├── reset_password.sh     # Password reset
│       ├── manage_groups.sh      # Group management
│       ├── configure_ssh.sh      # SSH configuration
│       ├── security_audit.sh     # Security auditing
│       ├── configure_network.sh  # Network configuration
│       ├── test_network.sh       # Connectivity testing
│       ├── configure_firewall.sh # Firewall management
│       └── network_diagnostics.sh # Network diagnostics
│
├── Source/                 # Plymouth themes
│   ├── arch-glow/         # Arch-themed boot splash
│   └── arch-mac-style/    # macOS-inspired theme
│
├── Cargo.toml             # Rust project configuration
├── Makefile              # Development build system
├── README.md             # This file
└── LICENSE               # MIT License
```

## 🔧 Technical Stack

- **Frontend**: Rust with ratatui/crossterm for responsive TUI interface
- **Backend**: Modular Bash scripts with comprehensive error handling
- **CLI**: clap for robust argument parsing and help generation
- **Configuration**: JSON-based configuration files with validation
- **Package Management**: Native pacman integration + AUR API via curl
- **Testing**: Comprehensive Rust test suite with CI/CD pipeline

## 🚀 Development

### **Build from Source**
```bash
# Clone and build
git clone https://github.com/your-username/archinstall.git
cd archinstall
cargo build --release
cp target/release/archinstall-tui .
```

### **Development Workflow**
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

### **Makefile Targets**
```bash
make build          # Build the project
make test           # Run test suite
make lint           # Run linting checks
make iso-ready      # Verify ISO compatibility
make clean          # Clean build artifacts
```

## 📋 Requirements

### **System Requirements**
- **Arch Linux live ISO** (latest recommended)
- **Root privileges** (installer will request)
- **Internet connection** (for package downloads and AUR access)
- **Minimum 8GB RAM** (recommended for smooth operation)
- **UEFI or BIOS** (both supported with automatic detection)

### **Dependencies**
- **Runtime**: None (statically compiled binary)
- **Development**: Rust toolchain, bash, standard Unix tools
- **Installation**: pacman, curl (jq only needed for direct bash script usage, not required for TUI)

## 🔐 Security & Safety Guarantees

ArchInstall TUI is designed with **defense in depth**. Multiple independent safety mechanisms ensure safe operation.

### Core Safety Pillars

| Pillar | Guarantee | Mechanism |
|--------|-----------|-----------|
| **Death Pact** | No orphaned processes | `PR_SET_PDEATHSIG` + process groups |
| **Typed Arguments** | No malformed script calls | `ScriptArgs` trait system |
| **Refusals** | No accidental destruction | Dry-run mode + environment gating |

### Dry-Run Mode

Preview what would happen without making changes:

```bash
# See exactly what would be executed
./archinstall-tui --dry-run install --config my_config.json

# Test tools safely
./archinstall-tui --dry-run tools disk wipe --device /dev/sda --method zero
# Output: [DRY RUN] Skipped: wipe_disk.sh
```

### Process Safety

- **Death Pact**: All child processes terminate if TUI crashes (even SIGKILL)
- **Process Groups**: Entire process trees are signaled together
- **Signal Forwarding**: Bash scripts forward signals to grandchildren

### Destructive Operation Protection

- **Environment Gating**: Scripts refuse without `CONFIRM_*` variables
- **State Machine**: Operations run in validated sequence only
- **Logged Warnings**: All destructive operations are logged before execution

### Traditional Security

- **Input Sanitization**: Prevents command injection vulnerabilities
- **Path Validation**: Ensures safe file operations and prevents directory traversal
- **UUID-based Mounting**: Reliable partition identification
- **Secure Password Handling**: Proper validation and storage
- **Error Isolation**: Prevents cascade failures (`set -euo pipefail`)
- **Permission Checks**: Validates required privileges before operations

### Documentation

For detailed safety information:
- [Safety Model](docs/SAFETY_MODEL.md) - Unified safety overview
- [Process Safety](docs/process-safety.md) - Death pact implementation
- [Destructive Operations](docs/destructive-ops-policy.md) - Data destruction policy
- [Architecture](docs/architecture.md) - System design

## 📄 License

MIT License - See LICENSE file for details

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## 🆘 Support

- **Issues**: Report bugs and request features via GitHub Issues
- **Discussions**: Join community discussions for help and ideas
- **Documentation**: Check the wiki for detailed guides and troubleshooting

## 🎉 What Makes This Special

This isn't just another Arch installer - it's a **complete Arch Linux ecosystem**:

- **Beginner-Friendly**: Intuitive TUI for new users
- **Power User Ready**: Comprehensive CLI tools for system administration
- **Production Quality**: Robust error handling and security measures
- **Modular Design**: Easy to extend and customize
- **Zero Dependencies**: Works immediately on any Arch ISO
- **Professional Grade**: Suitable for both personal and enterprise use

Whether you're installing Arch Linux for the first time or managing a fleet of servers, this toolkit has you covered.
