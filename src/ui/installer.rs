//! Installation UI rendering module
//!
//! This module handles rendering of installation-related UI:
//! - Configuration UI
//! - Automated install UI
//! - Installation progress
//! - Completion screen
//! - Tool execution

use super::header::{render_installer_output, render_progress_bar, HeaderRenderer};
use crate::app::AppState;
use crate::theme::Colors;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

/// Render configuration UI in specified area
pub fn render_configuration_ui_in_area(
    f: &mut Frame,
    state: &AppState,
    area: Rect,
    header: &HeaderRenderer,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(7), // Header
            Constraint::Length(3), // Title
            Constraint::Min(10),   // Configuration options
            Constraint::Length(3), // Start button
        ])
        .split(area);

    header.render_header(f, chunks[0]);
    header.render_title(f, chunks[1], "Arch Linux Installation Configuration");
    render_config_options(f, chunks[2], state);
    render_start_button(f, chunks[3], state);
}

/// Render automated install UI in specified area
pub fn render_automated_install_ui_in_area(
    f: &mut Frame,
    _state: &AppState,
    area: Rect,
    header: &HeaderRenderer,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(7), // Header
            Constraint::Length(3), // Title
            Constraint::Min(0),    // Content
        ])
        .split(area);

    header.render_header(f, chunks[0]);
    header.render_title(f, chunks[1], "Automated Installation");

    // Split content area into description and config format
    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[2]);

    // Left panel - Description and instructions
    let description_lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  ⚡ ", Style::default().fg(Colors::SECONDARY)),
            Span::styled(
                "Quick, Reproducible Installs",
                Style::default()
                    .fg(Colors::FG_PRIMARY)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Automated installation uses a configuration file",
            Style::default().fg(Colors::FG_SECONDARY),
        )]),
        Line::from(vec![Span::styled(
            "  to install Arch Linux with your preferred settings.",
            Style::default().fg(Colors::FG_SECONDARY),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  ✓ ", Style::default().fg(Colors::SUCCESS)),
            Span::styled(
                "Disk partitioning & formatting",
                Style::default().fg(Colors::FG_PRIMARY),
            ),
        ]),
        Line::from(vec![
            Span::styled("  ✓ ", Style::default().fg(Colors::SUCCESS)),
            Span::styled(
                "Bootloader installation (GRUB/systemd-boot)",
                Style::default().fg(Colors::FG_PRIMARY),
            ),
        ]),
        Line::from(vec![
            Span::styled("  ✓ ", Style::default().fg(Colors::SUCCESS)),
            Span::styled(
                "User account creation",
                Style::default().fg(Colors::FG_PRIMARY),
            ),
        ]),
        Line::from(vec![
            Span::styled("  ✓ ", Style::default().fg(Colors::SUCCESS)),
            Span::styled(
                "Desktop environment setup",
                Style::default().fg(Colors::FG_PRIMARY),
            ),
        ]),
        Line::from(vec![
            Span::styled("  ✓ ", Style::default().fg(Colors::SUCCESS)),
            Span::styled(
                "Custom package installation",
                Style::default().fg(Colors::FG_PRIMARY),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  📁 ", Style::default().fg(Colors::PRIMARY)),
            Span::styled("Supported formats: ", Style::default().fg(Colors::FG_SECONDARY)),
            Span::styled(".toml, .json", Style::default().fg(Colors::PRIMARY)),
        ]),
    ];

    let description = Paragraph::new(description_lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Overview ")
                .title_style(
                    Style::default()
                        .fg(Colors::PRIMARY)
                        .add_modifier(Modifier::BOLD),
                )
                .border_style(Style::default().fg(Colors::PRIMARY)),
        )
        .style(Style::default().bg(Colors::BG_PRIMARY));
    f.render_widget(description, content_chunks[0]);

    // Right panel - Config file format example
    let config_lines = vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            "  # Example config.toml",
            Style::default().fg(Colors::FG_MUTED),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  hostname", Style::default().fg(Colors::PRIMARY)),
            Span::styled(" = ", Style::default().fg(Colors::FG_PRIMARY)),
            Span::styled("\"archlinux\"", Style::default().fg(Colors::SUCCESS)),
        ]),
        Line::from(vec![
            Span::styled("  username", Style::default().fg(Colors::PRIMARY)),
            Span::styled(" = ", Style::default().fg(Colors::FG_PRIMARY)),
            Span::styled("\"user\"", Style::default().fg(Colors::SUCCESS)),
        ]),
        Line::from(vec![
            Span::styled("  install_disk", Style::default().fg(Colors::PRIMARY)),
            Span::styled(" = ", Style::default().fg(Colors::FG_PRIMARY)),
            Span::styled("\"/dev/sda\"", Style::default().fg(Colors::SUCCESS)),
        ]),
        Line::from(vec![
            Span::styled("  bootloader", Style::default().fg(Colors::PRIMARY)),
            Span::styled(" = ", Style::default().fg(Colors::FG_PRIMARY)),
            Span::styled("\"grub\"", Style::default().fg(Colors::SUCCESS)),
        ]),
        Line::from(vec![
            Span::styled("  desktop_environment", Style::default().fg(Colors::PRIMARY)),
            Span::styled(" = ", Style::default().fg(Colors::FG_PRIMARY)),
            Span::styled("\"gnome\"", Style::default().fg(Colors::SUCCESS)),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  [packages]",
            Style::default().fg(Colors::SECONDARY),
        )]),
        Line::from(vec![
            Span::styled("  base", Style::default().fg(Colors::PRIMARY)),
            Span::styled(" = [", Style::default().fg(Colors::FG_PRIMARY)),
            Span::styled("\"vim\", \"git\"", Style::default().fg(Colors::SUCCESS)),
            Span::styled("]", Style::default().fg(Colors::FG_PRIMARY)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Press ", Style::default().fg(Colors::FG_MUTED)),
            Span::styled(
                "Enter",
                Style::default()
                    .fg(Colors::SECONDARY)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " to browse for config files",
                Style::default().fg(Colors::FG_MUTED),
            ),
        ]),
    ];

    let config_example = Paragraph::new(config_lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Config Format ")
                .title_style(
                    Style::default()
                        .fg(Colors::PRIMARY)
                        .add_modifier(Modifier::BOLD),
                )
                .border_style(Style::default().fg(Colors::PRIMARY)),
        )
        .style(Style::default().bg(Colors::BG_PRIMARY));
    f.render_widget(config_example, content_chunks[1]);
}

