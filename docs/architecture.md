# ArchTUI Architecture

Architecture and design of ArchTUI, a terminal-based Arch Linux installer.

## 1. Design Goals

### 1.1 Determinism

Every installation produces identical results given identical inputs. The installer:

- Uses a structured configuration as the single source of truth
- Executes scripts in a deterministic order defined by a state machine
- Validates all inputs before beginning destructive operations
- Logs every action for reproducibility

### 1.2 Safety

The installer is designed to fail safely rather than cause partial damage:

- **Fail Fast**: Validation occurs before any destructive operation
- **Death Pact**: All child processes terminate if the TUI crashes
- **Environment Gating**: Destructive operations require explicit environment confirmation
- **No Interactive Prompts**: Scripts cannot request user input mid-execution

### 1.3 Recoverability

When failures occur, the system state is predictable:

- State transitions are logged to disk
- Failed stages are recorded with context
- Partial operations can be identified via logs
- No silent failures or swallowed errors

---

## 2. Rust/Bash Separation

The architecture enforces a strict separation between **control plane** (Rust) and **execution plane** (Bash).

### 2.1 Control Plane (Rust)

Rust owns all decision-making:

| Responsibility | Implementation |
|----------------|----------------|
| State management | `InstallStage` enum with validated transitions |
| Configuration | `InstallerConfig` parsed and validated at startup |
| Sequencing | State machine determines script execution order |
| Policy enforcement | Destructive operations gated by `CONFIRM_*` env vars |
| Process lifecycle | `ProcessGuard` + `ChildRegistry` + `PR_SET_PDEATHSIG` |
| Package management | ALPM bindings (`alpm-rs`), never `Command::new("pacman")` |
| Error handling | `anyhow::Result` with `.context()` on every `?` |

### 2.2 Execution Plane (Bash)

Bash scripts are stateless workers. They execute commands and report exit codes.

| Allowed | Forbidden |
|---------|-----------|
| Execute system commands (`sgdisk`, `cryptsetup`, `mkfs.*`) | Make decisions or branch on policy |
| Report exit codes | Prompt for input (`read` is banned) |
| Log progress via `log_info`, `log_warn`, `log_error` | Change execution order |
| Validate environment contract at entry | Catch and hide errors |
| Forward signals to children | Use `source` (must use `source_or_die`) |

### 2.3 Communication Protocol

```
┌──────────────────────────────────────────────────────────────────┐
│                         RUST (Control)                           │
├──────────────────────────────────────────────────────────────────┤
│  1. Validates configuration                                      │
│  2. Sets environment variables (INSTALL_DISK, CONFIRM_*, etc.)   │
│  3. Spawns bash script in process group                          │
│  4. Streams stdout/stderr back to TUI                            │
│  5. Receives exit code                                           │
│  6. Advances or fails state machine                              │
└──────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌──────────────────────────────────────────────────────────────────┐
│                         BASH (Execution)                         │
├──────────────────────────────────────────────────────────────────┤
│  1. Validates environment contract (refuses without CONFIRM_*)   │
│  2. Executes commands                                            │
│  3. Logs to stdout/stderr                                        │
│  4. Returns exit code (0=success, non-zero=failure)              │
└──────────────────────────────────────────────────────────────────┘
```

Scripts never receive arguments via stdin. Configuration passes via environment variables and CLI flags with typed `ScriptArgs` structs.

---

## 3. Install State Machine

Installation proceeds through a linear sequence of stages defined in `src/install_state.rs`.

```
NotStarted (0)
    │
    ▼
ValidatingConfig (1)       ← Verify user configuration is valid
    │
    ▼
PreparingSystem (2)        ← Sync clock, update mirrors
    │
    ▼
InstallingDependencies (3) ← Install required packages on live system
    │
    ▼
PartitioningDisk (4)       ← [DESTRUCTIVE] Partition and format disk
    │
    ▼
InstallingBaseSystem (5)   ← pacstrap base system
    │
    ▼
GeneratingFstab (6)        ← Generate /etc/fstab
    │
    ▼
ConfiguringChroot (7)      ← Configure locale, users, bootloader, DE
    │
    ▼
Finalizing (8)             ← Cleanup and verification
    │
    ▼
Completed (9)              ← Terminal state: success

    ┌─────────────────────┐
    │ Failed (255)        │ ← Terminal state: any stage can fail
    │ (records stage)     │
    └─────────────────────┘
```

