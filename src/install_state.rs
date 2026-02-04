//! Install State Machine
//!
//! This module provides an authoritative, Rust-side source of truth for installation progress.
//! It enforces valid state transitions and makes it impossible to skip stages programmatically.
//!
//! # Design Principles
//!
//! - **Single Source of Truth**: The `InstallerContext` owns the current stage
//! - **Validated Transitions**: Only forward transitions to the next stage are allowed
//! - **No Global State**: State is owned by `InstallerContext`, not global/static
//! - **Fail Fast**: Invalid transitions return errors immediately
//!
//! # Stage Flow
//!
//! ```text
//! NotStarted
//!     ↓
//! ValidatingConfig
//!     ↓
//! PreparingSystem
//!     ↓
//! InstallingDependencies
//!     ↓
//! PartitioningDisk
//!     ↓
//! InstallingBaseSystem
//!     ↓
//! GeneratingFstab
//!     ↓
//! ConfiguringChroot
//!     ↓
//! Finalizing
//!     ↓
//! Completed
//!
//! (Any stage can transition to Failed)
//! ```

// Library API - these types are exported for external use but not yet consumed by the binary
#![allow(dead_code)]

use crate::hardware::{FirmwareMode, HardwareInfo, NetworkState};
use std::fmt;
use thiserror::Error;

/// Installation stages in sequential order.
///
/// Each stage represents a distinct phase of the Arch Linux installation process.
/// Stages are ordered and can only progress forward (except for failure transitions).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum InstallStage {
    /// Installation has not started yet
    NotStarted = 0,

    /// Phase 1: Validating user configuration
    ValidatingConfig = 1,

    /// Phase 2: Preparing the live system (clock sync, mirrors)
    PreparingSystem = 2,

    /// Phase 3: Installing required dependencies
    InstallingDependencies = 3,

    /// Phase 4: Partitioning and formatting the target disk
    /// This is a DESTRUCTIVE stage - requires explicit confirmation
    PartitioningDisk = 4,

    /// Phase 5: Installing base system via pacstrap
    InstallingBaseSystem = 5,

    /// Phase 6: Generating /etc/fstab
    GeneratingFstab = 6,

    /// Phase 7: Configuring the system inside chroot
    /// (locale, users, bootloader, desktop environment)
    ConfiguringChroot = 7,

    /// Phase 8: Final cleanup and verification
    Finalizing = 8,

    /// Installation completed successfully (terminal state)
    Completed = 9,

    /// Installation failed (terminal state)
    /// Contains the stage at which failure occurred
    Failed = 255,
}

impl InstallStage {
    /// Returns the numeric order of this stage (0-9, 255 for Failed)
    #[inline]
    pub const fn order(self) -> u8 {
        self as u8
    }