/// Render tool execution in specified area
pub fn render_tool_execution_in_area(
    f: &mut Frame,
    state: &AppState,
    area: Rect,
    header: &HeaderRenderer,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(7), // Header
            Constraint::Length(3), // Title
            Constraint::Min(0),    // Output
        ])
        .split(area);

    header.render_header(f, chunks[0]);

    let title = if let Some(ref tool) = state.current_tool {
        format!("Running: {}", tool)
    } else {
        "Tool Execution".to_string()
    };
    header.render_title(f, chunks[1], &title);

    // Render tool output
    let output_items: Vec<ListItem> = state
        .tool_output
        .iter()
        .map(|line| ListItem::new(line.as_str()))
        .collect();

    let output_list = List::new(output_items)
        .block(Block::default().borders(Borders::ALL).title("Output"));
    f.render_widget(output_list, chunks[2]);
}

/// Render installation UI in specified area
pub fn render_installation_ui_in_area(
    f: &mut Frame,
    state: &AppState,
    area: Rect,
    header: &HeaderRenderer,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(7), // Header
            Constraint::Length(3), // Title
            Constraint::Length(3), // Progress bar
            Constraint::Min(0),    // Installer output
        ])
        .split(area);

    header.render_header(f, chunks[0]);
    header.render_title(f, chunks[1], "Arch Linux Installation Progress");
    render_progress_bar(f, chunks[2], state.installation_progress as u16);
    render_installer_output(f, chunks[3], &state.installer_output);
}

/// Render completion UI in specified area
pub fn render_completion_ui_in_area(
    f: &mut Frame,
    state: &AppState,
    area: Rect,
    header: &HeaderRenderer,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(7), // Header
            Constraint::Length(3), // Title
            Constraint::Min(0),    // Completion message
        ])
        .split(area);

    header.render_header(f, chunks[0]);
    header.render_title(f, chunks[1], "Installation Complete");

    let message = Paragraph::new(state.status_message.clone())
        .block(Block::default().borders(Borders::ALL).title("Status"))
        .alignment(Alignment::Center)
        .style(Style::default().fg(Colors::SUCCESS));
    f.render_widget(message, chunks[2]);
}

