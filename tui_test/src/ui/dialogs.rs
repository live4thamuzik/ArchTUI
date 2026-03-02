//! Dialog rendering (adapted for tui_test)
//!
//! ToolDialog, FloatingOutput, ConfirmDialog, FileBrowser overlays.

use crate::app::{AppState, ToolParameter};
use crate::components::confirm_dialog::ConfirmDialog;
use crate::components::floating_window::{FloatingWindow, FloatingWindowConfig};
use crate::theme::Colors;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Clear, List, ListItem, Paragraph, Scrollbar,
        ScrollbarOrientation, ScrollbarState, Wrap,
    },
    Frame,
};

/// Convert snake_case to Title Case
fn snake_to_title_case(s: &str) -> String {
    s.split('_')
        .filter(|w| !w.is_empty())
        .map(|w| {
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

/// Render tool parameter dialog
pub fn render_tool_dialog(f: &mut Frame, state: &AppState) {
    if let Some(ref dialog) = state.tool_dialog {
        let area = f.area();

        let dialog_width = (area.width * 3 / 4)
            .clamp(50, 80)
            .min(area.width.saturating_sub(2));
        let param_count = dialog.parameters.len() as u16;
        // Use more height when disk layout is present
        let layout_bonus = if !state.disk_layout.is_empty() {
            (state.disk_layout.len() as u16).min(12)
        } else {
            0
        };
        let dialog_height = (param_count + 9 + layout_bonus).min(area.height.saturating_sub(4));
        let dialog_x = area.width.saturating_sub(dialog_width) / 2;
        let dialog_y = area.height.saturating_sub(dialog_height) / 2;
        let dialog_rect = Rect::new(dialog_x, dialog_y, dialog_width, dialog_height);

        f.render_widget(Clear, dialog_rect);

        let title = format!(" {} ", snake_to_title_case(&dialog.tool_name));
        let pos = format!(
            " {}/{} ",
            dialog.current_param + 1,
            dialog.parameters.len()
        );
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Colors::BORDER_ACTIVE))
            .title(Line::from(vec![
                Span::styled("\u{2500}", Style::default().fg(Colors::BORDER_ACTIVE)),
                Span::styled(
                    title,
                    Style::default()
                        .fg(Colors::PRIMARY)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("\u{2500}", Style::default().fg(Colors::BORDER_ACTIVE)),
            ]))
            .title_bottom(
                Line::from(vec![Span::styled(
                    pos,
                    Style::default().fg(Colors::FG_MUTED),
                )])
                .alignment(Alignment::Right),
            )
            .style(Style::default().bg(Colors::BG_PRIMARY));

        let inner = block.inner(dialog_rect);
        f.render_widget(block, dialog_rect);

        // Check if disk layout is present to allocate more space
        let is_device_param = dialog.current_param < dialog.parameters.len()
            && matches!(
                dialog.parameters[dialog.current_param].name.as_str(),
                "device" | "disk" | "target"
            );
        let has_layout = is_device_param && !state.disk_layout.is_empty();

        let desc_constraint = if has_layout {
            Constraint::Min(6) // More room for disk layout
        } else {
            Constraint::Length(4)
        };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(3),    // Parameters
                desc_constraint,       // Description / Disk Layout
                Constraint::Length(1), // Instructions
            ])
            .split(inner);

        // Calculate label width for alignment
        let label_width = dialog
            .parameters
            .iter()
            .map(|p| {
                let label = snake_to_title_case(&p.name);
                2 + label.len() + if p.required { 2 } else { 0 } + 3
            })
            .max()
            .unwrap_or(10);

        let w = chunks[0].width as usize;
        let mut param_items = Vec::new();
        for (i, param) in dialog.parameters.iter().enumerate() {
            let is_selected = i == dialog.current_param;

            let raw_value = if i < dialog.param_values.len() {
                &dialog.param_values[i]
            } else {
                ""
            };

            let display_value = match &param.param_type {
                ToolParameter::Password(_) => {
                    let masked = "\u{2022}".repeat(raw_value.len());
                    if is_selected {
                        format!("{}\u{2588}", masked)
                    } else {
                        masked
                    }
                }
                ToolParameter::Selection(options, _) => {
                    let current_val = if raw_value.is_empty() {
                        options.first().map(|s| s.as_str()).unwrap_or("")
                    } else {
                        raw_value
                    };
                    if is_selected {
                        format!("\u{25c0} {} \u{25b6}", current_val)
                    } else {
                        current_val.to_string()
                    }
                }
                ToolParameter::Boolean(_) => {
                    let val = raw_value == "true";
                    if is_selected {
                        if val {
                            "\u{25c0} Yes \u{25b6}".to_string()
                        } else {
                            "\u{25c0} No \u{25b6}".to_string()
                        }
                    } else if val {
                        "Yes".to_string()
                    } else {
                        "No".to_string()
                    }
                }
                _ => {
                    if is_selected {
                        format!("{}\u{2588}", raw_value)
                    } else {
                        raw_value.to_string()
                    }
                }
            };

            let indicator = if is_selected { "\u{25b8} " } else { "  " };
            let label_text = snake_to_title_case(&param.name);

            // Build label spans separately to style the required marker
            let label_prefix = format!("{}{}", indicator, label_text);
            let padded_label = if param.required {
                // pad accounting for the 2-char " *"
                let base = format!("{}:", label_prefix);
                format!("{:<width$}", base, width = label_width.saturating_sub(2))
            } else {
                let base = format!("{}:", label_prefix);
                format!("{:<width$}", base, width = label_width)
            };

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

            let mut spans = vec![Span::styled(padded_label, label_style)];
            if param.required {
                spans.push(Span::styled(
                    " * ",
                    Style::default()
                        .fg(Colors::WARNING)
                        .add_modifier(Modifier::BOLD),
                ));
            }

            // Full-width highlight bar when selected
            if is_selected {
                let value_text = display_value.to_string();
                // Pad the value to fill remaining width
                let used: usize = spans.iter().map(|s| s.content.len()).sum();
                let remaining = w.saturating_sub(used + value_text.len());
                spans.push(Span::styled(value_text, value_style));
                if remaining > 0 {
                    spans.push(Span::styled(" ".repeat(remaining), value_style));
                }
                param_items.push(
                    ListItem::new(Line::from(spans))
                        .style(Style::default().bg(Colors::BG_SECONDARY)),
                );
            } else {
                spans.push(Span::styled(display_value, value_style));
                param_items.push(ListItem::new(Line::from(spans)));
            }
        }

        let param_list = List::new(param_items);
        f.render_widget(param_list, chunks[0]);

        // Description area — shows param description + disk layout when applicable
        let desc_text = if dialog.current_param < dialog.parameters.len() {
            dialog.parameters[dialog.current_param].description.clone()
        } else {
            String::new()
        };

        let desc_title = if has_layout { " Disk Layout " } else { " Description " };

        let desc_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(if has_layout {
                Colors::BORDER_ACTIVE
            } else {
                Colors::BORDER_INACTIVE
            }))
            .title(Line::from(vec![
                Span::styled("\u{2500}", Style::default().fg(if has_layout {
                    Colors::BORDER_ACTIVE
                } else {
                    Colors::BORDER_INACTIVE
                })),
                Span::styled(
                    desc_title,
                    Style::default().fg(if has_layout {
                        Colors::PRIMARY
                    } else {
                        Colors::FG_MUTED
                    }),
                ),
                Span::styled("\u{2500}", Style::default().fg(if has_layout {
                    Colors::BORDER_ACTIVE
                } else {
                    Colors::BORDER_INACTIVE
                })),
            ]))
            .style(Style::default().bg(Colors::BG_PRIMARY));

        if has_layout {
            // Show description + disk layout
            let mut lines: Vec<Line<'_>> = vec![
                Line::from(Span::styled(
                    format!(" {}", desc_text),
                    Style::default().fg(Colors::FG_SECONDARY),
                )),
                Line::from(""),
            ];
            for layout_line in &state.disk_layout {
                let style = if layout_line.contains("NAME") || layout_line.starts_with("  ─") {
                    Style::default().fg(Colors::FG_MUTED)
                } else if layout_line.contains("Disk model:") || layout_line.contains("Disklabel type:") {
                    Style::default().fg(Colors::SECONDARY)
                } else {
                    Style::default().fg(Colors::FG_PRIMARY)
                };
                lines.push(Line::from(Span::styled(layout_line.clone(), style)));
            }
            let desc = Paragraph::new(lines).block(desc_block);
            f.render_widget(desc, chunks[1]);
        } else {
            let desc = Paragraph::new(format!(" {}", desc_text))
                .block(desc_block)
                .style(Style::default().fg(Colors::FG_SECONDARY))
                .wrap(Wrap { trim: true });
            f.render_widget(desc, chunks[1]);
        }

        // Instructions: show Next or Execute based on current position
        let on_last = dialog.current_param >= dialog.parameters.len().saturating_sub(1);
        let enter_label = if on_last { " Execute  " } else { " Next  " };
        let instructions = Line::from(vec![
            Span::styled("Enter", Style::default().fg(Colors::SECONDARY)),
            Span::styled(enter_label, Style::default().fg(Colors::FG_MUTED)),
            Span::styled(
                "\u{2191}\u{2193}",
                Style::default().fg(Colors::SECONDARY),
            ),
            Span::styled(" Navigate  ", Style::default().fg(Colors::FG_MUTED)),
            Span::styled(
                "\u{2190}\u{2192}",
                Style::default().fg(Colors::SECONDARY),
            ),
            Span::styled(" Change  ", Style::default().fg(Colors::FG_MUTED)),
            Span::styled("Esc", Style::default().fg(Colors::SECONDARY)),
            Span::styled(" Back", Style::default().fg(Colors::FG_MUTED)),
        ]);
        f.render_widget(
            Paragraph::new(instructions).alignment(Alignment::Center),
            chunks[2],
        );
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

