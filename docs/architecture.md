Sections:

1. Design Goals
-Determinism
-Safety
-Recoverability

2. Rust/Bash Separation
-Control plane vs execution plane
-Why Bash is intentionally “dumb”

3. Install State Machine
-All stages
-Valid transitions
-Failure handling

4. Process Safety Model
-Process groups
-Death signals
-Signal handling
-Why orphaned processes are impossible

5. Destructive Operations Policy
-Confirmation model
-Environment gating
-Logging requirements

6. Why This Is Safer Than Traditional Installers
-Upstream maintainers love this section.
