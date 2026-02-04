# ArchTUI

**ArchTUI** — an experimental terminal-based UI (TUI) interface and tooling environment for Arch Linux.

> ⚠️ **Under development.** This project is *not complete*. It is not stable and **should not** be used on personal, production, or critical systems. Use only in disposable or test environments.

---

## What it is

ArchTUI is a Rust-based terminal user interface framework with utilities aimed at assisting with Arch Linux workflows.

The user interface itself (menus, windows, navigation, layout) is mostly implemented. Most unfinished work lies in connecting the UI to real logic and system actions — the “engine and transmission”. The structure exists; much of the functionality is still being wired up.

---

## Features (planned / intended)

These are the intended goals of the project. Most are **not finished yet**:

- **Guided installer**  
  A semi-interactive installer allowing users to customize an Arch Linux installation with a wide range of options.  
  While this is aimed at helping newcomers, learning a manual installation via the Arch Wiki is strongly encouraged to properly understand the system.

- **Known good configuration uploads**  
  Ability to import and apply verified configurations for fast, repeatable deployments.

- **Troubleshooting tools**  
  A collection of TUI-based utilities for common Arch-related tasks, including:
  - Disk and partition tools
  - User and group management
  - Network and security helpers

---

## What works now

At the current stage you can:

- Run the TUI and navigate most UI screens
- View menus and pages representing intended features
- Explore the application structure and UI flow
- Contribute wiring and feature implementations

ArchTUI does **not** yet perform meaningful system operations such as installing Arch, modifying disks, or applying configurations.

---

## Limitations

- Large portions of logic are unimplemented or placeholders
- Error handling is incomplete
- Crashes and undefined behavior are expected
- No safety guarantees are provided

Do **not** run this on real systems.

---

## Building and running

Example workflow (adjust as needed):

```sh
git clone https://github.com/live4thamuzik/ArchTUI.git
cd ArchTUI

# build (requires Rust toolchain)
cargo build

# run
cargo run
