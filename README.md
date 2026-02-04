# ArchTUI

A terminal-based interface for installing and administering Arch Linux. Rust TUI frontend, modular Bash backend.

> **This project is under active development and is not stable.**
> It is incomplete, untested on real hardware in its current state, and will break.
> **Do not use this on systems you care about.**
> Testing should be done in virtual machines or disposable environments only.
> There are no stability guarantees. Interfaces, configuration formats, and behavior are all subject to change without notice.

---

## What it is

ArchTUI is a guided installer and system administration toolkit for Arch Linux. The TUI handles configuration, validation, and sequencing. Bash scripts handle execution. The two layers communicate through typed argument structs and environment contracts — Rust decides what to do, Bash does it.

It is not a replacement for reading the Arch Wiki or understanding how a manual installation works. If you have never installed Arch by hand, do that first.

---

## What it does

**Guided installation** — walks through disk partitioning, filesystem creation, base system install, bootloader setup, user creation, locale/timezone, desktop environment selection, and package installation. Supports automated runs from a saved JSON configuration file.

**System tools** — 25 standalone administration scripts accessible from the TUI or directly via CLI. Disk operations, service management, user/group administration, network configuration, security auditing.

---

## Installation

```
git clone https://github.com/live4thamuzik/ArchTUI.git
cd ArchTUI
```

A pre-compiled binary (`archtui`) ships in the repo. The Arch live ISO does not have the space or tooling to build from source — the Rust toolchain alone exceeds available ISO memory, and the build also requires `jq`. Clone and run directly. To build from source on a full system:

```
cargo build --release
cp target/release/archtui ./
```

Or use the Makefile:

```
make build
```

---

## Usage

### TUI mode (default)

```
./archtui
```

Launches the interactive terminal interface. Navigate with arrow keys, Enter to select, Esc to go back, Q to quit.

### Automated install from config

```
./archtui install --config config.json
```

Runs a headless installation using a previously saved configuration file.

### Save config without installing

```
./archtui install --save-config config.json
```

Walk through the TUI, configure everything, then write the result to a JSON file for later use or review.

### Validate a config file

```
./archtui validate config.json
```

Check a configuration file for errors without running anything.

### Dry-run mode

```
./archtui --dry-run install --config config.json
./archtui --dry-run tools disk wipe --device /dev/sda --method quick --confirm
```

Preview what would be executed. Destructive operations are skipped and logged. Non-destructive operations (health checks, system info) still run.

### CLI tools

```
./archtui tools disk format --device /dev/sda1 --filesystem ext4
./archtui tools disk health --device /dev/sda
./archtui tools disk wipe --device /dev/sda --method secure --confirm
./archtui tools system services --action status --service sshd
./archtui tools system info --detailed
./archtui tools user add --username admin --groups wheel,video
./archtui tools user security --action full
./archtui tools network test --action full
./archtui tools network firewall --action status
```

Run `./archtui tools --help` for the full list. Each subcommand has its own `--help`.

---

## Partitioning strategies

| Strategy | Encryption | RAID | LVM | Description |
|---|---|---|---|---|
| Simple | No | No | No | Single disk, EFI + root |
| Simple + LUKS | Yes | No | No | Single disk, encrypted root |
| LVM | No | No | Yes | Logical volume management |
| LVM + LUKS | Yes | No | Yes | Encrypted LVM |
| RAID | No | Yes | No | Software RAID (mdadm) |
| RAID + LUKS | Yes | Yes | No | Encrypted RAID |
| RAID + LVM | No | Yes | Yes | RAID with LVM on top |
| RAID + LVM + LUKS | Yes | Yes | Yes | Full stack |
| Manual | User choice | User choice | User choice | Guided manual partitioning via cfdisk |

All automated strategies create an EFI System Partition and an XBOOTLDR partition.

## Supported options

**Filesystems:** ext4, xfs, btrfs (with optional snapshots), f2fs, fat32

**Bootloaders:** GRUB (UEFI and BIOS), systemd-boot (UEFI only)

**Kernels:** linux, linux-lts, linux-zen, linux-hardened

**Desktop environments:** GNOME, KDE Plasma, Hyprland, or none

**AUR helpers:** paru, yay, or none

**GPU drivers:** auto-detect, NVIDIA, AMD, Intel

---

## Architecture

