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
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
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
            Span::styled("  ⚡ ", Style::default().fg(Color::Yellow)),
            Span::styled(
                "Quick, Reproducible Installs",
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Automated installation uses a configuration file",
            Style::default().fg(Color::Gray),
        )]),
        Line::from(vec![Span::styled(
            "  to install Arch Linux with your preferred settings.",
            Style::default().fg(Color::Gray),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  ✓ ", Style::default().fg(Color::Green)),
            Span::styled(
                "Disk partitioning & formatting",
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled("  ✓ ", Style::default().fg(Color::Green)),
            Span::styled(
                "Bootloader installation (GRUB/systemd-boot)",
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled("  ✓ ", Style::default().fg(Color::Green)),
            Span::styled(
                "User account creation",
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled("  ✓ ", Style::default().fg(Color::Green)),
            Span::styled(
                "Desktop environment setup",
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled("  ✓ ", Style::default().fg(Color::Green)),
            Span::styled(
                "Custom package installation",
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  📁 ", Style::default().fg(Color::Cyan)),
            Span::styled("Supported formats: ", Style::default().fg(Color::Gray)),
            Span::styled(".toml, .json", Style::default().fg(Color::Cyan)),
        ]),
    ];

    let description = Paragraph::new(description_lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Overview ")
                .title_style(
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .style(Style::default().bg(Color::Rgb(20, 20, 30)));
    f.render_widget(description, content_chunks[0]);

    // Right panel - Config file format example
    let config_lines = vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            "  # Example config.toml",
            Style::default().fg(Color::DarkGray),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  hostname", Style::default().fg(Color::Cyan)),
            Span::styled(" = ", Style::default().fg(Color::White)),
            Span::styled("\"archlinux\"", Style::default().fg(Color::Green)),
        ]),
        Line::from(vec![
            Span::styled("  username", Style::default().fg(Color::Cyan)),
            Span::styled(" = ", Style::default().fg(Color::White)),
            Span::styled("\"user\"", Style::default().fg(Color::Green)),
        ]),
        Line::from(vec![
            Span::styled("  install_disk", Style::default().fg(Color::Cyan)),
            Span::styled(" = ", Style::default().fg(Color::White)),
            Span::styled("\"/dev/sda\"", Style::default().fg(Color::Green)),
        ]),
        Line::from(vec![
            Span::styled("  bootloader", Style::default().fg(Color::Cyan)),
            Span::styled(" = ", Style::default().fg(Color::White)),
            Span::styled("\"grub\"", Style::default().fg(Color::Green)),
        ]),
        Line::from(vec![
            Span::styled("  desktop_environment", Style::default().fg(Color::Cyan)),
            Span::styled(" = ", Style::default().fg(Color::White)),
            Span::styled("\"gnome\"", Style::default().fg(Color::Green)),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  [packages]",
            Style::default().fg(Color::Yellow),
        )]),
        Line::from(vec![
            Span::styled("  base", Style::default().fg(Color::Cyan)),
            Span::styled(" = [", Style::default().fg(Color::White)),
            Span::styled("\"vim\", \"git\"", Style::default().fg(Color::Green)),
            Span::styled("]", Style::default().fg(Color::White)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Press ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                "Enter",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " to browse for config files",
                Style::default().fg(Color::DarkGray),
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
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .style(Style::default().bg(Color::Rgb(20, 20, 30)));
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
        .style(Style::default().fg(Color::Green));
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
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };

    ListItem::new(text).style(style)
}

/// Render start button
fn render_start_button(f: &mut Frame, area: Rect, state: &AppState) {
    let is_selected = state.config_scroll.selected_index == state.config.options.len();
    let button_text = if is_selected {
        "  START INSTALLATION (Press Enter)  "
    } else {
        "  START INSTALLATION  "
    };

    let style = if is_selected {
        Style::default()
            .fg(Color::Yellow)
            .bg(Color::Rgb(0, 100, 0))
    } else {
        Style::default().fg(Color::Green)
    };

    let button = Paragraph::new(button_text)
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Center)
        .style(style);
    f.render_widget(button, area);
}
