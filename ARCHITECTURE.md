# ArchTUI Architecture

This document describes the architecture and design of the ArchTUI project.

## Overview

ArchTUI is a hybrid Rust/Bash application for installing Arch Linux. The architecture uses Rust for the terminal user interface (TUI) and orchestration, while leveraging Bash scripts for actual system operations.

```
┌─────────────────────────────────────────────────────────────────┐
│                        User Interface                            │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │              Rust TUI (ratatui + crossterm)              │   │
│  │  - Menu navigation                                        │   │
│  │  - Configuration dialogs                                  │   │
│  │  - Progress display                                       │   │
│  │  - Embedded terminal (PTY)                               │   │
│  └──────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Configuration Layer                           │
│  ┌─────────────────────┐    ┌─────────────────────────────┐    │
│  │  config.rs          │    │  config_file.rs             │    │
│  │  - Runtime config   │◄──►│  - JSON/TOML persistence    │    │
│  │  - UI state         │    │  - Import/Export            │    │
│  └─────────────────────┘    └─────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼ Environment Variables
┌─────────────────────────────────────────────────────────────────┐
│                    Bash Script Layer                             │
│  ┌────────────────┐  ┌────────────────┐  ┌────────────────┐    │
│  │  install.sh    │  │ chroot_config  │  │  disk_utils    │    │
│  │  (main entry)  │──►│  (in-chroot)   │  │  (partitioning)│    │
│  └────────────────┘  └────────────────┘  └────────────────┘    │
│           │                                      ▲               │
│           └──────────────────────────────────────┘               │
│                              │                                   │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │                    Strategy Scripts                      │    │
│  │  simple │ lvm │ raid │ luks │ raid_lvm_luks │ manual    │    │
│  └─────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────┘
```

## Directory Structure

```
archinstall/
├── src/                      # Rust source code
│   ├── main.rs              # CLI entry point and event loop
│   ├── app.rs               # Application state machine (3,335 LOC)
│   ├── ui/                  # UI rendering (modular)
│   │   ├── mod.rs           # Main renderer dispatcher
│   │   ├── menus.rs         # Menu rendering
│   │   ├── dialogs.rs       # Dialog rendering
│   │   ├── installer.rs     # Installation UI
│   │   ├── header.rs        # Common widgets
│   │   └── descriptions.rs  # Tool descriptions
│   ├── input.rs             # Input handling and dialogs
│   ├── config.rs            # Configuration options
│   ├── config_file.rs       # Config file I/O
│   ├── components/          # Reusable UI components
│   ├── installer.rs         # Script execution
│   └── ...
│
├── scripts/                  # Bash backend
│   ├── install.sh           # Main installation orchestrator
│   ├── install_wrapper.sh   # TUI output wrapper
│   ├── chroot_config.sh     # In-chroot configuration
│   ├── disk_utils.sh        # Disk operations
│   ├── disk_strategies.sh   # Strategy dispatcher
│   ├── utils.sh             # Common utilities
│   ├── config_loader.sh     # JSON config loader
│   ├── strategies/          # Partitioning strategies (9)
│   ├── desktops/            # DE installation scripts (6)
│   ├── tools/               # System admin tools (19)
│   └── tests/               # BATS test suite
│
├── tests/                    # Rust integration tests
├── Source/                   # Plymouth themes
└── Cargo.toml               # Rust dependencies
```

## Component Details

### Rust Frontend

#### Application State (`app.rs`)
The central state machine managing 13+ application modes:
- `MainMenu` - Entry point
- `GuidedInstaller` - Step-by-step configuration
- `AutomatedInstall` - Config file-based installation
- `ToolsMenu` / `DiskTools` / `SystemTools` / `UserTools` / `NetworkTools` - Admin tools
- `Installation` - Active installation progress
- `EmbeddedTerminal` - PTY for interactive tools
- And more...

#### UI Rendering (`ui/`)
Modular rendering system using ratatui:
- `mod.rs` - Main dispatcher routing to mode-specific renderers
- `menus.rs` - All menu screens with selection highlighting
- `dialogs.rs` - Input dialogs, confirmation dialogs, floating windows
- `installer.rs` - Installation progress, output display
- `header.rs` - ASCII art header, nav bar, progress bars
- `descriptions.rs` - Tool and option descriptions