/// Render configuration options list with scrolling
fn render_config_options(f: &mut Frame, area: Rect, state: &AppState) {
    let (start_idx, end_idx) = state.config_scroll.visible_range();

    // Create visible items with proper styling
    let visible_items: Vec<ListItem> = state
        .config
        .options
        .iter()
        .enumerate()
        .skip(start_idx)
        .take(end_idx - start_idx)
        .map(|(index, option)| create_config_item(option, index, state.config_scroll.selected_index))
        .collect();

    // Create title with page info
    let title = if let Some((current_page, total_pages)) = state.config_scroll.page_info() {
        format!(
            "Configuration Options (Page {}/{} - ↑↓ Scroll, PgUp/PgDn, Home/End)",
            current_page, total_pages
        )
    } else {
        "Configuration Options".to_string()
    };

    let list = List::new(visible_items).block(Block::default().borders(Borders::ALL).title(title));
    f.render_widget(list, area);
}

/// Create a configuration item with proper styling
fn create_config_item(
    option: &crate::config::ConfigOption,
    index: usize,
    current_step: usize,
) -> ListItem<'static> {
    let display_value = if option.value.is_empty() {
        "[Press Enter]".to_string()
    } else {
        // Special display logic for different field types
        match option.name.as_str() {
            "User Password" | "Root Password" => "***".to_string(),
            "Additional Pacman Packages" | "Additional AUR Packages" => {
                if option.value.is_empty() {
                    "[Press Enter]".to_string()
                } else {
                    option.value.clone()
                }
            }
            _ => option.value.clone(),
        }
    };

    let text = format!("{}: {}", option.name, display_value);
    let style = if index == current_step {
        Style::default().fg(Colors::SECONDARY)
    } else {
        Style::default()
    };

    ListItem::new(text).style(style)
}

/// Render action buttons (Test Config + Start Install) - Sprint 8
fn render_start_button(f: &mut Frame, area: Rect, state: &AppState) {
    let is_button_row = state.config_scroll.selected_index == state.config.options.len();

    // Split area into two buttons
    let button_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // Test Config button (index 0)
    let test_selected = is_button_row && state.installer_button_selection == 0;
    let test_text = if test_selected {
        "  TEST CONFIG (Enter)  "
    } else {
        "  TEST CONFIG  "
    };
    let test_style = if test_selected {
        Style::default()
            .fg(Colors::BG_PRIMARY)
            .bg(Colors::PRIMARY)
    } else if is_button_row {
        Style::default().fg(Colors::PRIMARY)
    } else {
        Style::default().fg(Colors::FG_MUTED)
    };
    let test_button = Paragraph::new(test_text)
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Center)
        .style(test_style);
    f.render_widget(test_button, button_chunks[0]);

    // Start Install button (index 1)
    let start_selected = is_button_row && state.installer_button_selection == 1;
    let start_text = if start_selected {
        "  START INSTALLATION (Enter)  "
    } else {
        "  START INSTALLATION  "
    };
    let start_style = if start_selected {
        Style::default()
            .fg(Colors::BG_PRIMARY)
            .bg(Colors::SUCCESS)
    } else if is_button_row {
        Style::default().fg(Colors::SUCCESS)
    } else {
        Style::default().fg(Colors::FG_MUTED)
    };
    let start_button = Paragraph::new(start_text)
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Center)
        .style(start_style);
    f.render_widget(start_button, button_chunks[1]);
}

/// Render dry-run summary in specified area (Sprint 8)
pub fn render_dry_run_summary_in_area(
    f: &mut Frame,
    state: &AppState,
    area: Rect,
    header: &HeaderRenderer,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(7), // Header
            Constraint::Length(3), // Title
            Constraint::Min(0),    // Summary content
        ])
        .split(area);

    header.render_header(f, chunks[0]);
    header.render_title(f, chunks[1], "Dry Run Summary - Actions to be Performed");

    // Build summary content
    let summary_lines: Vec<ListItem> = if let Some(ref summary) = state.dry_run_summary {
        summary
            .iter()
            .map(|line| {
                let style = if line.starts_with("[DESTRUCTIVE]") {
                    Style::default().fg(Colors::ERROR)
                } else if line.starts_with("[SKIP]") {
                    Style::default().fg(Colors::FG_MUTED)
                } else if line.starts_with("  ->") {
                    Style::default().fg(Colors::FG_SECONDARY)
                } else {
                    Style::default().fg(Colors::FG_PRIMARY)
                };
                ListItem::new(line.as_str()).style(style)
            })
            .collect()
    } else {
        vec![ListItem::new("No actions to perform").style(Style::default().fg(Colors::FG_MUTED))]
    };

    let summary_list = List::new(summary_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Actions (Press B to go back, Enter to dismiss) ")
            .title_style(Style::default().fg(Colors::PRIMARY)),
    );
    f.render_widget(summary_list, chunks[2]);
}
