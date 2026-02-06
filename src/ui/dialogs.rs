//! Dialog rendering module
//!
//! This module handles rendering of all dialogs: input dialogs,
//! confirmation dialogs, embedded terminal, floating output, and file browser.

use crate::app::ToolParameter;
use crate::app::AppState;
use crate::components::confirm_dialog::ConfirmDialog;
use crate::components::floating_window::{FloatingWindow, FloatingWindowConfig};
use crate::components::pty_terminal::PtyTerminal;
use crate::input::InputHandler;
use crate::theme::Colors;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

/// Render embedded terminal
pub fn render_embedded_terminal(
    f: &mut Frame,
    state: &AppState,
    area: Rect,
    pty_terminal: Option<&mut PtyTerminal>,
) {
    if let Some(pty) = pty_terminal {
        let title = if let Some(ref term_state) = state.embedded_terminal {
            format!(" {} - Press Ctrl+Q to exit ", term_state.tool_name)
        } else {
            " Terminal - Press Ctrl+Q to exit ".to_string()
        };
        pty.render(f, area, &title);
    } else {
        // Fallback if no PTY available
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Terminal ")
            .style(Style::default().bg(Colors::SELECTED_FG));
        f.render_widget(block, area);
    }
}

/// Render floating output window
pub fn render_floating_output(f: &mut Frame, state: &AppState) {
    if let Some(ref output) = state.floating_output {
        let config = FloatingWindowConfig {
            title: output.title.clone(),
            width_percent: 80,
            height_percent: 70,
            ..Default::default()
        };
        let window = FloatingWindow::new(config);

        if let Some(progress) = output.progress {
            window.render_with_progress(
                f,
                f.area(),
                &output.content,
                progress,
                &output.status,
            );
        } else {
            window.render_text(
                f,
                f.area(),
                &output.content,
                Some("Press Esc or Enter to close"),
            );
        }
    }
}

/// Render the file browser
pub fn render_file_browser(f: &mut Frame, state: &AppState) {
    if let Some(ref browser) = state.file_browser {
        crate::components::file_browser::FileBrowser::render(f, browser);
    }
}

/// Render tool dialog in specified area
pub fn render_tool_dialog_in_area(f: &mut Frame, state: &AppState, area: Rect) {
    // Render background
    let bg = Block::default().style(Style::default().bg(Colors::SELECTED_FG));
    f.render_widget(bg, area);

    // Delegate to tool dialog renderer
    render_tool_dialog(f, state);
}

/// Render tool parameter dialog
pub fn render_tool_dialog(f: &mut Frame, state: &AppState) {
    if let Some(ref dialog) = state.tool_dialog {
        let area = f.area();

        // Create a centered dialog box
        let dialog_width = (area.width * 3 / 4).min(80);
        let dialog_height = (area.height * 3 / 4).min(20);
        let dialog_x = (area.width - dialog_width) / 2;
        let dialog_y = (area.height - dialog_height) / 2;

        let dialog_rect = Rect::new(dialog_x, dialog_y, dialog_width, dialog_height);

        // Draw dialog background
        f.render_widget(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("Configure {}", dialog.tool_name))
                .style(Style::default().bg(Colors::FG_MUTED)),
            dialog_rect,
        );

        // Render parameter list
        let param_area = Rect::new(
            dialog_x + 2,
            dialog_y + 2,
            dialog_width - 4,
            dialog_height - 6,
        );

        let mut param_items = Vec::new();
        for (i, param) in dialog.parameters.iter().enumerate() {
            let style = if i == dialog.current_param {
                Style::default().fg(Colors::SECONDARY)
            } else {
                Style::default()
            };

            let raw_value = if i < dialog.param_values.len() {
                &dialog.param_values[i]
            } else {
                ""
            };

            // Format display value based on parameter type
            let display_value = match &param.param_type {
                ToolParameter::Password(_) => "*".repeat(raw_value.len()),
                ToolParameter::Selection(options, _) => {
                    // Show selection with arrow indicators
                    let current_val = if raw_value.is_empty() {
                        options.first().map(|s| s.as_str()).unwrap_or("")
                    } else {
                        raw_value
                    };
                    if i == dialog.current_param {
                        format!("< {} >", current_val)
                    } else {
                        current_val.to_string()
                    }
                }
                _ => raw_value.to_string(),
            };

            param_items.push(ListItem::new(Line::from(vec![
                Span::styled(format!("{}: ", param.name), Style::default().fg(Colors::PRIMARY)),
                Span::styled(display_value, style),
            ])));
        }

        let param_list =
            List::new(param_items).highlight_style(Style::default().fg(Colors::SECONDARY));

        f.render_widget(param_list, param_area);

        // Render instructions
        let instruction_area = Rect::new(
            dialog_x + 2,
            dialog_y + dialog_height - 3,
            dialog_width - 4,
            1,
        );

        f.render_widget(
            Paragraph::new("Enter: Next/Execute | Left/Right: Change option | Esc: Back")
                .style(Style::default().fg(Colors::FG_SECONDARY)),
            instruction_area,
        );
    }
}

