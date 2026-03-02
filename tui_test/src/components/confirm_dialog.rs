//! Confirmation dialog component (redesigned)
//!
//! Rounded borders, severity-colored chrome, filled/outlined buttons.

#![allow(dead_code)]

use crate::theme::{Colors, Severity, Styles, Theme, UiText};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap},
    Frame,
};

/// Severity level for the confirmation dialog
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmSeverity {
    /// Informational - no data loss risk
    Info,
    /// Warning - potential data loss
    Warning,
    /// Danger - guaranteed data destruction
    Danger,
}

/// State for a confirmation dialog
#[derive(Debug, Clone)]
pub struct ConfirmDialogState {
    /// Title of the dialog
    pub title: String,
    /// Main message to display
    pub message: String,
    /// Additional details or warnings
    pub details: Vec<String>,
    /// Severity level
    pub severity: ConfirmSeverity,
    /// Currently selected option (0 = No/Cancel on left, 1 = Yes/Confirm on right)
    pub selected: usize,
    /// Callback identifier for what to do on confirm
    pub confirm_action: String,
    /// Optional additional data for the action
    pub action_data: Option<String>,
}

impl ConfirmDialogState {
    /// Create a new confirmation dialog
    pub fn new(title: &str, message: &str, severity: ConfirmSeverity, confirm_action: &str) -> Self {
        Self {
            title: title.to_string(),
            message: message.to_string(),
            details: Vec::new(),
            severity,
            selected: 0, // Default to "No" (left button) for safety
            confirm_action: confirm_action.to_string(),
            action_data: None,
        }
    }

    /// Add a detail line
    pub fn with_detail(mut self, detail: &str) -> Self {
        self.details.push(detail.to_string());
        self
    }

    /// Add action data
    pub fn with_action_data(mut self, data: &str) -> Self {
        self.action_data = Some(data.to_string());
        self
    }

    /// Toggle selection between Yes and No
    pub fn toggle_selection(&mut self) {
        self.selected = if self.selected == 0 { 1 } else { 0 };
    }

    /// Select No/Cancel
    pub fn select_no(&mut self) {
        self.selected = 0;
    }

    /// Select Yes/Confirm
    pub fn select_yes(&mut self) {
        self.selected = 1;
    }

    /// Check if Yes is selected
    pub fn is_confirmed(&self) -> bool {
        self.selected == 1
    }
}

/// Confirmation dialog for destructive operations
pub struct ConfirmDialog;

impl ConfirmDialog {
    pub fn render(f: &mut Frame, state: &ConfirmDialogState) {
        let area = f.area();

        let dialog_width = 64u16.min(area.width.saturating_sub(4));
        let detail_rows = if state.details.is_empty() {
            0
        } else {
            state.details.len() as u16 + 1
        };
        let dialog_height = (10 + detail_rows).min(area.height.saturating_sub(4));
        let dialog_x = (area.width.saturating_sub(dialog_width)) / 2;
        let dialog_y = (area.height.saturating_sub(dialog_height)) / 2;
        let dialog_area = Rect::new(dialog_x, dialog_y, dialog_width, dialog_height);

        f.render_widget(Clear, dialog_area);

        let severity = match state.severity {
            ConfirmSeverity::Info => Severity::Info,
            ConfirmSeverity::Warning => Severity::Warning,
            ConfirmSeverity::Danger => Severity::Danger,
        };
        let border_color = Theme::severity_color(severity);
        let icon = Theme::severity_icon(severity);

        // Background tint based on severity
        let bg_style = match severity {
            Severity::Danger => Style::default().bg(Colors::BG_DANGER),
            _ => Style::default().bg(Colors::BG_PRIMARY),
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title(Line::from(vec![
                Span::styled("\u{2500}", Style::default().fg(border_color)),
                Span::styled(
                    format!(" {} {} ", icon, state.title),
                    Style::default()
                        .fg(border_color)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("\u{2500}", Style::default().fg(border_color)),
            ]))
            .border_style(Style::default().fg(border_color))
            .style(bg_style);

        let inner = block.inner(dialog_area);
        f.render_widget(block, dialog_area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(2), // Message
                Constraint::Min(1),   // Details
                Constraint::Length(3), // Buttons
            ])
            .split(inner);

        // Message
        let message_style = Theme::severity_style(severity);
        let message = Paragraph::new(state.message.clone())
            .style(message_style)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });
        f.render_widget(message, chunks[0]);

