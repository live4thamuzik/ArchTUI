//! Guided installer wizard screens (Sprint 7).
//!
//! This module provides the individual screens for the guided installer workflow:
//! - `DiskSelect` - Select installation target disk
//! - `UserConfig` - Configure hostname, username, and password
//!
//! # Safety
//!
//! - Disk selection prominently displays size and model to prevent accidental wipes
//! - Passwords are masked during input
//! - User cannot proceed without completing required fields

use crate::theme::Colors;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};
use std::path::PathBuf;

// ============================================================================
// Disk Information
// ============================================================================

/// Information about a detected disk.
///
/// Used for display in the disk selection screen. All fields are populated
/// from `lsblk` output via `SystemInfoArgs`.
#[derive(Debug, Clone)]
pub struct DiskInfo {
    /// Device path (e.g., `/dev/sda`).
    pub device: PathBuf,
    /// Disk model name.
    pub model: String,
    /// Disk size in human-readable format (e.g., "500G").
    pub size: String,
    /// Disk size in bytes for sorting.
    pub size_bytes: u64,
    /// Transport type (e.g., "sata", "nvme", "usb").
    pub transport: String,
    /// Whether the disk is removable.
    pub removable: bool,
}

impl DiskInfo {
    /// Create DiskInfo from lsblk JSON output fields.
    pub fn from_lsblk(
        name: &str,
        model: Option<&str>,
        size: Option<&str>,
        tran: Option<&str>,
        rm: bool,
    ) -> Self {
        Self {
            device: PathBuf::from(format!("/dev/{}", name)),
            model: model.unwrap_or("Unknown").to_string(),
            size: size.unwrap_or("Unknown").to_string(),
            size_bytes: Self::parse_size(size.unwrap_or("0")),
            transport: tran.unwrap_or("unknown").to_string(),
            removable: rm,
        }
    }

    /// Parse size string to bytes (approximate).
    fn parse_size(size: &str) -> u64 {
        let size = size.trim();
        if size.is_empty() {
            return 0;
        }

        let (num_str, suffix) = size.split_at(size.len().saturating_sub(1));
        let num: f64 = num_str.parse().unwrap_or(0.0);

        match suffix.to_uppercase().as_str() {
            "K" => (num * 1024.0) as u64,
            "M" => (num * 1024.0 * 1024.0) as u64,
            "G" => (num * 1024.0 * 1024.0 * 1024.0) as u64,
            "T" => (num * 1024.0 * 1024.0 * 1024.0 * 1024.0) as u64,
            _ => num as u64,
        }
    }

    /// Format disk info for display in the list.
    pub fn display_line(&self) -> String {
        let warning = if self.removable { " [REMOVABLE]" } else { "" };
        format!(
            "{} - {} ({}){}",
            self.device.display(),
            self.model,
            self.size,
            warning
        )
    }
}

// ============================================================================
// Disk Select Screen
// ============================================================================

/// State for the disk selection screen.
#[derive(Debug, Clone, Default)]
pub struct DiskSelectState {
    /// List of detected disks.
    pub disks: Vec<DiskInfo>,
    /// Currently selected disk index.
    pub selected: usize,
    /// Whether disk list is being loaded.
    pub loading: bool,
    /// Error message if disk detection failed.
    pub error: Option<String>,
}