    /// Returns true if this is a terminal state (Completed or Failed)
    #[inline]
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Failed)
    }

    /// Returns true if this stage involves destructive disk operations
    #[inline]
    pub const fn is_destructive(self) -> bool {
        matches!(self, Self::PartitioningDisk)
    }

    /// Returns the next stage in the sequence, or None if at a terminal state
    pub const fn next(self) -> Option<Self> {
        match self {
            Self::NotStarted => Some(Self::ValidatingConfig),
            Self::ValidatingConfig => Some(Self::PreparingSystem),
            Self::PreparingSystem => Some(Self::InstallingDependencies),
            Self::InstallingDependencies => Some(Self::PartitioningDisk),
            Self::PartitioningDisk => Some(Self::InstallingBaseSystem),
            Self::InstallingBaseSystem => Some(Self::GeneratingFstab),
            Self::GeneratingFstab => Some(Self::ConfiguringChroot),
            Self::ConfiguringChroot => Some(Self::Finalizing),
            Self::Finalizing => Some(Self::Completed),
            Self::Completed | Self::Failed => None,
        }
    }

    /// Returns the previous stage in the sequence, or None if at NotStarted or Failed
    pub const fn previous(self) -> Option<Self> {
        match self {
            Self::ValidatingConfig => Some(Self::NotStarted),
            Self::PreparingSystem => Some(Self::ValidatingConfig),
            Self::InstallingDependencies => Some(Self::PreparingSystem),
            Self::PartitioningDisk => Some(Self::InstallingDependencies),
            Self::InstallingBaseSystem => Some(Self::PartitioningDisk),
            Self::GeneratingFstab => Some(Self::InstallingBaseSystem),
            Self::ConfiguringChroot => Some(Self::GeneratingFstab),
            Self::Finalizing => Some(Self::ConfiguringChroot),
            Self::Completed => Some(Self::Finalizing),
            Self::NotStarted | Self::Failed => None,
        }
    }

    /// Returns a human-readable description of this stage
    pub const fn description(self) -> &'static str {
        match self {
            Self::NotStarted => "Not started",
            Self::ValidatingConfig => "Validating configuration",
            Self::PreparingSystem => "Preparing system",
            Self::InstallingDependencies => "Installing dependencies",
            Self::PartitioningDisk => "Partitioning disk",
            Self::InstallingBaseSystem => "Installing base system",
            Self::GeneratingFstab => "Generating fstab",
            Self::ConfiguringChroot => "Configuring system",
            Self::Finalizing => "Finalizing installation",
            Self::Completed => "Installation complete",
            Self::Failed => "Installation failed",
        }
    }

    /// Returns the approximate progress percentage for this stage
    pub const fn progress_percent(self) -> u8 {
        match self {
            Self::NotStarted => 0,
            Self::ValidatingConfig => 5,
            Self::PreparingSystem => 10,
            Self::InstallingDependencies => 15,
            Self::PartitioningDisk => 25,
            Self::InstallingBaseSystem => 45,
            Self::GeneratingFstab => 60,
            Self::ConfiguringChroot => 75,
            Self::Finalizing => 95,
            Self::Completed => 100,
            Self::Failed => 0, // Progress is meaningless for failed state
        }
    }

    /// Returns all stages in order (excluding Failed)
    pub const fn all_stages() -> &'static [Self] {
        &[
            Self::NotStarted,
            Self::ValidatingConfig,
            Self::PreparingSystem,
            Self::InstallingDependencies,
            Self::PartitioningDisk,
            Self::InstallingBaseSystem,
            Self::GeneratingFstab,
            Self::ConfiguringChroot,
            Self::Finalizing,
            Self::Completed,
        ]
    }
}

impl fmt::Display for InstallStage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.description())
    }
}

/// Errors that can occur during state transitions
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum InstallTransitionError {
    /// Attempted to skip one or more stages
    #[error("Cannot skip from {from} to {to} (must transition through intermediate stages)")]
    SkippedStage {
        from: InstallStage,
        to: InstallStage,
    },

    /// Attempted to go backwards (not allowed)
    #[error("Cannot go backwards from {from} to {to} (installation is forward-only)")]
    BackwardTransition {
        from: InstallStage,
        to: InstallStage,
    },

    /// Attempted to transition from a terminal state
    #[error("Cannot transition from terminal state {from} (installation is {}", if *from == InstallStage::Completed { "complete" } else { "failed" })]
    FromTerminalState { from: InstallStage },

    /// Attempted a transition that requires confirmation without providing it
    #[error("Stage {stage} requires explicit confirmation (destructive operation)")]
    MissingConfirmation { stage: InstallStage },

    /// Attempted to transition to the same state
    #[error("Already at stage {stage}")]
    AlreadyAtStage { stage: InstallStage },
}