/// Render confirmation dialog overlay
pub fn render_confirm_dialog(f: &mut Frame, state: &AppState) {
    if let Some(ref dialog_state) = state.confirm_dialog {
        ConfirmDialog::render(f, dialog_state);
    }
}

/// Render input dialog overlay
pub fn render_input_dialog(f: &mut Frame, input_handler: &mut InputHandler) {
    if let Some(ref mut dialog) = input_handler.current_dialog {
        let area = f.area();

        // Fill entire screen with black background
        let background = Block::default()
            .borders(Borders::NONE)
            .style(Style::default().bg(Colors::SELECTED_FG).fg(Colors::SELECTED_FG));
        f.render_widget(background, area);

        // Calculate dialog size and position (centered)
        let dialog_width = 80;
        let dialog_height = 25;
        let x = (area.width.saturating_sub(dialog_width)) / 2;
        let y = (area.height.saturating_sub(dialog_height)) / 2;

        let dialog_area = Rect::new(x, y, dialog_width, dialog_height);

        // Create dialog layout
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Title
                Constraint::Length(3), // Instructions
                Constraint::Min(0),    // Content
                Constraint::Length(3), // Buttons/status
            ])
            .split(dialog_area);

        // Render dialog with black background and white border
        let dialog_bg = Block::default()
            .borders(Borders::ALL)
            .style(Style::default().bg(Colors::SELECTED_FG).fg(Colors::FG_PRIMARY));
        f.render_widget(dialog_bg, dialog_area);

        // Title
        let title = Paragraph::new(dialog.title.clone())
            .block(Block::default().borders(Borders::NONE))
            .alignment(Alignment::Center)
            .style(Style::default().fg(Colors::SECONDARY));
        f.render_widget(title, chunks[0]);

        // Instructions
        let instructions = Paragraph::new(dialog.instructions.clone())
            .block(Block::default().borders(Borders::NONE))
            .alignment(Alignment::Center)
            .style(Style::default().fg(Colors::FG_PRIMARY));
        f.render_widget(instructions, chunks[1]);

        // Content based on input type
        let selected_index = dialog.input_type.get_selected_index();
        match &mut dialog.input_type {
            crate::input::InputType::TextInput { .. } => {
                let input_text = dialog.get_display_value();
                let input_display = if input_text.is_empty() {
                    "Enter value...".to_string()
                } else {
                    input_text
                };

                let input_widget = Paragraph::new(input_display)
                    .block(Block::default().borders(Borders::ALL).title("Input"))
                    .style(Style::default().fg(Colors::SUCCESS));
                f.render_widget(input_widget, chunks[2]);
            }
            crate::input::InputType::Selection {
                scroll_state,
                options,
                ..
            } => {
                let (start, end) = scroll_state.visible_range();
                let items: Vec<ListItem> = options
                    .iter()
                    .enumerate()
                    .skip(start)
                    .take(end - start)
                    .map(|(index, option)| {
                        let style = if index == selected_index {
                            Style::default().fg(Colors::SECONDARY)
                        } else {
                            Style::default()
                        };
                        ListItem::new(option.clone()).style(style)
                    })
                    .collect();

                let list = List::new(items)
                    .block(Block::default().borders(Borders::ALL).title("Options"));
                f.render_widget(list, chunks[2]);
            }
            crate::input::InputType::DiskSelection {
                available_disks, ..
            } => {
                let items: Vec<ListItem> = available_disks
                    .iter()
                    .enumerate()
                    .map(|(index, disk)| {
                        let style = if index == selected_index {
                            Style::default().fg(Colors::SECONDARY)
                        } else {
                            Style::default()
                        };
                        ListItem::new(disk.clone()).style(style)
                    })
                    .collect();

                let list = List::new(items).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Available Disks"),
                );
                f.render_widget(list, chunks[2]);
            }
            crate::input::InputType::PackageSelection {
                current_input,
                output_lines,
                scroll_offset,
                package_list,
                show_search_results,
                search_results,
                list_state,
                is_pacman,
                ..
            } => {
                let title = if *is_pacman {
                    "Interactive Pacman Package Selection"
                } else {
                    "Interactive AUR Package Selection"
                };

                let block = Block::default()
                    .borders(Borders::ALL)
                    .title(title)
                    .title_style(
                        Style::default()
                            .fg(Colors::PRIMARY)
                            .add_modifier(Modifier::BOLD),
                    )
                    .title_bottom("Type commands, Enter to execute, Esc to exit")
                    .style(Style::default().bg(Colors::SELECTED_FG).fg(Colors::FG_PRIMARY));

                if *show_search_results && !search_results.is_empty() {
                    // Display search results with scrolling
                    let package_items: Vec<ListItem> = search_results
                        .iter()
                        .map(|p| {
                            let status = if p.installed { "[I]" } else { "[ ]" };

                            // Check if this package is already selected in our config
                            let is_selected = package_list.contains(&p.name);
                            let selection_indicator = if is_selected { "✓" } else { " " };

                            let text = format!(
                                "{} {} {}/{} ({}) - {}",
                                status,
                                selection_indicator,
                                p.repo,
                                p.name,
                                p.version,
                                p.description
                            );

                            // Style selected packages differently
                            let style = if is_selected {
                                Style::default()
                                    .fg(Colors::SUCCESS)
                                    .add_modifier(Modifier::BOLD)
                            } else {
                                Style::default()
                            };

                            ListItem::new(text).style(style)
                        })
                        .collect();

                    let search_list = List::new(package_items)
                        .block(block.title(
                            "Search Results - ↑↓ Navigate | Enter Toggle Selection | Esc Exit",
                        ))
                        .highlight_style(
                            Style::default()
                                .fg(Colors::SUCCESS_LIGHT)
                                .add_modifier(Modifier::BOLD),
                        )
                        .highlight_symbol(">> ");

                    f.render_stateful_widget(search_list, chunks[2], list_state);
                } else {
                    // Display normal command interface (like old version)
                    let max_visible_lines: usize = 15;
                    let mut list_items: Vec<ListItem> = output_lines
                        .iter()
                        .skip(*scroll_offset)
                        .take(max_visible_lines.saturating_sub(1))
                        .map(|line| ListItem::new(line.as_str()))
                        .collect();

                    // Add current input line
                    let prompt = if *is_pacman {
                        "Package selection> "
                    } else {
                        "AUR package selection> "
                    };
                    let input_line = format!("{}{}", prompt, current_input);
                    list_items.push(
                        ListItem::new(input_line).style(Style::default().fg(Colors::SECONDARY)),
                    );

                    let list = List::new(list_items)
                        .block(block)
                        .style(Style::default().bg(Colors::SELECTED_FG).fg(Colors::FG_PRIMARY));

                    f.render_widget(list, chunks[2]);
                }
            }
            crate::input::InputType::Warning { message, .. } => {
                // Render warning message with proper formatting
                let warning_text = message.join("\n");
                let warning_widget = Paragraph::new(warning_text)
                    .block(Block::default().borders(Borders::ALL).title("⚠️  WARNING"))
                    .style(Style::default().fg(Colors::ERROR))
                    .alignment(Alignment::Center)
                    .wrap(ratatui::widgets::Wrap { trim: true });
                f.render_widget(warning_widget, chunks[2]);
            }
            crate::input::InputType::PasswordInput { .. } => {
                let input_text = dialog.get_display_value();
                let input_display = if input_text.is_empty() {
                    "Enter password...".to_string()
                } else {
                    input_text
                };

                let input_widget = Paragraph::new(input_display)
                    .block(Block::default().borders(Borders::ALL).title("Password"))
                    .style(Style::default().fg(Colors::SUCCESS));
                f.render_widget(input_widget, chunks[2]);
            }
            crate::input::InputType::MultiDiskSelection {
                selected_disks,
                available_disks,
                scroll_state,
                min_disks,
                max_disks,
                ..
            } => {
                // Create list items with selection status
                let items: Vec<ListItem> = available_disks
                    .iter()
                    .enumerate()
                    .map(|(i, disk)| {
                        let is_selected = selected_disks.contains(disk);
                        let status = if is_selected { "[X]" } else { "[ ]" };
                        let item_text = format!("{} {}", status, disk);

                        ListItem::new(item_text).style(if i == scroll_state.selected_index {
                            Style::default().fg(Colors::SECONDARY).bg(Colors::FG_MUTED)
                        } else if is_selected {
                            Style::default().fg(Colors::SUCCESS)
                        } else {
                            Style::default().fg(Colors::FG_PRIMARY)
                        })
                    })
                    .collect();

                let list = List::new(items)
                    .block(Block::default().borders(Borders::ALL).title(format!(
                        "Selected: {}/{} (Min: {}, Max: {})",
                        selected_disks.len(),
                        max_disks,
                        min_disks,
                        max_disks
                    )))
                    .highlight_style(Style::default().fg(Colors::SECONDARY).bg(Colors::FG_MUTED));

                f.render_widget(list, chunks[2]);
            }
        }

        // Status/buttons
        let status = Paragraph::new("Enter: Confirm | Esc: Cancel")
            .block(Block::default().borders(Borders::NONE))
            .alignment(Alignment::Center)
            .style(Style::default().fg(Colors::PRIMARY));
        f.render_widget(status, chunks[3]);
    }
}
