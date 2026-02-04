//! Centralized theme and styling for the TUI
//!
//! This module provides a single source of truth for all colors, styles,
//! and visual constants used throughout the application. This makes it easy
//! to maintain visual consistency and enables future theming support.
//!
//! # Usage
//! ```rust
//! use archtui::theme::{Colors, Styles, Theme, LogLevel};
//! use ratatui::style::Style;
//!
//! // Use color constants
//! let style = Style::default().fg(Colors::PRIMARY);
//!
//! // Use pre-built styles
//! let title_style = Styles::title();
//!
//! // Use semantic styles
//! let error_style = Theme::log_style(LogLevel::Error);
//! ```

#![allow(dead_code)]

use ratatui::style::{Color, Modifier, Style};

// =============================================================================
// COLOR PALETTE
// =============================================================================

/// Core color palette for the application
/// All colors should be defined here rather than hardcoded in components
pub struct Colors;

impl Colors {
    // -------------------------------------------------------------------------
    // Base Colors (backgrounds, foregrounds)
    // -------------------------------------------------------------------------

    /// Primary dark background - used for most panels and dialogs
    pub const BG_PRIMARY: Color = Color::Rgb(20, 20, 30);

    /// Alternative dark background - used for contrast areas
    pub const BG_SECONDARY: Color = Color::Rgb(30, 30, 40);

    /// Warning/danger dialog background
    pub const BG_DANGER: Color = Color::Rgb(30, 20, 20);

    /// Gauge/progress bar background
    pub const BG_GAUGE: Color = Color::Rgb(40, 40, 50);

    /// Default foreground text color
    pub const FG_PRIMARY: Color = Color::White;

    /// Secondary/muted text color
    pub const FG_SECONDARY: Color = Color::Gray;

    /// Disabled/inactive text color
    pub const FG_MUTED: Color = Color::DarkGray;

    // -------------------------------------------------------------------------
    // Accent Colors (branding, emphasis)
    // -------------------------------------------------------------------------

    /// Primary accent color - used for borders, titles, highlights
    pub const PRIMARY: Color = Color::Cyan;

    /// Secondary accent color - used for selected items, emphasis
    pub const SECONDARY: Color = Color::Yellow;

    /// Tertiary accent color
    pub const TERTIARY: Color = Color::Blue;

    // -------------------------------------------------------------------------
    // Semantic Colors (status, feedback)
    // -------------------------------------------------------------------------

    /// Success/positive feedback
    pub const SUCCESS: Color = Color::Green;

    /// Light success variant
    pub const SUCCESS_LIGHT: Color = Color::LightGreen;

    /// Warning/caution feedback
    pub const WARNING: Color = Color::Yellow;

    /// Light warning variant
    pub const WARNING_LIGHT: Color = Color::LightYellow;

    /// Error/danger feedback
    pub const ERROR: Color = Color::Red;

    /// Light error variant
    pub const ERROR_LIGHT: Color = Color::LightRed;

    /// Informational feedback
    pub const INFO: Color = Color::Blue;

    /// Light info variant
    pub const INFO_LIGHT: Color = Color::LightBlue;

    // -------------------------------------------------------------------------
    // UI Element Colors
    // -------------------------------------------------------------------------

    /// Active border color
    pub const BORDER_ACTIVE: Color = Color::Cyan;

    /// Inactive/unfocused border color
    pub const BORDER_INACTIVE: Color = Color::DarkGray;

    /// Selected item highlight
    pub const SELECTED_BG: Color = Color::Yellow;

    /// Selected item text (for contrast on yellow bg)
    pub const SELECTED_FG: Color = Color::Black;

    /// Unselected list item
    pub const UNSELECTED: Color = Color::Gray;

    /// Scrollbar/indicator color
    pub const SCROLLBAR: Color = Color::DarkGray;

    /// Header/title text
    pub const HEADER: Color = Color::Cyan;

    /// Progress bar fill
    pub const PROGRESS: Color = Color::Green;

    // -------------------------------------------------------------------------
    // Severity Colors (for dialogs, confirmations)
    // -------------------------------------------------------------------------

    /// Info severity border/accent
    pub const SEVERITY_INFO: Color = Color::Cyan;

    /// Warning severity border/accent
    pub const SEVERITY_WARNING: Color = Color::Yellow;

