//! Redesigned theme with muted, modern palette
//!
//! Inspired by OpenAPI-TUI: warm tones, muted borders, soft contrast.
//! Replaces harsh bright cyan with teal/amber palette.

#![allow(dead_code)]

use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::BorderType;

// =============================================================================
// COLOR PALETTE — Muted Warm Tones
// =============================================================================

pub struct Colors;

impl Colors {
    // ── Base Colors ──────────────────────────────────────────────────────
    pub const BG_PRIMARY: Color = Color::Rgb(22, 22, 30);
    pub const BG_SECONDARY: Color = Color::Rgb(28, 30, 38);
    pub const BG_DANGER: Color = Color::Rgb(35, 22, 22);
    pub const BG_GAUGE: Color = Color::Rgb(35, 38, 48);

    pub const FG_PRIMARY: Color = Color::Rgb(200, 200, 210);
    pub const FG_SECONDARY: Color = Color::Rgb(130, 135, 145);
    pub const FG_MUTED: Color = Color::Rgb(75, 78, 88);

    // ── Accent Colors ────────────────────────────────────────────────────
    /// Primary accent — muted teal (replaces bright Cyan)
    pub const PRIMARY: Color = Color::Rgb(100, 160, 170);
    /// Secondary accent — warm amber (replaces bright Yellow)
    pub const SECONDARY: Color = Color::Rgb(210, 170, 90);
    /// Tertiary accent — soft blue
    pub const TERTIARY: Color = Color::Rgb(90, 130, 190);

    // ── Semantic Colors (kept vivid for visibility) ──────────────────────
    pub const SUCCESS: Color = Color::Rgb(80, 190, 120);
    pub const SUCCESS_LIGHT: Color = Color::Rgb(120, 220, 150);
    pub const WARNING: Color = Color::Rgb(220, 180, 60);
    pub const WARNING_LIGHT: Color = Color::Rgb(240, 210, 100);
    pub const ERROR: Color = Color::Rgb(220, 80, 80);
    pub const ERROR_LIGHT: Color = Color::Rgb(240, 120, 120);
    pub const INFO: Color = Color::Rgb(90, 140, 210);
    pub const INFO_LIGHT: Color = Color::Rgb(120, 170, 230);

    // ── UI Element Colors ────────────────────────────────────────────────
    pub const BORDER_ACTIVE: Color = Color::Rgb(80, 130, 140);
    pub const BORDER_INACTIVE: Color = Color::Rgb(55, 58, 65);

    pub const SELECTED_BG: Color = Color::Rgb(210, 170, 90);
    pub const SELECTED_FG: Color = Color::Rgb(22, 22, 30);

    pub const UNSELECTED: Color = Color::Rgb(130, 135, 145);
    pub const SCROLLBAR: Color = Color::Rgb(60, 65, 75);
    pub const SCROLLBAR_TRACK: Color = Color::Rgb(35, 38, 45);
    pub const SCROLLBAR_THUMB: Color = Color::Rgb(80, 85, 95);
    pub const HEADER: Color = Color::Rgb(100, 160, 170);
    pub const PROGRESS: Color = Color::Rgb(80, 190, 120);

    // ── Severity Colors ──────────────────────────────────────────────────
    pub const SEVERITY_INFO: Color = Color::Rgb(100, 160, 170);
    pub const SEVERITY_WARNING: Color = Color::Rgb(220, 180, 60);
    pub const SEVERITY_DANGER: Color = Color::Rgb(220, 80, 80);

    // ── Menu/Navigation ──────────────────────────────────────────────────
    pub const CATEGORY: Color = Color::Rgb(210, 170, 90);
    pub const TOOL_NAME: Color = Color::Rgb(100, 160, 170);
    pub const TOOL_DESC: Color = Color::Rgb(130, 135, 145);
    pub const NAV_HINT: Color = Color::Rgb(75, 78, 88);

    // ── Tool Category Accents ───────────────────────────────────────────
    pub const CAT_DISK: Color = Color::Rgb(90, 130, 190);    // soft blue
    pub const CAT_SYSTEM: Color = Color::Rgb(100, 160, 170); // teal
    pub const CAT_USER: Color = Color::Rgb(210, 170, 90);    // amber
    pub const CAT_NETWORK: Color = Color::Rgb(80, 190, 120); // green