/// Render file browser
pub fn render_file_browser(f: &mut Frame, state: &AppState) {
    if let Some(ref browser) = state.file_browser {
        let area = f.area();
        let width = (area.width * 80 / 100).min(100);
        let height = (area.height * 80 / 100).min(30);
        let x = (area.width.saturating_sub(width)) / 2;
        let y = (area.height.saturating_sub(height)) / 2;
        let browser_area = Rect::new(x, y, width, height);

        f.render_widget(Clear, browser_area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(5),
                Constraint::Length(3),
            ])
            .split(browser_area);

        // Path display with embedded title
        let path_display = format!(" {} ", browser.current_dir.display());
        let path_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title(Line::from(vec![
                Span::styled("\u{2500}", Style::default().fg(Colors::BORDER_ACTIVE)),
                Span::styled(
                    " Select Configuration File ",
                    Style::default()
                        .fg(Colors::PRIMARY)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("\u{2500}", Style::default().fg(Colors::BORDER_ACTIVE)),
            ]))
            .border_style(Style::default().fg(Colors::BORDER_ACTIVE))
            .style(Style::default().bg(Colors::BG_PRIMARY));
        let path_paragraph = Paragraph::new(path_display)
            .style(Style::default().fg(Colors::SECONDARY))
            .block(path_block);
        f.render_widget(path_paragraph, chunks[0]);

        // File list
        let visible_height = chunks[1].height.saturating_sub(2) as usize;
        let items: Vec<ListItem> = browser
            .entries
            .iter()
            .enumerate()
            .skip(browser.scroll_offset)
            .take(visible_height)
            .map(|(i, entry)| {
                let (icon, color) = if entry.is_dir {
                    ("\u{1f4c1}", Colors::INFO)
                } else if entry.name.ends_with(".toml") {
                    ("\u{1f4c4}", Colors::SUCCESS)
                } else if entry.name.ends_with(".json") {
                    ("\u{1f4c4}", Colors::SECONDARY)
                } else {
                    ("\u{1f4c4}", Colors::FG_PRIMARY)
                };

                let size_str = if entry.is_dir {
                    String::new()
                } else {
                    format_size(entry.size)
                };

                let style = if i == browser.selected {
                    Style::default()
                        .fg(Colors::SELECTED_FG)
                        .bg(Colors::PRIMARY)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(color)
                };

                let line = Line::from(vec![
                    Span::styled(format!(" {} ", icon), style),
                    Span::styled(format!("{:<40}", entry.name), style),
                    Span::styled(size_str, style),
                ]);
                ListItem::new(line)
            })
            .collect();

        let list_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Colors::BORDER_INACTIVE));
        let list = List::new(items).block(list_block);
        f.render_widget(list, chunks[1]);

        // Scrollbar on file list
        let total_entries = browser.entries.len();
        if total_entries > visible_height {
            let mut scrollbar_state =
                ScrollbarState::new(total_entries).position(browser.selected);
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(None)
                .end_symbol(None)
                .track_symbol(Some("\u{2502}"))
                .thumb_symbol("\u{2588}")
                .track_style(Style::default().fg(Colors::SCROLLBAR_TRACK))
                .thumb_style(Style::default().fg(Colors::SCROLLBAR_THUMB));
            f.render_stateful_widget(scrollbar, chunks[1], &mut scrollbar_state);
        }

        // Help text
        let help_text = if let Some(err) = &browser.error {
            err.clone()
        } else {
            "\u{2191}\u{2193} Navigate | Enter Select | ~ Home | / Root | Esc Cancel".to_string()
        };
        let help_style = if browser.error.is_some() {
            Style::default().fg(Colors::ERROR)
        } else {
            Style::default().fg(Colors::FG_MUTED)
        };
        let help_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Colors::BORDER_INACTIVE));
        let help_paragraph = Paragraph::new(help_text)
            .style(help_style)
            .block(help_block);
        f.render_widget(help_paragraph, chunks[2]);
    }
}