    /// Danger severity border/accent
    pub const SEVERITY_DANGER: Color = Color::Red;

    // -------------------------------------------------------------------------
    // Menu/Navigation Colors
    // -------------------------------------------------------------------------

    /// Category header color
    pub const CATEGORY: Color = Color::Yellow;

    /// Subcategory/tool name color
    pub const TOOL_NAME: Color = Color::Cyan;

    /// Tool description color
    pub const TOOL_DESC: Color = Color::Gray;

    /// Navigation hint color
    pub const NAV_HINT: Color = Color::DarkGray;

    // -------------------------------------------------------------------------
    // Installation Progress Colors
    // -------------------------------------------------------------------------

    /// Phase/section header
    pub const PHASE: Color = Color::Cyan;

    /// Active/running step
    pub const STEP_ACTIVE: Color = Color::Yellow;

    /// Completed step
    pub const STEP_COMPLETE: Color = Color::Green;

    /// Pending step
    pub const STEP_PENDING: Color = Color::Gray;

    /// Failed step
    pub const STEP_FAILED: Color = Color::Red;
}

// =============================================================================
// PRE-BUILT STYLES
// =============================================================================

/// Pre-built styles for common UI patterns
/// Use these instead of constructing styles inline for consistency
pub struct Styles;

impl Styles {
    // -------------------------------------------------------------------------
    // Text Styles
    // -------------------------------------------------------------------------

    /// Default text style
    pub fn text() -> Style {
        Style::default().fg(Colors::FG_PRIMARY)
    }

    /// Muted/secondary text
    pub fn text_muted() -> Style {
        Style::default().fg(Colors::FG_MUTED)
    }

    /// Secondary text (gray)
    pub fn text_secondary() -> Style {
        Style::default().fg(Colors::FG_SECONDARY)
    }

    /// Bold text
    pub fn text_bold() -> Style {
        Style::default()
            .fg(Colors::FG_PRIMARY)
            .add_modifier(Modifier::BOLD)
    }

    // -------------------------------------------------------------------------
    // Title/Header Styles
    // -------------------------------------------------------------------------

    /// Main title style (cyan, bold)
    pub fn title() -> Style {
        Style::default()
            .fg(Colors::PRIMARY)
            .add_modifier(Modifier::BOLD)
    }

    /// Section header style
    pub fn header() -> Style {
        Style::default()
            .fg(Colors::HEADER)
            .add_modifier(Modifier::BOLD)
    }

    /// Category header (yellow)
    pub fn category() -> Style {
        Style::default()
            .fg(Colors::CATEGORY)
            .add_modifier(Modifier::BOLD)
    }

    // -------------------------------------------------------------------------
    // Border/Block Styles
    // -------------------------------------------------------------------------

    /// Active border style
    pub fn border_active() -> Style {
        Style::default().fg(Colors::BORDER_ACTIVE)
    }

    /// Inactive border style
    pub fn border_inactive() -> Style {
        Style::default().fg(Colors::BORDER_INACTIVE)
    }

    /// Panel background
    pub fn panel_bg() -> Style {
        Style::default().bg(Colors::BG_PRIMARY)
    }

    /// Alternative panel background
    pub fn panel_bg_alt() -> Style {
        Style::default().bg(Colors::BG_SECONDARY)
    }

    /// Danger panel background
    pub fn panel_bg_danger() -> Style {
        Style::default().bg(Colors::BG_DANGER)
    }

    // -------------------------------------------------------------------------
    // Selection Styles
    // -------------------------------------------------------------------------

    /// Selected/highlighted item
    pub fn selected() -> Style {
        Style::default()
            .fg(Colors::SELECTED_FG)
            .bg(Colors::SELECTED_BG)
            .add_modifier(Modifier::BOLD)
    }

    /// Unselected list item
    pub fn unselected() -> Style {
        Style::default().fg(Colors::UNSELECTED)
    }

    /// Focused item (cyan highlight)
    pub fn focused() -> Style {
        Style::default()
            .fg(Colors::PRIMARY)
            .add_modifier(Modifier::BOLD)
    }

    // -------------------------------------------------------------------------
    // Status/Feedback Styles
    // -------------------------------------------------------------------------

    /// Success message style
    pub fn success() -> Style {
        Style::default().fg(Colors::SUCCESS)
    }