#### Components (`components/`)
Reusable UI widgets:
- `pty_terminal.rs` - Embedded terminal using portable-pty
- `floating_window.rs` - Overlay windows with progress
- `file_browser.rs` - Config file selection
- `confirm_dialog.rs` - Yes/No confirmations
- `keybindings.rs` - Context-aware keyboard shortcuts
- `help_overlay.rs` - Help display
- `nav_bar.rs` - Bottom navigation bar

### Bash Backend

#### Main Scripts
- `install.sh` - Orchestrates the installation phases
- `install_wrapper.sh` - Wraps output for TUI consumption
- `chroot_config.sh` - Runs inside chroot for system setup
- `disk_utils.sh` - Partition detection, formatting, mounting
- `utils.sh` - Logging, validation, common functions

#### Partitioning Strategies (`strategies/`)
Nine disk layout options:
1. `simple.sh` - Basic root + EFI
2. `simple_luks.sh` - Encrypted simple
3. `lvm.sh` - LVM partitioning
4. `lvm_luks.sh` - Encrypted LVM
5. `raid.sh` - Software RAID
6. `raid_luks.sh` - Encrypted RAID
7. `raid_lvm.sh` - RAID + LVM
8. `raid_lvm_luks.sh` - Full stack (RAID + LVM + LUKS)
9. `manual.sh` - User-guided partitioning

#### Desktop Environments (`desktops/`)
- `gnome.sh`, `kde.sh`, `hyprland.sh`, `i3.sh`, `xfce.sh`, `none.sh`

## Data Flow

### Rust → Bash Interface
Configuration passes via environment variables:
```
Rust (config.rs) → Environment Variables → Bash (config_loader.sh)
```

Key variables:
- `INSTALL_DISK` - Target disk
- `BOOTLOADER` - grub/systemd-boot
- `FILESYSTEM` - ext4/btrfs/xfs
- `PARTITIONING_STRATEGY` - simple/lvm/raid/etc
- `DESKTOP_ENVIRONMENT` - gnome/kde/hyprland/etc
- `USERNAME`, `HOSTNAME`, `TIMEZONE`, `LOCALE`

### Installation Flow
```
1. User configures options in TUI
2. Rust exports config as env vars
3. install.sh runs with install_wrapper.sh
4. Output streams back to TUI (stdout parsing)
5. chroot_config.sh runs in target system
6. TUI shows progress and completion
```

## Error Handling

### Bash Scripts
- All scripts use `set -euo pipefail`
- Cleanup traps for unmounting on failure
- `log_*` functions for consistent logging
- Exit codes: 0=success, 1=error, 2=user cancel

### Rust
- `anyhow` for error propagation
- Graceful degradation for non-critical failures
- Confirmation dialogs for destructive operations

## Security Considerations

- Passwords handled in memory only (not logged)
- LUKS passphrase input uses secure prompts
- No default passwords
- Confirmation dialogs for:
  - Disk formatting
  - Disk wiping
  - Service enabling

## Testing

### Rust Tests
- Unit tests in respective modules
- Integration tests in `tests/`
- Run: `cargo test`

### Bash Tests
- BATS framework in `scripts/tests/`
- Mock system commands
- Run: `./scripts/tests/run_tests.sh`

## Key Design Decisions

1. **Hybrid Architecture**: Rust for UI safety, Bash for system operations
2. **Strategy Pattern**: Partitioning strategies as separate scripts
3. **Environment Variables**: Simple, debuggable interface between Rust and Bash
4. **PTY Integration**: Enables interactive tools (cfdisk, etc.) within TUI
5. **Modular UI**: Separate files for different UI concerns
6. **No External Deps**: Statically compiled binary for ISO distribution

## Extending the Project

### Adding a Partitioning Strategy
1. Create `scripts/strategies/new_strategy.sh`
2. Implement `prepare_disk()` function
3. Add option in `disk_strategies.sh`
4. Add UI option in `config.rs`

### Adding a Desktop Environment
1. Create `scripts/desktops/new_de.sh`
2. Implement package lists and services
3. Add option in Rust configuration

### Adding a System Tool
1. Create `scripts/tools/category/new_tool.sh`
2. Add to appropriate menu in `app.rs`
3. Add description in `ui/descriptions.rs`