impl DiskSelectState {
    /// Move selection up.
    pub fn select_previous(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    /// Move selection down.
    pub fn select_next(&mut self) {
        if !self.disks.is_empty() && self.selected < self.disks.len() - 1 {
            self.selected += 1;
        }
    }

    /// Get the currently selected disk.
    pub fn get_selected(&self) -> Option<&DiskInfo> {
        self.disks.get(self.selected)
    }
}

/// Render the disk selection screen.
///
/// # Safety Display Requirements
///
/// - Disk model is shown prominently
/// - Disk size is shown prominently
/// - Removable disks are flagged with a warning
/// - User must explicitly confirm selection
pub fn render_disk_select_screen(
    f: &mut Frame,
    area: Rect,
    state: &DiskSelectState,
    selected_disk: Option<&PathBuf>,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title
            Constraint::Length(5), // Warning
            Constraint::Min(10),   // Disk list
            Constraint::Length(3), // Instructions
        ])
        .split(area);

    // Title
    let title = Paragraph::new("Select Installation Disk")
        .style(
            Style::default()
                .fg(Colors::PRIMARY)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::BOTTOM));
    f.render_widget(title, chunks[0]);

    // Safety warning
    let warning_lines = vec![
        Line::from(vec![
            Span::styled("  WARNING: ", Style::default().fg(Colors::ERROR).add_modifier(Modifier::BOLD)),
            Span::styled(
                "The selected disk will be COMPLETELY ERASED!",
                Style::default().fg(Colors::ERROR),
            ),
        ]),
        Line::from(vec![Span::styled(
            "  Verify the disk model and size before proceeding.",
            Style::default().fg(Colors::WARNING),
        )]),
        Line::from(""),
    ];
    let warning = Paragraph::new(warning_lines);
    f.render_widget(warning, chunks[1]);

    // Disk list
    if state.loading {
        let loading = Paragraph::new("  Detecting disks...")
            .style(Style::default().fg(Colors::FG_SECONDARY));
        f.render_widget(loading, chunks[2]);
    } else if let Some(ref err) = state.error {
        let error = Paragraph::new(format!("  Error: {}", err))
            .style(Style::default().fg(Colors::ERROR));
        f.render_widget(error, chunks[2]);
    } else if state.disks.is_empty() {
        let no_disks = Paragraph::new("  No disks detected. Check your hardware.")
            .style(Style::default().fg(Colors::WARNING));
        f.render_widget(no_disks, chunks[2]);
    } else {
        let items: Vec<ListItem> = state
            .disks
            .iter()
            .enumerate()
            .map(|(i, disk)| {
                let is_selected = Some(&disk.device) == selected_disk;
                let marker = if is_selected { " [SELECTED] " } else { "  " };

                let style = if i == state.selected {
                    Style::default()
                        .fg(Colors::BG_PRIMARY)
                        .bg(Colors::PRIMARY)
                        .add_modifier(Modifier::BOLD)
                } else if is_selected {
                    Style::default()
                        .fg(Colors::SUCCESS)
                        .add_modifier(Modifier::BOLD)
                } else if disk.removable {
                    Style::default().fg(Colors::WARNING)
                } else {
                    Style::default().fg(Colors::FG_PRIMARY)
                };

                ListItem::new(format!("{}{}", marker, disk.display_line())).style(style)
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Available Disks ")
                    .title_style(Style::default().fg(Colors::SECONDARY)),
            )
            .highlight_style(
                Style::default()
                    .fg(Colors::BG_PRIMARY)
                    .bg(Colors::PRIMARY)
                    .add_modifier(Modifier::BOLD),
            );

        let mut list_state = ListState::default();
        list_state.select(Some(state.selected));
        f.render_stateful_widget(list, chunks[2], &mut list_state);
    }

    // Instructions
    let instructions = Paragraph::new(vec![Line::from(vec![
        Span::styled(" [Enter] ", Style::default().fg(Colors::PRIMARY).add_modifier(Modifier::BOLD)),
        Span::raw("Select  "),
        Span::styled(" [j/k] ", Style::default().fg(Colors::PRIMARY).add_modifier(Modifier::BOLD)),
        Span::raw("Navigate  "),
        Span::styled(" [Esc] ", Style::default().fg(Colors::PRIMARY).add_modifier(Modifier::BOLD)),
        Span::raw("Back"),
    ])])
    .alignment(Alignment::Center)
    .block(Block::default().borders(Borders::TOP));
    f.render_widget(instructions, chunks[3]);
}

// ============================================================================
// User Config Screen
// ============================================================================

/// Input field identifiers for user configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserConfigField {
    Hostname,
    Username,
    Password,
    ConfirmPassword,
    RootPassword,
}

impl UserConfigField {
    /// Get all fields in order.
    pub fn all() -> &'static [Self] {
        &[
            Self::Hostname,
            Self::Username,
            Self::Password,
            Self::ConfirmPassword,
            Self::RootPassword,
        ]
    }

    /// Get field label.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Hostname => "Hostname",
            Self::Username => "Username",
            Self::Password => "Password",
            Self::ConfirmPassword => "Confirm Password",
            Self::RootPassword => "Root Password (optional)",
        }
    }

    /// Check if field should be masked.
    pub fn is_password(&self) -> bool {
        matches!(
            self,
            Self::Password | Self::ConfirmPassword | Self::RootPassword
        )
    }

    /// Check if field is required.
    pub fn is_required(&self) -> bool {
        matches!(self, Self::Hostname | Self::Username | Self::Password | Self::ConfirmPassword)
    }
}

