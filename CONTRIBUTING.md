# Contributing to ArchTUI

Thank you for your interest in contributing to ArchTUI! This document provides guidelines for contributing to the project.

## Architecture Overview

ArchTUI uses a two-layer architecture:
- **Rust Layer**: Controls the TUI interface, spawns bash scripts, manages process groups
- **Bash Layer**: Handles actual system operations (partitioning, installation, configuration)

Scripts communicate via **environment variables**, never stdin (for security and process isolation).

## Development Setup

```bash
# Clone and build
git clone <repo-url>
cd ArchTUI
cargo build --no-default-features

# Run tests
cargo test --no-default-features

# Validate bash syntax
bash -n scripts/disk_utils.sh
bash -n scripts/strategies/*.sh
bash -n scripts/tools/*.sh
```

## Adding a New Tool Script

1. **Create the script** in `scripts/tools/your_tool.sh`:
   ```bash
   #!/bin/bash
   # your_tool.sh - Description of what it does
   set -euo pipefail

   # Source common utilities
   SCRIPT_DIR="$(dirname "${BASH_SOURCE[0]}")"
   source_or_die "$SCRIPT_DIR/../utils.sh"

   # Signal handling
   trap 'cleanup_and_exit SIGTERM' SIGTERM
   trap 'cleanup_and_exit SIGINT' SIGINT

   # Parse arguments (use --flags, not positional args)
   while [[ $# -gt 0 ]]; do
       case "$1" in
           --option) OPTION="$2"; shift 2 ;;
           *) error_exit "Unknown argument: $1" ;;
       esac
   done

   # Implementation...
   ```

2. **Create manifest** in `scripts/manifests/your_tool.json`:
   ```json
   {
     "script": "scripts/tools/your_tool.sh",
     "description": "Description of what the tool does",
     "destructive": false,
     "version": "1.0",
     "needs_stdin": false,
     "valid_exit_codes": [0],
     "required_env": [],
     "optional_env": []
   }
   ```
   For destructive scripts, add:
   ```json
   "destructive": true,
   "required_confirmation": "CONFIRM_YOUR_TOOL",
   ```

3. **Add Rust args struct** in `src/scripts/`:
   ```rust
   pub struct YourToolArgs {
       pub option: String,
   }

   impl ScriptArgs for YourToolArgs {
       fn script_name(&self) -> &str {
           "your_tool.sh"
       }

       fn to_cli_args(&self) -> Vec<String> {
           vec!["--option".into(), self.option.clone()]
       }

       fn is_destructive(&self) -> bool {
           false
       }
   }
   ```

4. **Wire into TUI** in `src/app/mod.rs` (if needed for UI)

## Adding a Partition Strategy

1. **Create strategy** in `scripts/strategies/your_strategy.sh`

2. **Follow the standard pattern**:
   ```bash
   #!/bin/bash
   # your_strategy.sh - Description
   set -euo pipefail

   SCRIPT_DIR="$(dirname "${BASH_SOURCE[0]}")"
   source_or_die "$SCRIPT_DIR/../disk_utils.sh"

   execute_your_strategy_partitioning() {
       # Setup cleanup trap FIRST
       setup_partitioning_trap

       # 1. Create partitions (ESP, boot, root, home)
       # 2. Format filesystems
       # 3. Mount in correct order (/mnt, /mnt/boot, /mnt/efi)
       # 4. Capture UUIDs for bootloader
       capture_device_info "root" "$root_device"

       # 5. Log completion
       log_partitioning_complete "your_strategy"
   }

   export -f execute_your_strategy_partitioning
   ```

3. **Required exports** (for bootloader configuration):
   - `ROOT_UUID` - Root partition UUID
   - `LUKS_UUID` - LUKS container UUID (if encrypted)
   - `SWAP_UUID` - Swap partition UUID (if swap enabled)

## Code Style

### Bash Scripts

- Always use `set -euo pipefail`
- Use `source_or_die` instead of bare `source`
- Never use `eval` - use array expansion for commands
- Never use `read` from stdin - use environment variables
- Add trap handlers for cleanup on error/interrupt
- Use `log_info`, `log_error`, `log_success` from utils.sh

### Rust Code

- Follow standard Rust formatting (`cargo fmt`)
- Use `ArchTuiError` for error handling, not anyhow
- All `Command::new` calls must use `.in_new_process_group()`
- Recover from mutex poisoning, don't panic

## Testing

### Rust Tests
```bash
cargo test --no-default-features
```

### Bash Syntax Validation
```bash
# All scripts
for f in scripts/**/*.sh; do bash -n "$f" && echo "$f: OK"; done
```

### Security Checks
```bash
# No eval (except safe contexts)
grep -r "eval " scripts/

# No bare read (should use -r)
grep -r "read " scripts/ | grep -v "read -r"
```

### Generate Documentation
```bash
# Generate Rust docs (includes private items)
cargo doc --no-deps --document-private-items --no-default-features

# Or use the helper script
./scripts/generate_docs.sh --open
```

## Pull Request Checklist

- [ ] Code follows style guidelines
- [ ] All bash scripts pass `bash -n` syntax check
- [ ] Rust code compiles with `cargo check --no-default-features`
- [ ] All tests pass with `cargo test --no-default-features`
- [ ] New scripts have manifests
- [ ] Destructive operations require confirmation env var
- [ ] Documentation updated if needed

## Questions?

Open an issue for questions or discussion about changes before submitting large PRs.
