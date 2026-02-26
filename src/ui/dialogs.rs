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
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
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
        let mut window = FloatingWindow::new(config);
        window.set_scroll_offset(output.scroll_offset);

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

/// Convert snake_case tool name to Title Case display name
/// Recognizes common acronyms: IP, EFI, SSH, URL, JSON, DNS, UUID, AUR, USB
fn snake_to_title_case(s: &str) -> String {
    s.split('_')
        .filter(|w| !w.is_empty())
        .map(|w| {
            // Check for known acronyms (case-insensitive match)
            match w.to_ascii_lowercase().as_str() {
                "ip" => "IP".to_string(),
                "efi" => "EFI".to_string(),
                "ssh" => "SSH".to_string(),
                "url" => "URL".to_string(),
                "json" => "JSON".to_string(),
                "dns" => "DNS".to_string(),
                "uuid" => "UUID".to_string(),
                "aur" => "AUR".to_string(),
                "usb" => "USB".to_string(),
                _ => {
                    let mut chars = w.chars();
                    match chars.next() {
                        Some(c) => {
                            let upper: String = c.to_uppercase().collect();
                            format!("{}{}", upper, chars.as_str())
                        }
                        None => String::new(),
                    }
                }
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Render tool parameter dialog as a floating overlay
pub fn render_tool_dialog(f: &mut Frame, state: &AppState) {
    if let Some(ref dialog) = state.tool_dialog {
        let area = f.area();

        // Centered dialog box — same sizing approach as FloatingWindow
        let dialog_width = (area.width * 3 / 4).min(80);
        let param_count = dialog.parameters.len() as u16;
        // Height: 2 border + params + 1 separator + 3 desc + 1 instructions
        let dialog_height = (param_count + 7).min(area.height.saturating_sub(4));
        let dialog_x = area.width.saturating_sub(dialog_width) / 2;
        let dialog_y = area.height.saturating_sub(dialog_height) / 2;

        let dialog_rect = Rect::new(dialog_x, dialog_y, dialog_width, dialog_height);

        // Clear whatever is underneath
        f.render_widget(Clear, dialog_rect);

        // Title: "Configure Network" instead of "Configure configure_network"
        let title = format!(" {} ", snake_to_title_case(&dialog.tool_name));

        // Outer block — matches FloatingWindow style: cyan borders, dark bg
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Colors::PRIMARY))
            .title(Span::styled(
                title,
                Style::default()
                    .fg(Colors::PRIMARY)
                    .add_modifier(Modifier::BOLD),
            ))
            .style(Style::default().bg(Colors::BG_PRIMARY));
        f.render_widget(block, dialog_rect);

        // Inner area (inside borders)
        let inner = Rect::new(
            dialog_rect.x + 1,
            dialog_rect.y + 1,
            dialog_rect.width.saturating_sub(2),
            dialog_rect.height.saturating_sub(2),
        );

        // 3-part vertical layout: params | separator+description | instructions
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(3),    // Parameter list
                Constraint::Length(3), // Description area
                Constraint::Length(1), // Instructions
            ])
            .split(inner);

        // --- Parameter list ---
        // Find the longest label to align values
        let label_width = dialog.parameters.iter().map(|p| {
            let label = snake_to_title_case(&p.name);
            // "▸ " (2) + label + " *" (2 if required, 0 otherwise) + ":  " (3)
            2 + label.len() + if p.required { 2 } else { 0 } + 3
        }).max().unwrap_or(10);

        let mut param_items = Vec::new();
        for (i, param) in dialog.parameters.iter().enumerate() {
            let is_selected = i == dialog.current_param;

            let raw_value = if i < dialog.param_values.len() {
                &dialog.param_values[i]
            } else {
                ""
            };

            // Format display value based on parameter type
            let display_value = match &param.param_type {
                ToolParameter::Password(_) => {
                    let masked = "*".repeat(raw_value.len());
                    if is_selected {
                        format!("{}_", masked)
                    } else {
                        masked
                    }
                }
                ToolParameter::Selection(options, _) => {
                    let current_val = if raw_value.is_empty() {
                        // SAFETY: options guaranteed non-empty by get_tool_parameters
                        options.first().map(|s| s.as_str()).unwrap_or("")
                    } else {
                        raw_value
                    };
                    if is_selected {
                        format!("< {} >", current_val)
                    } else {
                        current_val.to_string()
                    }
                }
                _ => {
                    if is_selected {
                        format!("{}_", raw_value)
                    } else {
                        raw_value.to_string()
                    }
                }
            };

            // Build label: "▸ Config Type *:  " or "  Config Type:   "
            let indicator = if is_selected { "▸ " } else { "  " };
            let label_text = snake_to_title_case(&param.name);
            let required_marker = if param.required { " *" } else { "" };
            let label = format!("{}{}{}:", indicator, label_text, required_marker);
            // Pad label to align values
            let padded_label = format!("{:<width$}", label, width = label_width);

            let label_style = if is_selected {
                Style::default()
                    .fg(Colors::PRIMARY)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Colors::FG_SECONDARY)
            };

            let value_style = if is_selected {
                Style::default().fg(Colors::SECONDARY)
            } else {
                Style::default().fg(Colors::FG_PRIMARY)
            };

            param_items.push(ListItem::new(Line::from(vec![
                Span::styled(padded_label, label_style),
                Span::styled(display_value, value_style),
            ])));
        }

        let param_list = List::new(param_items);
        f.render_widget(param_list, chunks[0]);

        // --- Description area (separator + selected param description) ---
        let desc_text = if dialog.current_param < dialog.parameters.len() {
            dialog.parameters[dialog.current_param].description.clone()
        } else {
            String::new()
        };

        let desc_block = Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(Colors::FG_MUTED));
        let desc = Paragraph::new(desc_text)
            .block(desc_block)
            .style(Style::default().fg(Colors::FG_SECONDARY).bg(Colors::BG_PRIMARY))
            .wrap(Wrap { trim: true });
        f.render_widget(desc, chunks[1]);

        // --- Instructions ---
        f.render_widget(
            Paragraph::new("Enter: Execute | ↑↓: Navigate | ←→: Change | Esc: Back")
                .style(Style::default().fg(Colors::FG_MUTED))
                .alignment(Alignment::Center),
            chunks[2],
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

        // Dynamic dialog sizing — use available space, cap at reasonable maximums
        let dialog_width = (area.width * 85 / 100).clamp(40, 120);
        let dialog_height = (area.height * 80 / 100).clamp(10, 35);
        let x = area.width.saturating_sub(dialog_width) / 2;
        let y = area.height.saturating_sub(dialog_height) / 2;

        let dialog_area = Rect::new(x, y, dialog_width, dialog_height);

        // Clear whatever is underneath (overlay, not full-screen fill)
        f.render_widget(Clear, dialog_area);

        // Outer block — matches ToolDialog/FloatingWindow: cyan border, dark bg, bold title
        let title = format!(" {} ", dialog.title);
        let outer_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Colors::PRIMARY))
            .title(Span::styled(
                title,
                Style::default()
                    .fg(Colors::PRIMARY)
                    .add_modifier(Modifier::BOLD),
            ))
            .title_bottom(Line::from(Span::styled(
                format!(" {} ", dialog.instructions),
                Style::default().fg(Colors::FG_MUTED),
            )))
            .style(Style::default().bg(Colors::BG_PRIMARY));
        f.render_widget(outer_block, dialog_area);

        // Inner area (inside borders + padding: 1 top, 2 horizontal)
        let inner = Rect::new(
            dialog_area.x.saturating_add(3),
            dialog_area.y.saturating_add(2),
            dialog_area.width.saturating_sub(6),
            dialog_area.height.saturating_sub(4),
        );

        // 3-chunk layout: Content + spacer + Status bar
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),    // Content
                Constraint::Length(1), // Spacer
                Constraint::Length(1), // Status bar
            ])
            .split(inner);

        let content_area = chunks[0];

        // Content based on input type
        let selected_index = dialog.input_type.get_selected_index();
        match &mut dialog.input_type {
            crate::input::InputType::TextInput { .. } => {
                let input_text = dialog.get_display_value();
                let display_text = if input_text.is_empty() {
                    Span::styled("Enter value..._", Style::default().fg(Colors::FG_MUTED))
                } else {
                    Span::styled(
                        format!("{}_", input_text),
                        Style::default().fg(Colors::FG_PRIMARY),
                    )
                };

                let input_widget = Paragraph::new(Line::from(display_text))
                    .style(Style::default().bg(Colors::BG_PRIMARY));
                f.render_widget(input_widget, content_area);
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
                        let (indicator, style) = if index == selected_index {
                            (
                                "▸ ",
                                Style::default()
                                    .fg(Colors::SECONDARY)
                                    .add_modifier(Modifier::BOLD),
                            )
                        } else {
                            ("  ", Style::default().fg(Colors::FG_PRIMARY))
                        };
                        ListItem::new(Line::from(vec![
                            Span::styled(indicator, style),
                            Span::styled(option.clone(), style),
                        ]))
                    })
                    .collect();

                let list = List::new(items)
                    .style(Style::default().bg(Colors::BG_PRIMARY));
                f.render_widget(list, content_area);
            }
            crate::input::InputType::DiskSelection {
                available_disks, ..
            } => {
                let items: Vec<ListItem> = available_disks
                    .iter()
                    .enumerate()
                    .map(|(index, disk)| {
                        let (indicator, style) = if index == selected_index {
                            (
                                "▸ ",
                                Style::default()
                                    .fg(Colors::SECONDARY)
                                    .add_modifier(Modifier::BOLD),
                            )
                        } else {
                            ("  ", Style::default().fg(Colors::FG_PRIMARY))
                        };
                        ListItem::new(Line::from(vec![
                            Span::styled(indicator, style),
                            Span::styled(disk.clone(), style),
                        ]))
                    })
                    .collect();

                let list = List::new(items)
                    .style(Style::default().bg(Colors::BG_PRIMARY));
                f.render_widget(list, content_area);
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
                if *show_search_results && !search_results.is_empty() {
                    // Display search results with scrolling
                    let package_items: Vec<ListItem> = search_results
                        .iter()
                        .map(|p| {
                            // Check if this package is already selected — exact word match
                            let is_selected =
                                package_list.split_whitespace().any(|pkg| pkg == p.name);

                            // Single bracket indicator: [✓] selected, [I] installed, [ ] neither
                            let status = if is_selected {
                                "[✓]"
                            } else if p.installed {
                                "[I]"
                            } else {
                                "[ ]"
                            };

                            let text = format!(
                                "{} {}/{} ({}) - {}",
                                status,
                                p.repo,
                                p.name,
                                p.version,
                                p.description
                            );

                            let style = if is_selected {
                                Style::default()
                                    .fg(Colors::SUCCESS)
                                    .add_modifier(Modifier::BOLD)
                            } else {
                                Style::default().fg(Colors::FG_PRIMARY)
                            };

                            ListItem::new(text).style(style)
                        })
                        .collect();

                    let search_hint = Line::from(Span::styled(
                        " ↑↓ Navigate | Enter: Toggle | Esc: Back ",
                        Style::default().fg(Colors::FG_MUTED),
                    ));

                    // Inner block for search results — no double border, just a separator hint
                    let results_block = Block::default()
                        .borders(Borders::TOP)
                        .border_style(Style::default().fg(Colors::FG_MUTED))
                        .title_top(search_hint)
                        .style(Style::default().bg(Colors::BG_PRIMARY));

                    let search_list = List::new(package_items)
                        .block(results_block)
                        .highlight_style(
                            Style::default()
                                .fg(Colors::SUCCESS_LIGHT)
                                .add_modifier(Modifier::BOLD),
                        )
                        .highlight_symbol(">> ");

                    f.render_stateful_widget(search_list, content_area, list_state);
                } else {
                    // Command mode: compact help bar at top, output+prompt below
                    let cmd_chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([
                            Constraint::Length(2), // Help bar + separator
                            Constraint::Min(0),   // Output + prompt
                        ])
                        .split(content_area);

                    // Compact command reference
                    let help_line = Line::from(vec![
                        Span::styled("search ", Style::default().fg(Colors::PRIMARY)),
                        Span::styled("<term>  ", Style::default().fg(Colors::FG_MUTED)),
                        Span::styled("add ", Style::default().fg(Colors::PRIMARY)),
                        Span::styled("<pkg>  ", Style::default().fg(Colors::FG_MUTED)),
                        Span::styled("remove ", Style::default().fg(Colors::PRIMARY)),
                        Span::styled("<pkg>  ", Style::default().fg(Colors::FG_MUTED)),
                        Span::styled("list  ", Style::default().fg(Colors::PRIMARY)),
                        Span::styled("done", Style::default().fg(Colors::PRIMARY)),
                    ]);
                    let help_block = Block::default()
                        .borders(Borders::BOTTOM)
                        .border_style(Style::default().fg(Colors::FG_MUTED))
                        .style(Style::default().bg(Colors::BG_PRIMARY));
                    let help_widget = Paragraph::new(help_line).block(help_block);
                    f.render_widget(help_widget, cmd_chunks[0]);

                    // Output lines + prompt
                    let output_area = cmd_chunks[1];
                    let visible_height = output_area.height as usize;
                    let mut list_items: Vec<ListItem> = output_lines
                        .iter()
                        .skip(*scroll_offset)
                        .take(visible_height.saturating_sub(1))
                        .map(|line| {
                            ListItem::new(line.as_str())
                                .style(Style::default().fg(Colors::FG_PRIMARY))
                        })
                        .collect();

                    // Add current input line with prompt
                    let prompt = if *is_pacman {
                        "Package selection> "
                    } else {
                        "AUR package selection> "
                    };
                    let input_line = Line::from(vec![
                        Span::styled(prompt, Style::default().fg(Colors::SECONDARY)),
                        Span::styled(
                            format!("{}_", current_input),
                            Style::default().fg(Colors::FG_PRIMARY),
                        ),
                    ]);
                    list_items.push(ListItem::new(input_line));

                    let list = List::new(list_items)
                        .style(Style::default().bg(Colors::BG_PRIMARY));

                    f.render_widget(list, output_area);
                }
            }
            crate::input::InputType::Warning { message, .. } => {
                let warning_text = message.join("\n");
                let warning_widget = Paragraph::new(warning_text)
                    .style(Style::default().fg(Colors::ERROR).bg(Colors::BG_PRIMARY))
                    .alignment(Alignment::Center)
                    .wrap(Wrap { trim: true });
                f.render_widget(warning_widget, content_area);
            }
            crate::input::InputType::PasswordInput { .. } => {
                let input_text = dialog.get_display_value();
                let display_text = if input_text.is_empty() {
                    Span::styled(
                        "Enter password..._",
                        Style::default().fg(Colors::FG_MUTED),
                    )
                } else {
                    Span::styled(
                        format!("{}_", input_text),
                        Style::default().fg(Colors::FG_PRIMARY),
                    )
                };

                let input_widget = Paragraph::new(Line::from(display_text))
                    .style(Style::default().bg(Colors::BG_PRIMARY));
                f.render_widget(input_widget, content_area);
            }
            crate::input::InputType::MultiDiskSelection {
                selected_disks,
                available_disks,
                scroll_state,
                min_disks,
                max_disks,
                ..
            } => {
                // Selection counter as a separator line at top
                let counter_text = format!(
                    " Selected: {}/{} (Min: {}, Max: {}) ",
                    selected_disks.len(),
                    max_disks,
                    min_disks,
                    max_disks
                );
                let counter_block = Block::default()
                    .borders(Borders::BOTTOM)
                    .border_style(Style::default().fg(Colors::FG_MUTED))
                    .title_top(Span::styled(
                        counter_text,
                        Style::default()
                            .fg(Colors::PRIMARY)
                            .add_modifier(Modifier::BOLD),
                    ))
                    .style(Style::default().bg(Colors::BG_PRIMARY));

                // Split content: 1 line for counter, rest for list
                let multi_chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Length(2), Constraint::Min(0)])
                    .split(content_area);

                f.render_widget(counter_block, multi_chunks[0]);

                let items: Vec<ListItem> = available_disks
                    .iter()
                    .enumerate()
                    .map(|(i, disk)| {
                        let is_selected = selected_disks.contains(disk);
                        let status = if is_selected { "[✓]" } else { "[ ]" };
                        let item_text = format!("{} {}", status, disk);

                        let style = if i == scroll_state.selected_index {
                            Style::default()
                                .fg(Colors::SECONDARY)
                                .add_modifier(Modifier::BOLD)
                        } else if is_selected {
                            Style::default().fg(Colors::SUCCESS)
                        } else {
                            Style::default().fg(Colors::FG_PRIMARY)
                        };

                        ListItem::new(item_text).style(style)
                    })
                    .collect();

                let list = List::new(items)
                    .style(Style::default().bg(Colors::BG_PRIMARY));
                f.render_widget(list, multi_chunks[1]);
            }
        }

        // Status bar — matches ToolDialog/FloatingWindow style
        f.render_widget(
            Paragraph::new("Enter: Confirm | Esc: Cancel")
                .style(Style::default().fg(Colors::FG_MUTED))
                .alignment(Alignment::Center),
            chunks[2],
        );
    }
}