    // ── Installation Progress ────────────────────────────────────────────
    pub const PHASE: Color = Color::Rgb(100, 160, 170);
    pub const STEP_ACTIVE: Color = Color::Rgb(210, 170, 90);
    pub const STEP_COMPLETE: Color = Color::Rgb(80, 190, 120);
    pub const STEP_PENDING: Color = Color::Rgb(130, 135, 145);
    pub const STEP_FAILED: Color = Color::Rgb(220, 80, 80);

    // ── Nav bar specific ─────────────────────────────────────────────────
    pub const NAV_KEY: Color = Color::Rgb(140, 190, 200);
    pub const NAV_ARROW: Color = Color::Rgb(75, 78, 88);
    pub const NAV_ACTION: Color = Color::Rgb(150, 150, 160);
    pub const NAV_BRACKET: Color = Color::Rgb(55, 58, 65);
}

// =============================================================================
// PRE-BUILT STYLES
// =============================================================================

pub struct Styles;

impl Styles {
    // ── Text ─────────────────────────────────────────────────────────────
    pub fn text() -> Style {
        Style::default().fg(Colors::FG_PRIMARY)
    }
    pub fn text_muted() -> Style {
        Style::default().fg(Colors::FG_MUTED)
    }
    pub fn text_secondary() -> Style {
        Style::default().fg(Colors::FG_SECONDARY)
    }
    pub fn text_bold() -> Style {
        Style::default()
            .fg(Colors::FG_PRIMARY)
            .add_modifier(Modifier::BOLD)
    }

    // ── Title/Header ─────────────────────────────────────────────────────
    pub fn title() -> Style {
        Style::default()
            .fg(Colors::PRIMARY)
            .add_modifier(Modifier::BOLD)
    }
    pub fn header() -> Style {
        Style::default()
            .fg(Colors::HEADER)
            .add_modifier(Modifier::BOLD)
    }
    pub fn category() -> Style {
        Style::default()
            .fg(Colors::CATEGORY)
            .add_modifier(Modifier::BOLD)
    }

    // ── Borders ──────────────────────────────────────────────────────────
    pub fn border_active() -> Style {
        Style::default().fg(Colors::BORDER_ACTIVE)
    }
    pub fn border_inactive() -> Style {
        Style::default().fg(Colors::BORDER_INACTIVE)
    }
    pub fn panel_bg() -> Style {
        Style::default().bg(Colors::BG_PRIMARY)
    }
    pub fn panel_bg_alt() -> Style {
        Style::default().bg(Colors::BG_SECONDARY)
    }
    pub fn panel_bg_danger() -> Style {
        Style::default().bg(Colors::BG_DANGER)
    }

    // ── Selection ────────────────────────────────────────────────────────
    pub fn selected() -> Style {
        Style::default()
            .fg(Colors::SELECTED_FG)
            .bg(Colors::SELECTED_BG)
            .add_modifier(Modifier::BOLD)
    }
    pub fn unselected() -> Style {
        Style::default().fg(Colors::UNSELECTED)
    }
    pub fn focused() -> Style {
        Style::default()
            .fg(Colors::PRIMARY)
            .add_modifier(Modifier::BOLD)
    }

    // ── Status/Feedback ──────────────────────────────────────────────────
    pub fn success() -> Style {
        Style::default().fg(Colors::SUCCESS)
    }
    pub fn warning() -> Style {
        Style::default().fg(Colors::WARNING)
    }
    pub fn error() -> Style {
        Style::default().fg(Colors::ERROR)
    }
    pub fn info() -> Style {
        Style::default().fg(Colors::INFO)
    }

    // ── Buttons ──────────────────────────────────────────────────────────
    pub fn button_active() -> Style {
        Style::default()
            .fg(Colors::SELECTED_FG)
            .bg(Colors::FG_PRIMARY)
            .add_modifier(Modifier::BOLD)
    }
    pub fn button_inactive() -> Style {
        Style::default().fg(Colors::FG_PRIMARY)
    }
    pub fn button_confirm() -> Style {
        Style::default()
            .fg(Colors::SELECTED_FG)
            .bg(Colors::SUCCESS)
            .add_modifier(Modifier::BOLD)
    }
    pub fn button_cancel() -> Style {
        Style::default()
            .fg(Colors::SELECTED_FG)
            .bg(Colors::FG_PRIMARY)
            .add_modifier(Modifier::BOLD)
    }
    pub fn button_danger() -> Style {
        Style::default()
            .fg(Colors::FG_PRIMARY)
            .bg(Colors::ERROR)
            .add_modifier(Modifier::BOLD)
    }