Invalid transitions are compile-time errors. The `advance()` method returns `Result<(), InstallTransitionError>`, and skipping stages returns errors.

When a stage fails:
1. `InstallerContext` records which stage failed
2. Error context is preserved
3. State transitions to `Failed(at_stage)`
4. All child processes are terminated (Death Pact)
5. User sees exactly which stage failed and why

---

## 4. Script Manifest System

Every bash script has a corresponding JSON manifest in `scripts/manifests/` that declares its contract.

```json
{
  "script": "scripts/tools/wipe_disk.sh",
  "description": "Securely wipe a disk",
  "destructive": true,
  "required_confirmation": "CONFIRM_WIPE_DISK",
  "version": "2.0",
  "needs_stdin": false,
  "valid_exit_codes": [0],
  "required_env": [
    {
      "name": "INSTALL_DISK",
      "description": "Target disk device path",
      "pattern": "^/dev/"
    }
  ],
  "optional_env": [
    {
      "name": "WIPE_METHOD",
      "description": "quick, secure, or auto",
      "default": "quick"
    }
  ]
}
```

Validation is defense in depth — both Rust and Bash check requirements:

**Rust (pre-execution):** `manifest.validate_environment()?;`

**Bash (at script start):**
```bash
if [[ "${CONFIRM_WIPE_DISK:-}" != "yes" ]]; then
    error_exit "CONFIRM_WIPE_DISK=yes is required"
fi
```

---

## 5. Directory Structure

```
ArchTUI/
├── src/                      # Rust source code
│   ├── main.rs               # CLI entry point and event loop
│   ├── lib.rs                # Library exports
│   ├── app/                  # Application state machine
│   │   ├── mod.rs            # Event loop, input handling, mode dispatch
│   │   └── state.rs          # AppState, AppMode, shared state
│   ├── ui/                   # UI rendering (modular)
│   │   ├── mod.rs            # Main renderer dispatcher
│   │   ├── menus.rs          # Menu rendering
│   │   ├── dialogs.rs        # Dialog rendering
│   │   ├── installer.rs      # Installation progress UI
│   │   ├── header.rs         # Progress bars, output display
│   │   ├── screens.rs        # Mode-specific screens
│   │   └── descriptions.rs   # Tool descriptions
│   ├── components/           # Reusable UI widgets
│   │   ├── pty_terminal.rs   # Embedded terminal (portable-pty)
│   │   ├── floating_window.rs # Overlay windows with progress
│   │   ├── file_browser.rs   # Config file selection
│   │   ├── confirm_dialog.rs # Yes/No confirmations
│   │   ├── keybindings.rs    # Context-aware keyboard shortcuts
│   │   ├── help_overlay.rs   # Help display
│   │   └── nav_bar.rs        # Bottom navigation bar
│   ├── install_state.rs      # State machine
│   ├── installer.rs          # Script execution + output streaming
│   ├── process_guard.rs      # Death pact implementation
│   ├── script_manifest.rs    # Manifest validation
│   ├── config.rs             # Configuration options
│   ├── config_file.rs        # Config file I/O (JSON)
│   ├── input.rs              # Input handling and dialogs
│   ├── types.rs              # Enums (Filesystem, Bootloader, DE, etc.)
│   ├── profiles/             # Desktop environment package profiles
│   ├── engine/               # Storage planning engine
│   ├── logic/                # Pre/post-install logic, package resolver
│   └── scripts/              # ScriptArgs structs (typed arguments)
│
├── scripts/                  # Bash backend
│   ├── install.sh            # Main installation orchestrator
│   ├── install_wrapper.sh    # TUI output wrapper
│   ├── chroot_config.sh      # In-chroot configuration
│   ├── disk_utils.sh         # Disk operations
│   ├── disk_strategies.sh    # Strategy dispatcher
│   ├── utils.sh              # Logging, validation, common functions
│   ├── config_loader.sh      # JSON config → environment variables
│   ├── run_as_user.sh        # Unprivileged execution helper
│   ├── strategies/           # 10 partitioning strategies
│   ├── tools/                # 28 system admin tools
│   ├── manifests/            # JSON script contracts
│   └── tests/                # BATS test suite
│
├── tests/                    # Rust integration tests
├── docs/                     # Project documentation
├── Source/                   # Plymouth themes
└── Cargo.toml                # Rust dependencies
```

---

## 6. Application Modes

The TUI operates as a mode-based state machine with 16 modes defined in `src/app/state.rs`:

- `MainMenu` — Entry point
- `GuidedInstaller` — Step-by-step configuration
- `AutomatedInstall` — Config file-based installation
- `ToolsMenu` / `DiskTools` / `SystemTools` / `UserTools` / `NetworkTools` — Admin tools
- `ToolDialog` — Tool parameter input
- `Installation` — Active installation progress
- `Complete` — Installation finished
- `EmbeddedTerminal` — PTY for interactive tools (cfdisk, etc.)
- `FloatingOutput` — Overlay output window
- `FileBrowser` — Config file selection
- `ConfirmDialog` — Destructive operation confirmation

---

## 7. Partitioning Strategies

Ten disk layout options in `scripts/strategies/`:

| Strategy | File | Description |
|----------|------|-------------|
| Simple | `simple.sh` | Basic root + EFI |
| Simple + LUKS | `simple_luks.sh` | Encrypted simple |
| LVM | `lvm.sh` | LVM partitioning |
| LVM + LUKS | `lvm_luks.sh` | Encrypted LVM |
| RAID | `raid.sh` | Software RAID (mdadm) |
| RAID + LUKS | `raid_luks.sh` | Encrypted RAID |
| RAID + LVM | `raid_lvm.sh` | RAID with LVM |
| RAID + LVM + LUKS | `raid_lvm_luks.sh` | Full stack |
| Manual | `manual.sh` | User-guided via cfdisk |
| Pre-mounted | `pre_mounted.sh` | Use existing mounts |

---

## 8. Package Management: ALPM vs Bash

The installer uses a hybrid approach: ALPM bindings for package operations, Bash for disk/system operations.

| Operation | Implementation | Reason |
|-----------|----------------|--------|
| Package install | ALPM (Rust) | Type-safe, progress callbacks |
| Package queries | ALPM (Rust) | Structured data, not string parsing |
| Disk partitioning | Bash | `sgdisk` CLI only |
| Disk formatting | Bash | `mkfs.*` CLI only |
| LUKS encryption | Bash | `cryptsetup` CLI, security-sensitive |
| Bootloader install | Bash | `grub-install` CLI, `arch-chroot` needed |

---

## 9. Data Flow

### Configuration

```
User (TUI) → config.rs → Environment Variables → config_loader.sh → Bash scripts
```

Key environment variables: `INSTALL_DISK`, `BOOTLOADER`, `FILESYSTEM`, `PARTITIONING_STRATEGY`, `DESKTOP_ENVIRONMENT`, `USERNAME`, `HOSTNAME`, `TIMEZONE`, `LOCALE`.

### Installation

```
1. User configures options in TUI
2. Rust validates and exports config as env vars
3. install.sh runs via install_wrapper.sh
4. Output streams back to TUI (stdout line parsing)
5. chroot_config.sh runs in target system
6. TUI shows progress and completion
```

---

## 10. Error Handling

**Bash:** All scripts use `set -euo pipefail`. Cleanup traps for unmounting on failure. `log_*` functions for consistent logging. Exit codes: 0=success, non-zero=error.

**Rust:** `anyhow` with `.context()` on every fallible call. Graceful degradation for non-critical failures. Mutex poisoning recovered via `unwrap_or_else(|e| e.into_inner())`.

---

## 11. Testing

- **Rust unit tests:** State machine, config parsing, manifest validation, resolver, input handling
- **Rust integration tests:** Death pact, process lifecycle, script execution
- **BATS tests:** Bash script validation, config loading, strategy helpers
- **CI:** `cargo fmt --check` + `cargo clippy -- -D warnings` + `cargo test` + shellcheck + BATS + `cargo audit`

---

## 12. Extending the Project

### Adding a Partitioning Strategy
1. Create `scripts/strategies/new_strategy.sh`
2. Add JSON manifest in `scripts/manifests/`
3. Add option in `disk_strategies.sh`
4. Add enum variant in `types.rs` and UI option in `config.rs`

### Adding a Desktop Environment
1. Add variant to `DesktopEnvironment` enum in `types.rs`
2. Add package list and service enable logic in `chroot_config.sh`
3. Add profile mapping in `profiles/mod.rs` via `desktop_to_profile()`
4. Add resolver packages in `logic/resolver.rs`

### Adding a System Tool
1. Create `scripts/tools/new_tool.sh`
2. Add JSON manifest in `scripts/manifests/`
3. Add `ScriptArgs` struct in `src/scripts/`
4. Add to appropriate menu in `app/mod.rs`
5. Add description in `ui/descriptions.rs`