/// Context for tracking installation state.
///
/// This struct owns the current installation stage and provides validated
/// transition methods. It ensures that stages cannot be skipped and that
/// transitions only move forward (except for failure).
///
/// # Example
///
/// ```
/// use archtui::install_state::{InstallerContext, InstallStage};
///
/// let mut ctx = InstallerContext::new();
/// assert_eq!(ctx.current_stage(), InstallStage::NotStarted);
///
/// // Advance to next stage
/// ctx.advance().unwrap();
/// assert_eq!(ctx.current_stage(), InstallStage::ValidatingConfig);
///
/// // Cannot skip stages
/// assert!(ctx.transition_to(InstallStage::PartitioningDisk).is_err());
/// ```
#[derive(Debug, Clone)]
pub struct InstallerContext {
    /// Current installation stage
    current: InstallStage,

    /// Stage at which failure occurred (if any)
    failed_at: Option<InstallStage>,

    /// History of completed stages with timestamps (stage order, unix timestamp)
    /// This allows debugging and progress tracking without global state
    stage_history: Vec<(InstallStage, u64)>,

    /// Whether destructive operations have been confirmed
    destructive_confirmed: bool,

    /// Detected firmware mode (UEFI or BIOS) — set once at startup
    firmware_mode: FirmwareMode,

    /// Detected network connectivity — set at startup, can be refreshed
    network_state: NetworkState,
}

impl Default for InstallerContext {
    fn default() -> Self {
        Self::new()
    }
}

impl InstallerContext {
    /// Create a new installer context in the NotStarted state.
    ///
    /// Defaults to BIOS firmware and Offline network (safe defaults).
    /// Use `with_hardware()` to initialize with detected hardware info.
    pub fn new() -> Self {
        Self {
            current: InstallStage::NotStarted,
            failed_at: None,
            stage_history: Vec::with_capacity(InstallStage::all_stages().len()),
            destructive_confirmed: false,
            firmware_mode: FirmwareMode::Bios,
            network_state: NetworkState::Offline,
        }
    }

    /// Create a new installer context initialized with detected hardware info.
    ///
    /// This is the preferred constructor for production use. It runs hardware
    /// detection at creation time and stores the results for the lifetime of
    /// the installation.
    pub fn with_hardware(hw: HardwareInfo) -> Self {
        Self {
            current: InstallStage::NotStarted,
            failed_at: None,
            stage_history: Vec::with_capacity(InstallStage::all_stages().len()),
            destructive_confirmed: false,
            firmware_mode: hw.firmware,
            network_state: hw.network,
        }
    }

    /// Returns the detected firmware mode (UEFI or BIOS).
    #[inline]
    pub fn firmware_mode(&self) -> FirmwareMode {
        self.firmware_mode
    }

    /// Returns the detected network connectivity state.
    #[inline]
    pub fn network_state(&self) -> NetworkState {
        self.network_state
    }

    /// Refresh network connectivity state (e.g., after user plugs in cable).
    pub fn refresh_network(&mut self) {
        self.network_state = crate::hardware::detect_internet();
        log::info!("Network state refreshed: {}", self.network_state);
    }

    /// Returns the current installation stage
    #[inline]
    pub fn current_stage(&self) -> InstallStage {
        self.current
    }

    /// Returns the stage at which failure occurred, if any
    #[inline]
    pub fn failed_at(&self) -> Option<InstallStage> {
        self.failed_at
    }

    /// Returns true if the installation has completed successfully
    #[inline]
    pub fn is_complete(&self) -> bool {
        self.current == InstallStage::Completed
    }

    /// Returns true if the installation has failed
    #[inline]
    pub fn is_failed(&self) -> bool {
        self.current == InstallStage::Failed
    }

    /// Returns true if the installation is in progress (not terminal)
    #[inline]
    pub fn is_in_progress(&self) -> bool {
        !self.current.is_terminal() && self.current != InstallStage::NotStarted
    }

    /// Returns the current progress percentage (0-100)
    #[inline]
    pub fn progress_percent(&self) -> u8 {
        self.current.progress_percent()
    }

    /// Returns the stage history as a slice of (stage, timestamp) pairs
    pub fn stage_history(&self) -> &[(InstallStage, u64)] {
        &self.stage_history
    }

