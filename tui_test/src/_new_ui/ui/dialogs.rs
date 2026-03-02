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
        let path_display = format!(" {} ", browser.current_dir);
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
