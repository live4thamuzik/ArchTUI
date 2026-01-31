# 🐧 ArchInstall TUI (Architectural Preview)

> ⚠️ **STATUS: PRE-ALPHA / UNDER ACTIVE REFACTORING**
>
> This project is currently undergoing a massive architectural hardening phase (Sprint 0/1).
> **DO NOT USE ON PRODUCTION SYSTEMS YET.**
>
> *Current Focus: Process isolation, error handling, and backend stability.*

**ArchInstall TUI** is a hybrid installer for Arch Linux. It utilizes a **Rust** frontend for a type-safe, crash-resilient User Interface, and a strict **Bash** backend for system operations.

Unlike simple wrapper scripts, this project enforces **Systems Programming Standards** to prevent partial installs, zombie processes, and data corruption on failure.

---

## 🛡️ Architecture & Safety Standards

We prioritize **safety over features**. The codebase follows the **"ArchInstall Standard"**:

### 1. The "Death Pact" (Process Isolation)
The Rust frontend acts as a Supervisor. It spawns backend scripts in dedicated **Process Groups**.
* **Behavior:** If the TUI crashes, panics, or receives `SIGINT` (Ctrl+C), it sends `SIGTERM` to the entire process group.
* **Result:** No orphaned `mkfs` or `pacstrap` processes continue running in the background. If the UI dies, the operation dies instantly.

### 2. Zero-Trust Backend
Bash scripts are treated as "untrusted workers."
* **Strict Mode:** All scripts run with `set -euo pipefail`.
* **No Global State:** Scripts do not rely on `$PWD` or environmental accidents. They use strict dependency injection.
* **Structured IPC:** Backend scripts communicate via JSON logs or strict status codes. They do not print raw text to stdout.

### 3. Type-Safe Configuration
* **Rust:** Uses Enums (`BootMode`, `Filesystem`) to represent state. "Impossible configurations" are rejected at compile time or before serialization.
* **Bash:** Receives configuration via validated JSON files (`config.json`), never via loose CLI flags.

---

## 🗺️ Implementation Roadmap & Status

Use this matrix to track actual progress. **If it is not marked ✅, do not assume it works.**

### 🟢 Phase 1: Core Engine (Sprint 0 - COMPLETED)
- [x] **Rust TUI Frontend:** Event loop, state management, and page navigation.
- [x] **Process Guard:** `ChildRegistry` implementation for tracking and killing subprocesses.
- [x] **Backend Hardening:** `utils.sh` with `source_or_die` and `init_script` lifecycle hooks.
- [x] **Config Schema:** Rust structs serialization to JSON.

### 🟡 Phase 2: Verification (Sprint 1 - IN PROGRESS)
- [ ] **Integration Tests:** Verify `process_guard.rs` actually kills children.
- [ ] **Sanity Checks:** Pre-flight check for `jq`, `sgdisk`, `mkfs`.
- [ ] **Dry-Run Mode:** Simulator for partition logic.

### 🔴 Phase 3: Feature Parity (Sprint 2 - PLANNED)
*Existing scripts are currently being refactored to meet Phase 1 standards.*

| Feature | Status | Notes |
| :--- | :--- | :--- |
| **Simple Partitioning** | 🚧 Refactoring | Porting to new `disk_strategies.sh` |
| **LVM Support** | ⏳ Pending | Script exists, needs IPC update |
| **LUKS Encryption** | ⏳ Pending | Script exists, needs IPC update |
| **RAID Support** | ⏳ Pending | Strategy logic defined, untested |
| **Bootloader (GRUB)** | ⏳ Pending | Basic installation logic only |
| **Desktop Environments** | ⏳ Pending | Scripts for Gnome/KDE exist, untested |

---

## 🛠️ Development & Building

### Prerequisites
* **Rust:** stable toolchain (`cargo`)
* **System Tools:** `jq` (required for backend JSON parsing), `bash` (v5+)

### Build
```bash
# Build the TUI
cargo build --release

# Run unit tests (Rust)
cargo test

# Run backend tests (Bats)
./run_tests.sh
```

## 🚀 Development

### **Build from Source**
```bash
# Clone and build
git clone https://github.com/your-username/archinstall.git
cd archinstall
cargo build --release
cp target/release/archinstall-tui .
```
### Running (Dev Mode)
To run the installer without installing it to the system:

```bash
# Ensure you are root (required for disk ops)
sudo ./target/release/archinstall-tui
```

## 📂 Project Structure
├── src/                  # Rust Frontend (The Brain)
│   ├── process_guard.rs  # The Death Pact implementation
│   ├── config_file.rs    # JSON Serialization
│   └── installer.rs      # IPC Controller
├── scripts/              # Bash Backend (The Muscle)
│   ├── strategies/       # Partitioning logic (Worker)
│   ├── tools/            # System admin tools (Worker)
│   ├── install.sh        # Main Orchestrator
│   └── utils.sh          # Safety primitives
└── tests/                # Integration tests

## 🤝 Contributing
Read the code first. This is not a standard shell script.
 1. Any new Bash script MUST source utils.sh and call init_script.
 2. Any new Rust config option MUST use an Enum, not a String.
 3. Destructive operations must be gated behind user confirmation.