        // Details
        if !state.details.is_empty() {
            let detail_lines: Vec<Line> = state
                .details
                .iter()
                .map(|d| {
                    Line::from(vec![
                        Span::styled(
                            "  \u{2022} ",
                            Style::default().fg(border_color),
                        ),
                        Span::styled(d.clone(), Styles::text_secondary()),
                    ])
                })
                .collect();
            let details = Paragraph::new(detail_lines).wrap(Wrap { trim: true });
            f.render_widget(details, chunks[1]);
        }

        // Buttons with filled/outlined styling
        let button_area = chunks[2];
        let button_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(10),
                Constraint::Percentage(35),
                Constraint::Percentage(10),
                Constraint::Percentage(35),
                Constraint::Percentage(10),
            ])
            .split(button_area);

        // No/Cancel button (index 1)
        let no_selected = state.selected == 0;
        let no_style = if no_selected {
            Style::default()
                .fg(Colors::BG_PRIMARY)
                .bg(Colors::FG_PRIMARY)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Colors::FG_SECONDARY)
        };
        let no_border_style = if no_selected {
            Style::default().fg(Colors::FG_PRIMARY)
        } else {
            Style::default().fg(Colors::BORDER_INACTIVE)
        };
        let no_button = Paragraph::new(UiText::BTN_NO_CANCEL)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(no_border_style),
            )
            .style(no_style)
            .alignment(Alignment::Center);
        f.render_widget(no_button, button_chunks[1]);

        // Yes/Confirm button (index 3)
        let yes_selected = state.selected == 1;
        let yes_style = if yes_selected {
            Theme::severity_button_active(severity)
        } else {
            Theme::severity_button_inactive(severity)
        };
        let yes_border_style = if yes_selected {
            Style::default().fg(border_color)
        } else {
            Style::default().fg(Colors::BORDER_INACTIVE)
        };
        let yes_text = match state.severity {
            ConfirmSeverity::Info => UiText::BTN_YES_CONTINUE,
            ConfirmSeverity::Warning => UiText::BTN_YES_PROCEED,
            ConfirmSeverity::Danger => UiText::BTN_CONFIRM_DELETE,
        };
        let yes_button = Paragraph::new(yes_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(yes_border_style),
            )
            .style(yes_style)
            .alignment(Alignment::Center);
        f.render_widget(yes_button, button_chunks[3]);
    }
}

/// Create a confirmation dialog for formatting a partition
pub fn format_partition_confirm(partition: &str, filesystem: &str) -> ConfirmDialogState {
    ConfirmDialogState::new(
        "FORMAT PARTITION",
        &format!("Format {} as {}?", partition, filesystem),
        ConfirmSeverity::Danger,
        "format_partition",
    )
    .with_detail(&format!("All data on {} will be permanently erased", partition))
    .with_detail("This operation CANNOT be undone")
}

/// Create a confirmation dialog for wiping a disk
pub fn wipe_disk_confirm(disk: &str) -> ConfirmDialogState {
    ConfirmDialogState::new(
        "WIPE ENTIRE DISK",
        &format!("Permanently erase ALL data on {}?", disk),
        ConfirmSeverity::Danger,
        "wipe_disk",
    )
    .with_detail("ALL partitions will be destroyed")
    .with_detail("ALL data will be permanently erased")
    .with_detail("This operation CANNOT be undone")
}

/// Create a confirmation dialog for installing a bootloader
pub fn bootloader_confirm(bootloader: &str, disk: &str) -> ConfirmDialogState {
    ConfirmDialogState::new(
        "INSTALL BOOTLOADER",
        &format!("Install {} bootloader to {}?", bootloader, disk),
        ConfirmSeverity::Warning,
        "install_bootloader",
    )
    .with_detail("This will modify the boot sector")
    .with_detail("Existing bootloaders may be overwritten")
}

/// Create a confirmation dialog for starting installation
pub fn start_install_confirm() -> ConfirmDialogState {
    ConfirmDialogState::new(
        "START INSTALLATION",
        "Begin Arch Linux installation?",
        ConfirmSeverity::Warning,
        "start_install",
    )
    .with_detail("This will modify the target disk")
    .with_detail("Ensure your configuration is correct")
}