    // ── Progress ─────────────────────────────────────────────────────────
    pub fn progress() -> Style {
        Style::default()
            .fg(Colors::PROGRESS)
            .bg(Colors::BG_GAUGE)
    }
    pub fn progress_text() -> Style {
        Style::default()
            .fg(Colors::PRIMARY)
            .add_modifier(Modifier::BOLD)
    }

    // ── Menu ─────────────────────────────────────────────────────────────
    pub fn menu_item() -> Style {
        Style::default().fg(Colors::TOOL_NAME)
    }
    pub fn menu_desc() -> Style {
        Style::default().fg(Colors::TOOL_DESC)
    }
    pub fn nav_hint() -> Style {
        Style::default().fg(Colors::NAV_HINT)
    }
}

// =============================================================================
// THEME CONTEXT
// =============================================================================

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Info,
    Warning,
    Danger,
}

pub struct Theme;

impl Theme {
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

    pub fn severity_color(severity: Severity) -> Color {
        match severity {
            Severity::Info => Colors::SEVERITY_INFO,
            Severity::Warning => Colors::SEVERITY_WARNING,
            Severity::Danger => Colors::SEVERITY_DANGER,
        }
    }

    pub fn severity_style(severity: Severity) -> Style {
        match severity {
            Severity::Info => Style::default().fg(Colors::FG_PRIMARY),
            Severity::Warning => Style::default().fg(Colors::WARNING),
            Severity::Danger => Style::default()
                .fg(Colors::ERROR)
                .add_modifier(Modifier::BOLD),
        }
    }

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

    pub fn severity_button_inactive(severity: Severity) -> Style {
        match severity {
            Severity::Info => Style::default().fg(Colors::SEVERITY_INFO),
            Severity::Warning => Style::default().fg(Colors::SEVERITY_WARNING),
            Severity::Danger => Style::default().fg(Colors::SEVERITY_DANGER),
        }
    }

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

    pub fn severity_icon(severity: Severity) -> &'static str {
        match severity {
            Severity::Info => "i",
            Severity::Warning => "!",
            Severity::Danger => "X",
        }
    }
}

// =============================================================================
// BORDER HELPERS (new for redesign)
// =============================================================================

pub struct Borders;

impl Borders {
    pub const DEFAULT_TYPE: BorderType = BorderType::Plain;
}

// =============================================================================
// UI CONSTANTS
// =============================================================================

pub struct UiConstants;

impl UiConstants {
    pub const DIALOG_WIDTH_PCT: u16 = 60;
    pub const DIALOG_MAX_WIDTH: u16 = 80;
    pub const DIALOG_MIN_WIDTH: u16 = 40;
    pub const DIALOG_PADDING: u16 = 4;
    pub const PANEL_PADDING: u16 = 1;
    pub const NAV_BAR_HEIGHT: u16 = 1;
    /// Compact header: 3 lines content + 1 padding = 4
    pub const HEADER_HEIGHT: u16 = 4;
    pub const STATUS_BAR_HEIGHT: u16 = 3;
    pub const PAGE_SCROLL_SIZE: usize = 10;
}

// =============================================================================
// TEXT CONSTANTS
// =============================================================================

pub struct UiText;

impl UiText {
    pub const BTN_YES_CONTINUE: &'static str = "[ Yes / Continue ]";
    pub const BTN_YES_PROCEED: &'static str = "[ Yes / Proceed ]";
    pub const BTN_CONFIRM_DELETE: &'static str = "[ CONFIRM DELETE ]";
    pub const BTN_NO_CANCEL: &'static str = "[ No / Cancel ]";
    pub const PRESS_ENTER: &'static str = "Press Enter to continue";
    pub const PRESS_ESC: &'static str = "Press Esc to cancel";
    pub const LOADING: &'static str = "Loading...";
    pub const PROCESSING: &'static str = "Processing...";
    pub const COMPLETE: &'static str = "Complete!";
    pub const FAILED: &'static str = "Failed";
}