    /// Warning message style
    pub fn warning() -> Style {
        Style::default().fg(Colors::WARNING)
    }

    /// Error message style
    pub fn error() -> Style {
        Style::default().fg(Colors::ERROR)
    }

    /// Info message style
    pub fn info() -> Style {
        Style::default().fg(Colors::INFO)
    }

    // -------------------------------------------------------------------------
    // Button Styles
    // -------------------------------------------------------------------------

    /// Active/selected button
    pub fn button_active() -> Style {
        Style::default()
            .fg(Colors::SELECTED_FG)
            .bg(Colors::FG_PRIMARY)
            .add_modifier(Modifier::BOLD)
    }

    /// Inactive button
    pub fn button_inactive() -> Style {
        Style::default().fg(Colors::FG_PRIMARY)
    }

    /// Confirm/Yes button (selected)
    pub fn button_confirm() -> Style {
        Style::default()
            .fg(Colors::SELECTED_FG)
            .bg(Colors::SUCCESS)
            .add_modifier(Modifier::BOLD)
    }

    /// Cancel/No button (selected)
    pub fn button_cancel() -> Style {
        Style::default()
            .fg(Colors::SELECTED_FG)
            .bg(Colors::FG_PRIMARY)
            .add_modifier(Modifier::BOLD)
    }

    /// Danger button (selected)
    pub fn button_danger() -> Style {
        Style::default()
            .fg(Colors::FG_PRIMARY)
            .bg(Colors::ERROR)
            .add_modifier(Modifier::BOLD)
    }

    // -------------------------------------------------------------------------
    // Progress/Gauge Styles
    // -------------------------------------------------------------------------

    /// Progress bar style
    pub fn progress() -> Style {
        Style::default()
            .fg(Colors::PROGRESS)
            .bg(Colors::BG_GAUGE)
    }

    /// Progress percentage text
    pub fn progress_text() -> Style {
        Style::default()
            .fg(Colors::PRIMARY)
            .add_modifier(Modifier::BOLD)
    }

    // -------------------------------------------------------------------------
    // Menu Item Styles
    // -------------------------------------------------------------------------

    /// Tool/menu item name
    pub fn menu_item() -> Style {
        Style::default().fg(Colors::TOOL_NAME)
    }

    /// Tool description
    pub fn menu_desc() -> Style {
        Style::default().fg(Colors::TOOL_DESC)
    }

    /// Navigation hint (keybindings)
    pub fn nav_hint() -> Style {
        Style::default().fg(Colors::NAV_HINT)
    }
}

// =============================================================================
// THEME CONTEXT
// =============================================================================

/// Log level for styling log output
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Debug,
    Info,
    Warning,
    Error,
    Success,
    Phase,
    Command,
}

/// Severity level for dialogs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Info,
    Warning,
    Danger,
}

/// Theme context providing semantic style lookups
pub struct Theme;

impl Theme {
    /// Get style for a log level
    pub fn log_style(level: LogLevel) -> Style {
        match level {
            LogLevel::Debug => Style::default().fg(Colors::FG_MUTED),
            LogLevel::Info => Style::default().fg(Colors::FG_PRIMARY),
            LogLevel::Warning => Style::default().fg(Colors::WARNING),
            LogLevel::Error => Style::default().fg(Colors::ERROR),
            LogLevel::Success => Style::default().fg(Colors::SUCCESS),
            LogLevel::Phase => Style::default()
                .fg(Colors::PHASE)
                .add_modifier(Modifier::BOLD),
            LogLevel::Command => Style::default().fg(Colors::FG_MUTED),
        }
    }

    /// Get border color for a severity level
    pub fn severity_color(severity: Severity) -> Color {
        match severity {
            Severity::Info => Colors::SEVERITY_INFO,
            Severity::Warning => Colors::SEVERITY_WARNING,
            Severity::Danger => Colors::SEVERITY_DANGER,
        }
    }

    /// Get style for a severity level (text)
    pub fn severity_style(severity: Severity) -> Style {
        match severity {
            Severity::Info => Style::default().fg(Colors::FG_PRIMARY),
            Severity::Warning => Style::default().fg(Colors::WARNING),
            Severity::Danger => Style::default()
                .fg(Colors::ERROR)
                .add_modifier(Modifier::BOLD),
        }
    }