    /// Confirm that destructive operations are authorized.
    ///
    /// This must be called before transitioning to `PartitioningDisk`.
    /// The confirmation is a one-way flag that cannot be revoked.
    pub fn confirm_destructive_operations(&mut self) {
        self.destructive_confirmed = true;
    }

    /// Returns true if destructive operations have been confirmed
    #[inline]
    pub fn is_destructive_confirmed(&self) -> bool {
        self.destructive_confirmed
    }

    /// Advance to the next stage in sequence.
    ///
    /// # Errors
    ///
    /// - `FromTerminalState` if already at Completed or Failed
    /// - `MissingConfirmation` if entering a destructive stage without confirmation
    pub fn advance(&mut self) -> Result<InstallStage, InstallTransitionError> {
        // Cannot advance from terminal state
        if self.current.is_terminal() {
            return Err(InstallTransitionError::FromTerminalState { from: self.current });
        }

        // Get next stage (safe: we checked is_terminal above)
        // SAFETY: next() only returns None for terminal states, which we checked above
        let next_stage = self.current.next().expect(
            "INTERNAL ERROR: non-terminal stage returned None from next() - this is a bug",
        );

        // Check confirmation for destructive stages
        if next_stage.is_destructive() && !self.destructive_confirmed {
            return Err(InstallTransitionError::MissingConfirmation { stage: next_stage });
        }

        // Record transition
        self.record_stage_transition(next_stage);
        self.current = next_stage;

        Ok(next_stage)
    }

    /// Transition to a specific stage (must be the next stage in sequence).
    ///
    /// This is stricter than `advance()` - it validates that you're transitioning
    /// to the expected stage, preventing logic errors.
    ///
    /// # Errors
    ///
    /// - `AlreadyAtStage` if target is the current stage
    /// - `BackwardTransition` if target is before current
    /// - `SkippedStage` if target is not the immediate next stage
    /// - `FromTerminalState` if current is a terminal state
    /// - `MissingConfirmation` if entering a destructive stage without confirmation
    pub fn transition_to(
        &mut self,
        target: InstallStage,
    ) -> Result<InstallStage, InstallTransitionError> {
        // Cannot transition from terminal state
        if self.current.is_terminal() {
            return Err(InstallTransitionError::FromTerminalState { from: self.current });
        }

        // Cannot transition to same state
        if target == self.current {
            return Err(InstallTransitionError::AlreadyAtStage { stage: target });
        }

        // Cannot transition to Failed via this method (use fail() instead)
        if target == InstallStage::Failed {
            return Err(InstallTransitionError::SkippedStage {
                from: self.current,
                to: target,
            });
        }

        // Check for backward transition
        if target.order() < self.current.order() {
            return Err(InstallTransitionError::BackwardTransition {
                from: self.current,
                to: target,
            });
        }

        // Check for skipped stages
        let next_stage = self.current.next();
        if next_stage != Some(target) {
            return Err(InstallTransitionError::SkippedStage {
                from: self.current,
                to: target,
            });
        }

        // Check confirmation for destructive stages
        if target.is_destructive() && !self.destructive_confirmed {
            return Err(InstallTransitionError::MissingConfirmation { stage: target });
        }

        // Valid transition
        self.record_stage_transition(target);
        self.current = target;

        Ok(target)
    }

    /// Mark the installation as failed.
    ///
    /// This can be called from any non-terminal state and records which stage
    /// the failure occurred at.
    ///
    /// # Errors
    ///
    /// - `FromTerminalState` if already at Completed or Failed
    pub fn fail(&mut self) -> Result<(), InstallTransitionError> {
        if self.current.is_terminal() {
            return Err(InstallTransitionError::FromTerminalState { from: self.current });
        }

        self.failed_at = Some(self.current);
        self.record_stage_transition(InstallStage::Failed);
        self.current = InstallStage::Failed;

        Ok(())
    }

