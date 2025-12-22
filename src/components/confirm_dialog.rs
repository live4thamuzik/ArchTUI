//! Confirmation dialog component
//!
//! Shows a warning dialog before destructive operations.

#![allow(dead_code)]

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
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
    /// Currently selected option (0 = Yes/Confirm on left, 1 = No/Cancel on right)
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
            selected: 1, // Default to "No" (right button) for safety
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
        self.selected = 1;  // No is now on right (button_chunks[1])
    }

    /// Select Yes/Confirm
    pub fn select_yes(&mut self) {
        self.selected = 0;  // Yes is now on left (button_chunks[0])
    }

    /// Check if Yes is selected
    pub fn is_confirmed(&self) -> bool {
        self.selected == 0  // Yes is now on left (selected == 0)
    }
}

/// Confirmation dialog for destructive operations
pub struct ConfirmDialog;

impl ConfirmDialog {
    /// Render the confirmation dialog
    pub fn render(f: &mut Frame, state: &ConfirmDialogState) {
        let area = f.area();

        // Calculate dialog size
        let dialog_width = 60u16.min(area.width.saturating_sub(4));
        let dialog_height = (12 + state.details.len() as u16).min(area.height.saturating_sub(4));

        let dialog_x = (area.width.saturating_sub(dialog_width)) / 2;
        let dialog_y = (area.height.saturating_sub(dialog_height)) / 2;

        let dialog_area = Rect::new(dialog_x, dialog_y, dialog_width, dialog_height);

        // Clear the area
        f.render_widget(Clear, dialog_area);

        // Get colors based on severity
        let (border_color, icon) = match state.severity {
            ConfirmSeverity::Info => (Color::Cyan, "ℹ️ "),
            ConfirmSeverity::Warning => (Color::Yellow, "⚠️ "),
            ConfirmSeverity::Danger => (Color::Red, "🚨"),
        };

        // Create the dialog block
        let block = Block::default()
            .borders(Borders::ALL)
            .title(format!(" {} {} ", icon, state.title))
            .title_style(Style::default().fg(border_color).add_modifier(Modifier::BOLD))
            .border_style(Style::default().fg(border_color))
            .style(Style::default().bg(Color::Rgb(30, 20, 20)));

        let inner = block.inner(dialog_area);
        f.render_widget(block, dialog_area);

        // Layout: message, details, buttons
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(3),  // Message
                Constraint::Min(1),     // Details
                Constraint::Length(3),  // Buttons
            ])
            .split(inner);

        // Render message
        let message_style = match state.severity {
            ConfirmSeverity::Info => Style::default().fg(Color::White),
            ConfirmSeverity::Warning => Style::default().fg(Color::Yellow),
            ConfirmSeverity::Danger => Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        };

        let message = Paragraph::new(state.message.clone())
            .style(message_style)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });
        f.render_widget(message, chunks[0]);

        // Render details
        if !state.details.is_empty() {
            let detail_lines: Vec<Line> = state.details
                .iter()
                .map(|d| Line::from(vec![
                    Span::styled("  • ", Style::default().fg(Color::DarkGray)),
                    Span::styled(d.clone(), Style::default().fg(Color::Gray)),
                ]))
                .collect();

            let details = Paragraph::new(detail_lines)
                .wrap(Wrap { trim: true });
            f.render_widget(details, chunks[1]);
        }

        // Render buttons
        let button_area = chunks[2];
        let button_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(50),
                Constraint::Percentage(50),
            ])
            .split(button_area);

        // Yes/Confirm button (LEFT)
        let yes_style = if state.selected == 0 {
            match state.severity {
                ConfirmSeverity::Info => Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
                ConfirmSeverity::Warning => Style::default()
                    .fg(Color::Black)
                    .bg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
                ConfirmSeverity::Danger => Style::default()
                    .fg(Color::White)
                    .bg(Color::Red)
                    .add_modifier(Modifier::BOLD),
            }
        } else {
            match state.severity {
                ConfirmSeverity::Info => Style::default().fg(Color::Cyan),
                ConfirmSeverity::Warning => Style::default().fg(Color::Yellow),
                ConfirmSeverity::Danger => Style::default().fg(Color::Red),
            }
        };

        let yes_text = match state.severity {
            ConfirmSeverity::Info => "[ Yes / Continue ]",
            ConfirmSeverity::Warning => "[ Yes / Proceed ]",
            ConfirmSeverity::Danger => "[ CONFIRM DELETE ]",
        };
        let yes_button = Paragraph::new(yes_text)
            .style(yes_style)
            .alignment(Alignment::Center);
        f.render_widget(yes_button, button_chunks[0]);

        // No/Cancel button (RIGHT)
        let no_style = if state.selected == 1 {
            Style::default()
                .fg(Color::Black)
                .bg(Color::White)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };
        let no_button = Paragraph::new("[ No / Cancel ]")
            .style(no_style)
            .alignment(Alignment::Center);
        f.render_widget(no_button, button_chunks[1]);
    }
}

/// Create a confirmation dialog for formatting a partition
pub fn format_partition_confirm(partition: &str, filesystem: &str) -> ConfirmDialogState {
    ConfirmDialogState::new(
        "Format Partition",
        &format!("Format {} with {}?", partition, filesystem),
        ConfirmSeverity::Warning,
        "format_partition",
    )
    .with_detail("All data on this partition will be erased")
    .with_detail("This operation cannot be undone")
    .with_action_data(partition)
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
    .with_action_data(disk)
}

/// Create a confirmation dialog for installing bootloader
pub fn bootloader_confirm(bootloader: &str, disk: &str) -> ConfirmDialogState {
    ConfirmDialogState::new(
        "Install Bootloader",
        &format!("Install {} to {}?", bootloader, disk),
        ConfirmSeverity::Warning,
        "install_bootloader",
    )
    .with_detail("The boot sector will be modified")
    .with_detail("Existing bootloader may be overwritten")
    .with_action_data(disk)
}

/// Create a confirmation dialog for starting installation
pub fn start_install_confirm() -> ConfirmDialogState {
    ConfirmDialogState::new(
        "Start Installation",
        "Begin Arch Linux installation?",
        ConfirmSeverity::Warning,
        "start_installation",
    )
    .with_detail("The target disk will be formatted")
    .with_detail("This process may take several minutes")
    .with_detail("Do not power off during installation")
}