    /// Get button style for severity when selected
    pub fn severity_button_active(severity: Severity) -> Style {
        match severity {
            Severity::Info => Style::default()
                .fg(Colors::SELECTED_FG)
                .bg(Colors::SEVERITY_INFO)
                .add_modifier(Modifier::BOLD),
            Severity::Warning => Style::default()
                .fg(Colors::SELECTED_FG)
                .bg(Colors::SEVERITY_WARNING)
                .add_modifier(Modifier::BOLD),
            Severity::Danger => Style::default()
                .fg(Colors::FG_PRIMARY)
                .bg(Colors::SEVERITY_DANGER)
                .add_modifier(Modifier::BOLD),
        }
    }

    /// Get button style for severity when not selected
    pub fn severity_button_inactive(severity: Severity) -> Style {
        match severity {
            Severity::Info => Style::default().fg(Colors::SEVERITY_INFO),
            Severity::Warning => Style::default().fg(Colors::SEVERITY_WARNING),
            Severity::Danger => Style::default().fg(Colors::SEVERITY_DANGER),
        }
    }

    /// Get style for installation step status
    pub fn step_style(completed: bool, active: bool, failed: bool) -> Style {
        if failed {
            Style::default().fg(Colors::STEP_FAILED)
        } else if completed {
            Style::default().fg(Colors::STEP_COMPLETE)
        } else if active {
            Style::default()
                .fg(Colors::STEP_ACTIVE)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Colors::STEP_PENDING)
        }
    }

    /// Get icon for severity level
    pub fn severity_icon(severity: Severity) -> &'static str {
        match severity {
            Severity::Info => "ℹ️ ",
            Severity::Warning => "⚠️ ",
            Severity::Danger => "🚨",
        }
    }
}

// =============================================================================
// UI CONSTANTS
// =============================================================================

/// UI dimension and layout constants
pub struct UiConstants;

impl UiConstants {
    /// Default dialog width percentage
    pub const DIALOG_WIDTH_PCT: u16 = 60;

    /// Default dialog max width
    pub const DIALOG_MAX_WIDTH: u16 = 80;

    /// Minimum dialog width
    pub const DIALOG_MIN_WIDTH: u16 = 40;

    /// Dialog padding from edges
    pub const DIALOG_PADDING: u16 = 4;

    /// Default panel padding
    pub const PANEL_PADDING: u16 = 1;

    /// Nav bar height
    pub const NAV_BAR_HEIGHT: u16 = 1;

    /// Header height (with ASCII art)
    pub const HEADER_HEIGHT: u16 = 10;

    /// Status bar height
    pub const STATUS_BAR_HEIGHT: u16 = 3;

    /// Scroll page size (items)
    pub const PAGE_SCROLL_SIZE: usize = 10;
}

// =============================================================================
// TEXT CONSTANTS
// =============================================================================

/// Common UI text strings
pub struct UiText;

impl UiText {
    // Button labels
    pub const BTN_YES_CONTINUE: &'static str = "[ Yes / Continue ]";
    pub const BTN_YES_PROCEED: &'static str = "[ Yes / Proceed ]";
    pub const BTN_CONFIRM_DELETE: &'static str = "[ CONFIRM DELETE ]";
    pub const BTN_NO_CANCEL: &'static str = "[ No / Cancel ]";

    // Common prompts
    pub const PRESS_ENTER: &'static str = "Press Enter to continue";
    pub const PRESS_ESC: &'static str = "Press Esc to cancel";

    // Status messages
    pub const LOADING: &'static str = "Loading...";
    pub const PROCESSING: &'static str = "Processing...";
    pub const COMPLETE: &'static str = "Complete!";
    pub const FAILED: &'static str = "Failed";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_constants() {
        // Ensure colors can be used
        let _ = Colors::PRIMARY;
        let _ = Colors::BG_PRIMARY;
    }

    #[test]
    fn test_styles() {
        // Ensure styles can be created
        let _ = Styles::title();
        let _ = Styles::selected();
        let _ = Styles::error();
    }

    #[test]
    fn test_theme_lookups() {
        let _ = Theme::log_style(LogLevel::Error);
        let _ = Theme::severity_color(Severity::Warning);
    }
}