/// State for the user configuration screen.
#[derive(Debug, Clone, Default)]
pub struct UserConfigState {
    /// Current field being edited.
    pub current_field: usize,
    /// Field values.
    pub hostname: String,
    pub username: String,
    pub password: String,
    pub confirm_password: String,
    pub root_password: String,
    /// Whether user should have sudo access.
    pub sudo_enabled: bool,
    /// Validation error message.
    pub error: Option<String>,
}

impl UserConfigState {
    /// Create new state with defaults.
    pub fn new() -> Self {
        Self {
            sudo_enabled: true,
            ..Default::default()
        }
    }

    /// Move to previous field.
    pub fn previous_field(&mut self) {
        if self.current_field > 0 {
            self.current_field -= 1;
        }
    }

    /// Move to next field.
    pub fn next_field(&mut self) {
        let max = UserConfigField::all().len() - 1;
        if self.current_field < max {
            self.current_field += 1;
        }
    }

    /// Get current field.
    pub fn current(&self) -> UserConfigField {
        UserConfigField::all()[self.current_field]
    }

    /// Get mutable reference to current field value.
    pub fn current_value_mut(&mut self) -> &mut String {
        match self.current() {
            UserConfigField::Hostname => &mut self.hostname,
            UserConfigField::Username => &mut self.username,
            UserConfigField::Password => &mut self.password,
            UserConfigField::ConfirmPassword => &mut self.confirm_password,
            UserConfigField::RootPassword => &mut self.root_password,
        }
    }

    /// Validate all fields.
    pub fn validate(&self) -> Result<(), String> {
        if self.hostname.is_empty() {
            return Err("Hostname is required".to_string());
        }
        if self.hostname.len() > 63 {
            return Err("Hostname must be 63 characters or less".to_string());
        }
        if !self.hostname.chars().all(|c| c.is_alphanumeric() || c == '-') {
            return Err("Hostname can only contain letters, numbers, and hyphens".to_string());
        }

        if self.username.is_empty() {
            return Err("Username is required".to_string());
        }
        if self.username.starts_with(|c: char| c.is_numeric()) {
            return Err("Username cannot start with a number".to_string());
        }
        if !self.username.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
            return Err("Username can only contain letters, numbers, underscores, and hyphens".to_string());
        }

        if self.password.is_empty() {
            return Err("Password is required".to_string());
        }
        if self.password != self.confirm_password {
            return Err("Passwords do not match".to_string());
        }

        Ok(())
    }

    /// Check if configuration is valid.
    pub fn is_valid(&self) -> bool {
        self.validate().is_ok()
    }
}