/// Render confirmation dialog
pub fn render_confirm_dialog(f: &mut Frame, state: &AppState) {
    if let Some(ref dialog_state) = state.confirm_dialog {
        ConfirmDialog::render(f, dialog_state);
    }
}

/// Render embedded terminal (PTY)
pub fn render_embedded_terminal(
    f: &mut Frame,
    _state: &AppState,
    area: Rect,
    pty_terminal: Option<&mut crate::components::pty_terminal::PtyTerminal>,
) {
    if let Some(pty) = pty_terminal {
        pty.render(f, area, "Terminal");
    } else {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title(" Terminal ")
            .border_style(Style::default().fg(Colors::BORDER_ACTIVE))
            .style(Style::default().bg(Colors::BG_PRIMARY));
        let msg = Paragraph::new("No terminal session active")
            .style(Style::default().fg(Colors::FG_MUTED))
            .block(block)
            .alignment(Alignment::Center);
        f.render_widget(msg, area);
    }
}

/// Render input dialog (from InputHandler popup)
///
/// This renders the real app's InputHandler dialog system (text input,
/// selection, disk selection, package selection, password, warning, multi-disk).
pub fn render_input_dialog(f: &mut Frame, input_handler: &mut crate::input::InputHandler) {
    if let Some(ref mut dialog) = input_handler.current_dialog {
        let area = f.area();
        let dialog_width = (area.width * 85 / 100).clamp(40, 120);
        let dialog_height = (area.height * 80 / 100).clamp(10, 35);
        let x = area.width.saturating_sub(dialog_width) / 2;
        let y = area.height.saturating_sub(dialog_height) / 2;
        let dialog_area = Rect::new(x, y, dialog_width, dialog_height);

        f.render_widget(Clear, dialog_area);

        let title = format!(" {} ", dialog.title);
        let outer_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
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

        let inner = Rect::new(
            dialog_area.x.saturating_add(3),
            dialog_area.y.saturating_add(2),
            dialog_area.width.saturating_sub(6),
            dialog_area.height.saturating_sub(4),
        );

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),
                Constraint::Length(1),
                Constraint::Length(1),
            ])
            .split(inner);

        let content_area = chunks[0];
        let selected_index = dialog.input_type.get_selected_index();

        match &mut dialog.input_type {
            crate::input::InputType::Selection { scroll_state, options, .. } => {
                let (start, end) = scroll_state.visible_range();
                let items: Vec<ListItem> = options.iter().enumerate()
                    .skip(start).take(end - start)
                    .map(|(index, option)| {
                        let (indicator, style) = if index == selected_index {
                            ("\u{25b8} ", Style::default().fg(Colors::SECONDARY).add_modifier(Modifier::BOLD))
                        } else {
                            ("  ", Style::default().fg(Colors::FG_PRIMARY))
                        };
                        ListItem::new(Line::from(vec![
                            Span::styled(indicator, style),
                            Span::styled(option.clone(), style),
                        ]))
                    })
                    .collect();
                let list = List::new(items).style(Style::default().bg(Colors::BG_PRIMARY));
                f.render_widget(list, content_area);
            }
            crate::input::InputType::DiskSelection { available_disks, .. } => {
                let items: Vec<ListItem> = available_disks.iter().enumerate()
                    .map(|(index, disk)| {
                        let (indicator, style) = if index == selected_index {
                            ("\u{25b8} ", Style::default().fg(Colors::SECONDARY).add_modifier(Modifier::BOLD))
                        } else {
                            ("  ", Style::default().fg(Colors::FG_PRIMARY))
                        };
                        ListItem::new(Line::from(vec![
                            Span::styled(indicator, style),
                            Span::styled(disk.clone(), style),
                        ]))
                    })
                    .collect();
                let list = List::new(items).style(Style::default().bg(Colors::BG_PRIMARY));
                f.render_widget(list, content_area);
            }
            crate::input::InputType::PackageSelection {
                current_input, output_lines, scroll_offset,
                package_list, show_search_results, search_results,
                list_state, is_pacman, ..
            } => {
                if *show_search_results && !search_results.is_empty() {
                    let package_items: Vec<ListItem> = search_results.iter()
                        .map(|p| {
                            let is_selected = package_list.split_whitespace().any(|pkg| pkg == p.name);
                            let status = if is_selected { "[\u{2713}]" } else if p.installed { "[I]" } else { "[ ]" };
                            let text = format!("{} {}/{} ({}) - {}", status, p.repo, p.name, p.version, p.description);
                            let style = if is_selected {
                                Style::default().fg(Colors::SUCCESS).add_modifier(Modifier::BOLD)
                            } else {
                                Style::default().fg(Colors::FG_PRIMARY)
                            };
                            ListItem::new(text).style(style)
                        })
                        .collect();
                    let search_hint = Line::from(Span::styled(
                        " \u{2191}\u{2193} Navigate | Enter: Toggle | Esc: Back ",
                        Style::default().fg(Colors::FG_MUTED),
                    ));
                    let results_block = Block::default()
                        .borders(Borders::TOP)
                        .border_style(Style::default().fg(Colors::FG_MUTED))
                        .title_top(search_hint)
                        .style(Style::default().bg(Colors::BG_PRIMARY));
                    let search_list = List::new(package_items)
                        .block(results_block)
                        .highlight_style(Style::default().fg(Colors::SUCCESS_LIGHT).add_modifier(Modifier::BOLD))
                        .highlight_symbol(">> ");
                    f.render_stateful_widget(search_list, content_area, list_state);
                } else {
                    let cmd_chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([Constraint::Length(2), Constraint::Min(0)])
                        .split(content_area);
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
                    f.render_widget(Paragraph::new(help_line).block(help_block), cmd_chunks[0]);
                    let output_area = cmd_chunks[1];
                    let visible_height = output_area.height as usize;
                    let mut list_items: Vec<ListItem> = output_lines.iter()
                        .skip(*scroll_offset)
                        .take(visible_height.saturating_sub(1))
                        .map(|line| ListItem::new(line.as_str()).style(Style::default().fg(Colors::FG_PRIMARY)))
                        .collect();
                    let prompt = if *is_pacman { "Package selection> " } else { "AUR package selection> " };
                    let input_line = Line::from(vec![
                        Span::styled(prompt, Style::default().fg(Colors::SECONDARY)),
                        Span::styled(format!("{}_", current_input), Style::default().fg(Colors::FG_PRIMARY)),
                    ]);
                    list_items.push(ListItem::new(input_line));
                    f.render_widget(List::new(list_items).style(Style::default().bg(Colors::BG_PRIMARY)), output_area);
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
            crate::input::InputType::MultiDiskSelection {
                selected_disks, available_disks, scroll_state, min_disks, max_disks, ..
            } => {
                let counter_text = format!(" Selected: {}/{} (Min: {}, Max: {}) ", selected_disks.len(), max_disks, min_disks, max_disks);
                let counter_block = Block::default()
                    .borders(Borders::BOTTOM)
                    .border_style(Style::default().fg(Colors::FG_MUTED))
                    .title_top(Span::styled(counter_text, Style::default().fg(Colors::PRIMARY).add_modifier(Modifier::BOLD)))
                    .style(Style::default().bg(Colors::BG_PRIMARY));
                let multi_chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Length(2), Constraint::Min(0)])
                    .split(content_area);
                f.render_widget(counter_block, multi_chunks[0]);
                let items: Vec<ListItem> = available_disks.iter().enumerate()
                    .map(|(i, disk)| {
                        let is_selected = selected_disks.contains(disk);
                        let status = if is_selected { "[\u{2713}]" } else { "[ ]" };
                        let item_text = format!("{} {}", status, disk);
                        let style = if i == scroll_state.selected_index {
                            Style::default().fg(Colors::SECONDARY).add_modifier(Modifier::BOLD)
                        } else if is_selected {
                            Style::default().fg(Colors::SUCCESS)
                        } else {
                            Style::default().fg(Colors::FG_PRIMARY)
                        };
                        ListItem::new(item_text).style(style)
                    })
                    .collect();
                f.render_widget(List::new(items).style(Style::default().bg(Colors::BG_PRIMARY)), multi_chunks[1]);
            }
        }

        f.render_widget(
            Paragraph::new("Enter: Confirm | Esc: Cancel")
                .style(Style::default().fg(Colors::FG_MUTED))
                .alignment(Alignment::Center),
            chunks[2],
        );
    }
}

fn format_size(size: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if size >= GB {
        format!("{:.1} GB", size as f64 / GB as f64)
    } else if size >= MB {
        format!("{:.1} MB", size as f64 / MB as f64)
    } else if size >= KB {
        format!("{:.1} KB", size as f64 / KB as f64)
    } else {
        format!("{} B", size)
    }
}
