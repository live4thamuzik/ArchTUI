CI WILL FAIL if these rules are violated.
Do not attempt workarounds.


Bash Rules
❌ source forbidden
❌ read forbidden
❌ /dev/urandom forbidden
✅ trap required in destructive scripts
✅ set -euo pipefail required

Rust Rules
❌ Command::new without .in_new_process_group()
❌ unwrap() without comment
❌ static mut
❌ global mutable state without justification doc
❌ Command::new("pacman") forbidden (Must use alpm bindings)
❌ Parsing stdout for package progress forbidden (Must use log_cb)

Architecture Rules
❌ Script execution without manifest
❌ Destructive op without state validation
❌ Missing documentation for new safety guarantees