    /// Record a stage transition in the history
    fn record_stage_transition(&mut self, stage: InstallStage) {
        // Use monotonic-ish timestamp (seconds since UNIX_EPOCH)
        // This is acceptable for logging purposes
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0); // Fallback to 0 if system time is before epoch (shouldn't happen)

        self.stage_history.push((stage, timestamp));
    }

    /// Reset the context to NotStarted state.
    ///
    /// This clears all history and confirmation flags. Use with caution.
    pub fn reset(&mut self) {
        self.current = InstallStage::NotStarted;
        self.failed_at = None;
        self.stage_history.clear();
        self.destructive_confirmed = false;
    }
}

// Convert InstallTransitionError to the main ArchTuiError type
impl From<InstallTransitionError> for crate::error::ArchTuiError {
    fn from(err: InstallTransitionError) -> Self {
        crate::error::ArchTuiError::InstallTransition(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // InstallStage Tests
    // =========================================================================

    #[test]
    fn test_stage_order_is_sequential() {
        let stages = InstallStage::all_stages();
        for (i, stage) in stages.iter().enumerate() {
            assert_eq!(
                stage.order() as usize,
                i,
                "Stage {:?} should have order {}",
                stage,
                i
            );
        }
    }

    #[test]
    fn test_stage_next_forms_chain() {
        let mut current = InstallStage::NotStarted;
        let mut count = 0;

        while let Some(next) = current.next() {
            current = next;
            count += 1;
            assert!(count < 20, "Infinite loop detected in stage chain");
        }

        assert_eq!(current, InstallStage::Completed);
        assert_eq!(count, 9); // NotStarted -> Completed is 9 transitions
    }

    #[test]
    fn test_stage_previous_forms_reverse_chain() {
        let mut current = InstallStage::Completed;
        let mut count = 0;

        while let Some(prev) = current.previous() {
            current = prev;
            count += 1;
            assert!(count < 20, "Infinite loop detected in stage chain");
        }

        assert_eq!(current, InstallStage::NotStarted);
        assert_eq!(count, 9);
    }

    #[test]
    fn test_terminal_states() {
        assert!(InstallStage::Completed.is_terminal());
        assert!(InstallStage::Failed.is_terminal());

        for stage in InstallStage::all_stages() {
            if *stage != InstallStage::Completed {
                assert!(!stage.is_terminal() || *stage == InstallStage::Failed);
            }
        }
    }

    #[test]
    fn test_destructive_stages() {
        assert!(InstallStage::PartitioningDisk.is_destructive());

        // Only PartitioningDisk should be marked destructive
        for stage in InstallStage::all_stages() {
            if *stage != InstallStage::PartitioningDisk {
                assert!(
                    !stage.is_destructive(),
                    "{:?} should not be destructive",
                    stage
                );
            }
        }
    }

    #[test]
    fn test_progress_percent_increases() {
        let stages = InstallStage::all_stages();
        let mut last_progress = 0u8;

        for stage in stages {
            let progress = stage.progress_percent();
            assert!(
                progress >= last_progress,
                "Progress should not decrease: {:?} has {}% after {}%",
                stage,
                progress,
                last_progress
            );
            last_progress = progress;
        }

        assert_eq!(InstallStage::Completed.progress_percent(), 100);
    }

    #[test]
    fn test_stage_display() {
        assert_eq!(InstallStage::NotStarted.to_string(), "Not started");
        assert_eq!(
            InstallStage::PartitioningDisk.to_string(),
            "Partitioning disk"
        );
        assert_eq!(
            InstallStage::Completed.to_string(),
            "Installation complete"
        );
    }

    // =========================================================================
    // InstallerContext Tests
    // =========================================================================

    #[test]
    fn test_context_starts_at_not_started() {
        let ctx = InstallerContext::new();
        assert_eq!(ctx.current_stage(), InstallStage::NotStarted);
        assert!(!ctx.is_in_progress());
        assert!(!ctx.is_complete());
        assert!(!ctx.is_failed());
    }

    #[test]
    fn test_advance_through_all_stages() {
        let mut ctx = InstallerContext::new();
        ctx.confirm_destructive_operations();

        let mut count = 0;
        while ctx.advance().is_ok() {
            count += 1;
            assert!(count < 20, "Infinite loop detected");
        }

        assert_eq!(ctx.current_stage(), InstallStage::Completed);
        assert!(ctx.is_complete());
        assert_eq!(count, 9);
    }

    #[test]
    fn test_cannot_advance_from_completed() {
        let mut ctx = InstallerContext::new();
        ctx.confirm_destructive_operations();

        // Advance to Completed
        while ctx.current_stage() != InstallStage::Completed {
            ctx.advance().expect("Should advance");
        }

        // Cannot advance further
        let err = ctx.advance().unwrap_err();
        assert!(matches!(
            err,
            InstallTransitionError::FromTerminalState { .. }
        ));
    }

    #[test]
    fn test_cannot_advance_from_failed() {
        let mut ctx = InstallerContext::new();
        ctx.advance().expect("Should advance to ValidatingConfig");
        ctx.fail().expect("Should fail");

        let err = ctx.advance().unwrap_err();
        assert!(matches!(
            err,
            InstallTransitionError::FromTerminalState { .. }
        ));
    }

    #[test]
    fn test_cannot_skip_stages() {
        let mut ctx = InstallerContext::new();

        // Try to skip from NotStarted to PartitioningDisk
        let err = ctx
            .transition_to(InstallStage::PartitioningDisk)
            .unwrap_err();
        assert!(matches!(err, InstallTransitionError::SkippedStage { .. }));

        // Advance normally
        ctx.advance().expect("Should advance");
        assert_eq!(ctx.current_stage(), InstallStage::ValidatingConfig);

        // Try to skip to InstallingBaseSystem
        let err = ctx
            .transition_to(InstallStage::InstallingBaseSystem)
            .unwrap_err();
        assert!(matches!(err, InstallTransitionError::SkippedStage { .. }));
    }

    #[test]
    fn test_cannot_go_backwards() {
        let mut ctx = InstallerContext::new();
        ctx.confirm_destructive_operations();

        // Advance a few stages
        ctx.advance().expect("ValidatingConfig");
        ctx.advance().expect("PreparingSystem");
        ctx.advance().expect("InstallingDependencies");

        // Try to go back
        let err = ctx
            .transition_to(InstallStage::ValidatingConfig)
            .unwrap_err();
        assert!(matches!(
            err,
            InstallTransitionError::BackwardTransition { .. }
        ));
    }

    #[test]
    fn test_cannot_transition_to_same_stage() {
        let mut ctx = InstallerContext::new();
        ctx.advance().expect("ValidatingConfig");

        let err = ctx
            .transition_to(InstallStage::ValidatingConfig)
            .unwrap_err();
        assert!(matches!(err, InstallTransitionError::AlreadyAtStage { .. }));
    }

    #[test]
    fn test_destructive_stage_requires_confirmation() {
        let mut ctx = InstallerContext::new();

        // Advance to just before destructive stage
        ctx.advance().expect("ValidatingConfig");
        ctx.advance().expect("PreparingSystem");
        ctx.advance().expect("InstallingDependencies");

        // Try to advance to PartitioningDisk without confirmation
        let err = ctx.advance().unwrap_err();
        assert!(matches!(
            err,
            InstallTransitionError::MissingConfirmation { .. }
        ));

        // Now confirm and advance
        ctx.confirm_destructive_operations();
        ctx.advance().expect("Should advance to PartitioningDisk");
        assert_eq!(ctx.current_stage(), InstallStage::PartitioningDisk);
    }

    #[test]
    fn test_fail_records_failed_at_stage() {
        let mut ctx = InstallerContext::new();
        ctx.confirm_destructive_operations();

        // Advance to InstallingBaseSystem
        ctx.advance().expect("ValidatingConfig");
        ctx.advance().expect("PreparingSystem");
        ctx.advance().expect("InstallingDependencies");
        ctx.advance().expect("PartitioningDisk");
        ctx.advance().expect("InstallingBaseSystem");

        // Fail at this stage
        ctx.fail().expect("Should fail");

        assert!(ctx.is_failed());
        assert_eq!(ctx.failed_at(), Some(InstallStage::InstallingBaseSystem));
    }

    #[test]
    fn test_cannot_fail_from_terminal_state() {
        let mut ctx = InstallerContext::new();
        ctx.confirm_destructive_operations();

        // Complete the installation
        while ctx.current_stage() != InstallStage::Completed {
            ctx.advance().expect("Should advance");
        }

        // Cannot fail from Completed
        let err = ctx.fail().unwrap_err();
        assert!(matches!(
            err,
            InstallTransitionError::FromTerminalState { .. }
        ));
    }

    #[test]
    fn test_stage_history_is_recorded() {
        let mut ctx = InstallerContext::new();
        ctx.confirm_destructive_operations();

        assert!(ctx.stage_history().is_empty());

        ctx.advance().expect("ValidatingConfig");
        assert_eq!(ctx.stage_history().len(), 1);
        assert_eq!(ctx.stage_history()[0].0, InstallStage::ValidatingConfig);

        ctx.advance().expect("PreparingSystem");
        assert_eq!(ctx.stage_history().len(), 2);
        assert_eq!(ctx.stage_history()[1].0, InstallStage::PreparingSystem);
    }

    #[test]
    fn test_reset_clears_state() {
        let mut ctx = InstallerContext::new();
        ctx.confirm_destructive_operations();

        // Advance and then reset
        ctx.advance().expect("ValidatingConfig");
        ctx.advance().expect("PreparingSystem");
        ctx.reset();

        assert_eq!(ctx.current_stage(), InstallStage::NotStarted);
        assert!(ctx.stage_history().is_empty());
        assert!(!ctx.is_destructive_confirmed());
        assert!(ctx.failed_at().is_none());
    }

    #[test]
    fn test_progress_percent_matches_stage() {
        let mut ctx = InstallerContext::new();
        ctx.confirm_destructive_operations();

        while !ctx.is_complete() {
            let expected = ctx.current_stage().progress_percent();
            assert_eq!(ctx.progress_percent(), expected);
            if ctx.advance().is_err() {
                break;
            }
        }
    }

    #[test]
    fn test_transition_to_validates_exact_next_stage() {
        let mut ctx = InstallerContext::new();

        // Valid: NotStarted -> ValidatingConfig
        ctx.transition_to(InstallStage::ValidatingConfig)
            .expect("Should transition");

        // Invalid: ValidatingConfig -> InstallingDependencies (skips PreparingSystem)
        let err = ctx
            .transition_to(InstallStage::InstallingDependencies)
            .unwrap_err();
        assert!(matches!(err, InstallTransitionError::SkippedStage { .. }));
    }

    // =========================================================================
    // Error Display Tests
    // =========================================================================

    #[test]
    fn test_error_display() {
        let err = InstallTransitionError::SkippedStage {
            from: InstallStage::NotStarted,
            to: InstallStage::PartitioningDisk,
        };
        let msg = err.to_string();
        assert!(msg.contains("Cannot skip"));
        assert!(msg.contains("Not started"));
        assert!(msg.contains("Partitioning disk"));
    }

    #[test]
    fn test_backward_error_display() {
        let err = InstallTransitionError::BackwardTransition {
            from: InstallStage::ConfiguringChroot,
            to: InstallStage::PartitioningDisk,
        };
        let msg = err.to_string();
        assert!(msg.contains("Cannot go backwards"));
    }
}