/// Render the user configuration screen.
pub fn render_user_config_screen(
    f: &mut Frame,
    area: Rect,
    state: &UserConfigState,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Title
            Constraint::Min(15),    // Form fields
            Constraint::Length(3),  // Error/status
            Constraint::Length(3),  // Instructions
        ])
        .split(area);

    // Title
    let title = Paragraph::new("User Configuration")
        .style(
            Style::default()
                .fg(Colors::PRIMARY)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::BOTTOM));
    f.render_widget(title, chunks[0]);

    // Form fields
    let form_chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3), // Hostname
            Constraint::Length(3), // Username
            Constraint::Length(3), // Password
            Constraint::Length(3), // Confirm Password
            Constraint::Length(3), // Root Password
            Constraint::Length(2), // Sudo checkbox
        ])
        .split(chunks[1]);

    let fields = UserConfigField::all();
    let values = [
        &state.hostname,
        &state.username,
        &state.password,
        &state.confirm_password,
        &state.root_password,
    ];

    for (i, (field, value)) in fields.iter().zip(values.iter()).enumerate() {
        let is_current = i == state.current_field;
        let is_required = field.is_required();

        let label_style = if is_current {
            Style::default().fg(Colors::PRIMARY).add_modifier(Modifier::BOLD)
        } else if is_required && value.is_empty() {
            Style::default().fg(Colors::WARNING)
        } else {
            Style::default().fg(Colors::FG_SECONDARY)
        };

        let display_value = if field.is_password() && !value.is_empty() {
            "*".repeat(value.len())
        } else {
            (*value).clone()
        };

        let cursor = if is_current { "_" } else { "" };
        let required_marker = if is_required { " *" } else { "" };

        let field_text = format!(
            "  {}{}: {}{}",
            field.label(),
            required_marker,
            display_value,
            cursor
        );

        let border_style = if is_current {
            Style::default().fg(Colors::BORDER_ACTIVE)
        } else {
            Style::default().fg(Colors::BORDER_INACTIVE)
        };

        let field_widget = Paragraph::new(field_text)
            .style(label_style)
            .block(Block::default().borders(Borders::ALL).border_style(border_style));

        f.render_widget(field_widget, form_chunks[i]);
    }

    // Sudo checkbox
    let sudo_marker = if state.sudo_enabled { "[x]" } else { "[ ]" };
    let sudo_style = Style::default().fg(Colors::FG_PRIMARY);
    let sudo = Paragraph::new(format!("  {} Enable sudo access for user", sudo_marker))
        .style(sudo_style);
    f.render_widget(sudo, form_chunks[5]);

    // Error/status
    if let Some(ref error) = state.error {
        let error_widget = Paragraph::new(format!("  Error: {}", error))
            .style(Style::default().fg(Colors::ERROR));
        f.render_widget(error_widget, chunks[2]);
    } else if state.is_valid() {
        let valid = Paragraph::new("  Configuration valid")
            .style(Style::default().fg(Colors::SUCCESS));
        f.render_widget(valid, chunks[2]);
    } else {
        let hint = Paragraph::new("  * Required fields")
            .style(Style::default().fg(Colors::FG_SECONDARY));
        f.render_widget(hint, chunks[2]);
    }

    // Instructions
    let instructions = Paragraph::new(vec![Line::from(vec![
        Span::styled(" [Tab] ", Style::default().fg(Colors::PRIMARY).add_modifier(Modifier::BOLD)),
        Span::raw("Next field  "),
        Span::styled(" [Shift+Tab] ", Style::default().fg(Colors::PRIMARY).add_modifier(Modifier::BOLD)),
        Span::raw("Previous  "),
        Span::styled(" [Enter] ", Style::default().fg(Colors::PRIMARY).add_modifier(Modifier::BOLD)),
        Span::raw("Continue"),
    ])])
    .alignment(Alignment::Center)
    .block(Block::default().borders(Borders::TOP));
    f.render_widget(instructions, chunks[3]);
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::{WizardData, WizardState};

    #[test]
    fn test_wizard_state_transitions() {
        let state = WizardState::Welcome;
        assert_eq!(state.next(), Some(WizardState::DiskSelect));
        assert_eq!(state.previous(), None);

        let state = WizardState::UserConfig;
        assert_eq!(state.next(), Some(WizardState::InstallProgress));
        assert_eq!(state.previous(), Some(WizardState::PackageSelect));

        // Cannot go back from installation
        let state = WizardState::InstallProgress;
        assert_eq!(state.previous(), None);
    }

    #[test]
    fn test_wizard_data_validation() {
        let mut data = WizardData::default();
        assert!(!data.has_valid_disk());
        assert!(!data.has_valid_user());
        assert!(!data.is_ready_for_install());

        data.selected_disk = Some(PathBuf::from("/dev/sda"));
        assert!(data.has_valid_disk());
        assert!(!data.is_ready_for_install());

        data.hostname = Some("archlinux".to_string());
        data.username = Some("user".to_string());
        data.password = Some("password".to_string());
        assert!(data.has_valid_user());
        assert!(data.is_ready_for_install());
    }

    #[test]
    fn test_wizard_data_zero_sensitive() {
        let mut data = WizardData::default();
        data.password = Some("secret123".to_string());
        data.root_password = Some("rootpass".to_string());

        data.zero_sensitive_data();

        assert!(data.password.is_none());
        assert!(data.root_password.is_none());
    }

    #[test]
    fn test_user_config_validation() {
        let mut state = UserConfigState::new();

        // Empty should fail
        assert!(state.validate().is_err());

        state.hostname = "myhost".to_string();
        state.username = "user".to_string();
        state.password = "pass".to_string();
        state.confirm_password = "pass".to_string();

        assert!(state.validate().is_ok());

        // Mismatched passwords
        state.confirm_password = "different".to_string();
        assert!(state.validate().is_err());
    }

    #[test]
    fn test_disk_info_parse_size() {
        assert_eq!(DiskInfo::parse_size("500G"), 500 * 1024 * 1024 * 1024);
        assert_eq!(DiskInfo::parse_size("1T"), 1024 * 1024 * 1024 * 1024);
        assert_eq!(DiskInfo::parse_size("256M"), 256 * 1024 * 1024);
    }
}