```
archtui (Rust)
  |
  |-- TUI: ratatui + crossterm
  |-- CLI: clap
  |-- Config: serde_json
  |-- Process management: nix, signal-hook
  |-- Package queries: alpm (optional, Arch-only)
  |
  v
scripts/ (Bash)
  |-- install.sh              Main installation orchestrator
  |-- strategies/*.sh         9 partitioning strategies
  |-- desktops/*.sh           Desktop environment installers
  |-- tools/*.sh              25 system administration tools
  |-- utils.sh, disk_utils.sh Common utilities
```

Rust owns all state, validation, and sequencing. Bash scripts are stateless executors — they receive typed arguments and environment variables, do the work, and exit. Scripts refuse to run without their expected environment contracts.

### Process safety

All child processes are spawned in isolated process groups. If the TUI exits for any reason — including a crash or SIGKILL — child processes are terminated via `PR_SET_PDEATHSIG`. The TUI registers a global child process registry, handles SIGINT/SIGTERM/SIGHUP, and sends group-wide signals to clean up entire process trees. No orphaned `sgdisk`, `cryptsetup`, or `mkfs` processes.

### Script argument system

Each bash script has a corresponding Rust struct implementing `ScriptArgs`. The struct produces CLI arguments, environment variables, and a script path at compile time. This prevents flag typos, enforces required parameters, and integrates with the dry-run system through an `is_destructive()` marker.

### Destructive operation policy

Destructive operations (disk wipe, format, partition, LUKS setup) require:
- Explicit confirmation flags in environment variables
- Validation before execution
- Logged warnings before any write operation
- Dry-run mode support

---

## Project structure

```
ArchTUI/
|-- src/                    Rust source
|   |-- app/                Application state machine
|   |-- ui/                 TUI rendering
|   |-- components/         Reusable UI components (PTY terminal, dialogs, file browser)
|   |-- scripts/            Typed argument structs for each script category
|   |-- logic/              Pre/post-install orchestration, dependency resolution
|   |-- engine/             Storage abstraction
|   |-- main.rs             Entry point and CLI routing
|   |-- process_guard.rs    Death pact and child process management
|   |-- script_traits.rs    ScriptArgs trait, dry-run mode
|   |-- hardware.rs         Firmware and network detection
|   |-- config.rs           Runtime configuration state
|   |-- config_file.rs      JSON config save/load
|   |-- types.rs            Enums for filesystems, partitions, bootloaders, etc.
|   |-- installer.rs        Installation workflow
|
|-- scripts/
|   |-- install.sh          Main install orchestrator
|   |-- strategies/         Partitioning strategy scripts (9)
|   |-- desktops/           Desktop environment scripts
|   |-- tools/              System administration scripts (25)
|
|-- docs/                   Architecture, safety model, process safety documentation
|-- .github/workflows/      CI: shellcheck + cargo test + cargo build
|-- Cargo.toml
|-- Makefile                Development build targets
```

---

## Building and development

Requires the Rust toolchain. On Arch: `sudo pacman -S rust`

```
make build          # Release build, copies binary to ./archtui
make test           # Run Rust and Bash test suites
make lint           # Clippy with warnings as errors
make format         # rustfmt
make dev            # format + lint + test + build
make clean          # Remove build artifacts
```

The `alpm` feature enables native pacman database queries via libalpm. It is optional and only compiles on Arch Linux where libalpm headers are available. Without it, package operations fall back to CLI pacman calls.

```
cargo build --release --features alpm
```

### Runtime dependencies

The compiled binary has no runtime library dependencies (statically linked, LTO, stripped). The bash scripts expect standard Arch Linux utilities: `pacman`, `sgdisk`, `mkfs.*`, `cryptsetup`, `mdadm`, `arch-chroot`, etc. — all present on the Arch live ISO.

---

## Current status

**This project is in active development.**

What exists and compiles:
- TUI framework, navigation, menus, dialogs, embedded PTY terminal
- Full CLI with subcommands for all 25 tools
- Typed argument system for all script categories
- Process safety (death pact, group signaling, signal handling)
- Dry-run mode
- JSON configuration save/load/validate
- Hardware detection (firmware mode, network state)
- Pre-install orchestration (mirror ranking with network awareness)
- Post-install orchestration (AUR helper, dotfiles — non-fatal)
- 264 unit tests passing
- CI pipeline (shellcheck + cargo test)

What is incomplete or untested:
- End-to-end installation has not been validated on real hardware
- Wiring between TUI configuration and script execution is partially connected
- Error recovery paths are not fully exercised
- Some UI screens are structural but do not yet drive real operations
- ALPM integration is feature-gated and lightly tested
- Btrfs snapshot scheduling is defined but not wired
- No release binaries are published

Do not assume anything works until you have tested it yourself in a disposable environment.
