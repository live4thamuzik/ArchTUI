//! Installation UI rendering (adapted for tui_test)
//!
//! Redesigned: no header/title bars. Breadcrumb + split-pane layout.
//! Consistent with the new menus.rs design.

use super::header::{render_installer_output, render_progress_bar};
use super::menus::render_breadcrumb;
use crate::app::{AppState, ConfigEditState};
use crate::theme::Colors;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, List, ListItem, Paragraph, Scrollbar,
        ScrollbarOrientation, ScrollbarState, Wrap,
    },
    Frame,
};

/// Active panel block — rounded, teal border, embedded title
fn panel_active(title: &str) -> Block<'_> {
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(Line::from(vec![
            Span::styled("\u{2500}", Style::default().fg(Colors::BORDER_ACTIVE)),
            Span::styled(
                format!(" {} ", title),
                Style::default()
                    .fg(Colors::PRIMARY)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("\u{2500}", Style::default().fg(Colors::BORDER_ACTIVE)),
        ]))
        .border_style(Style::default().fg(Colors::BORDER_ACTIVE))
        .style(Style::default().bg(Colors::BG_PRIMARY))
}

/// Inactive panel block — rounded, dim border
fn panel_inactive(title: &str) -> Block<'_> {
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(Line::from(vec![
            Span::styled("\u{2500}", Style::default().fg(Colors::BORDER_INACTIVE)),
            Span::styled(
                format!(" {} ", title),
                Style::default()
                    .fg(Colors::FG_SECONDARY)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("\u{2500}", Style::default().fg(Colors::BORDER_INACTIVE)),
        ]))
        .border_style(Style::default().fg(Colors::BORDER_INACTIVE))
        .style(Style::default().bg(Colors::BG_PRIMARY))
}

// =============================================================================
// Guided Installer — Configuration UI
// =============================================================================

pub fn render_configuration_ui(f: &mut Frame, state: &AppState, area: Rect) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Breadcrumb
            Constraint::Min(1),   // Content
            Constraint::Length(3), // Buttons
        ])
        .split(area);

    render_breadcrumb(f, layout[0], &["Guided Installer"]);

    // Split pane: config options left (40%), details right (60%)
    let panes = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(layout[1]);

    render_config_options(f, panes[0], state);
    render_config_detail(f, panes[1], state);
    render_start_button(f, layout[2], state);
}

/// Left pane: config options list with full-width highlight
fn render_config_options(f: &mut Frame, area: Rect, state: &AppState) {
    let (start_idx, end_idx) = state.config_scroll.visible_range();
    let w = area.width.saturating_sub(2) as usize;

    let visible_items: Vec<ListItem> = state
        .config
        .options
        .iter()
        .enumerate()
        .skip(start_idx)
        .take(end_idx - start_idx)
        .map(|(index, option)| {
            let is_sel = index == state.config_scroll.selected_index;

            let display_value = if option.value.is_empty() {
                "[Press Enter]".to_string()
            } else {
                match option.name.as_str() {
                    "User Password" | "Root Password" | "Encryption Password" => {
                        if option.value == "N/A" {
                            "N/A".to_string()
                        } else {
                            "***".to_string()
                        }
                    }
                    _ => option.value.clone(),
                }
            };

            let text = format!("{}: {}", option.name, display_value);

            if is_sel {
                let display = format!(" \u{25b8} {}", text);
                let padded = format!("{:<width$}", display, width = w);
                ListItem::new(padded).style(
                    Style::default()
                        .fg(Colors::BG_PRIMARY)
                        .bg(Colors::PRIMARY)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                ListItem::new(format!("   {}", text))
                    .style(Style::default().fg(Colors::FG_PRIMARY))
            }
        })
        .collect();

    let pos = if let Some((current_page, total_pages)) = state.config_scroll.page_info() {
        format!("Page {}/{}", current_page, total_pages)
    } else {
        format!(
            "{}/{}",
            state.config_scroll.selected_index + 1,
            state.config.options.len()
        )
    };

    let block = panel_active("Configuration").title_bottom(
        Line::from(vec![Span::styled(
            format!(" {} ", pos),
            Style::default().fg(Colors::FG_MUTED),
        )])
        .alignment(Alignment::Right),
    );

    let list = List::new(visible_items).block(block);
    f.render_widget(list, area);

    // Scrollbar
    let total = state.config.options.len();
    if total > area.height.saturating_sub(2) as usize {
        let mut scrollbar_state =
            ScrollbarState::new(total).position(state.config_scroll.selected_index);
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None)
            .track_symbol(Some("\u{2502}"))
            .thumb_symbol("\u{2588}")
            .track_style(Style::default().fg(Colors::SCROLLBAR_TRACK))
            .thumb_style(Style::default().fg(Colors::SCROLLBAR_THUMB));
        f.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
    }
}

/// Right pane: detail view OR interactive edit depending on state
fn render_config_detail(f: &mut Frame, area: Rect, state: &AppState) {
    match &state.config_edit {
        ConfigEditState::None => render_detail_info(f, area, state),
        ConfigEditState::Selection { choices, selected } => {
            render_detail_selection(f, area, state, choices, *selected);
        }
        ConfigEditState::TextInput { value, cursor } => {
            render_detail_text_input(f, area, state, value, *cursor, false);
        }
        ConfigEditState::PasswordInput { value, cursor } => {
            render_detail_text_input(f, area, state, value, *cursor, true);
        }
        ConfigEditState::PackageInput {
            packages,
            current_input,
            output_lines,
            is_pacman,
            search_results,
            results_selected,
            show_search_results,
        } => {
            render_detail_package_input(
                f,
                area,
                packages,
                current_input,
                output_lines,
                *is_pacman,
                search_results,
                *results_selected,
                *show_search_results,
            );
        }
    }
}

/// Static info about the selected config option (default state)
fn render_detail_info(f: &mut Frame, area: Rect, state: &AppState) {
    let sel = state.config_scroll.selected_index;
    let option = state.config.options.get(sel);

    let info_lines = if let Some(opt) = option {
        let mut lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                format!("  {}", opt.name),
                Style::default()
                    .fg(Colors::SECONDARY)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
        ];

        if !opt.description.is_empty() {
            lines.push(Line::from(Span::styled(
                format!("  {}", opt.description),
                Style::default().fg(Colors::FG_PRIMARY),
            )));
            lines.push(Line::from(""));
        }

        if opt.required {
            lines.push(Line::from(vec![
                Span::styled("  \u{25cf} ", Style::default().fg(Colors::WARNING)),
                Span::styled("Required field", Style::default().fg(Colors::FG_SECONDARY)),
            ]));
        }

        if !opt.default_value.is_empty() {
            lines.push(Line::from(vec![
                Span::styled("  Default: ", Style::default().fg(Colors::FG_MUTED)),
                Span::styled(&opt.default_value, Style::default().fg(Colors::FG_SECONDARY)),
            ]));
        }

        if !opt.options.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "  Available options:",
                Style::default().fg(Colors::FG_MUTED),
            )));
            for choice in &opt.options {
                lines.push(Line::from(vec![
                    Span::styled("    \u{2022} ", Style::default().fg(Colors::PRIMARY)),
                    Span::styled(choice, Style::default().fg(Colors::FG_SECONDARY)),
                ]));
            }
        }

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  Press Enter to edit",
            Style::default().fg(Colors::FG_MUTED),
        )));

        lines
    } else {
        vec![
            Line::from(""),
            Line::from(Span::styled(
                "  Select a configuration option",
                Style::default().fg(Colors::FG_MUTED),
            )),
        ]
    };

    let info = Paragraph::new(info_lines)
        .block(panel_inactive("Option Details"))
        .wrap(Wrap { trim: false });
    f.render_widget(info, area);
}

/// Selection list with full-width highlight bar
fn render_detail_selection(
    f: &mut Frame,
    area: Rect,
    state: &AppState,
    choices: &[String],
    selected: usize,
) {
    let sel = state.config_scroll.selected_index;
    let opt = state.config.options.get(sel);
    let opt_name = opt.map(|o| o.name.as_str()).unwrap_or("Option");
    let show_layout = opt_name == "Disk" && !state.disk_layout.is_empty();

    // Split area: selection list top, disk layout bottom (when applicable)
    let (list_area, layout_area) = if show_layout {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
            .split(area);
        (chunks[0], Some(chunks[1]))
    } else {
        (area, None)
    };

    let w = list_area.width.saturating_sub(2) as usize;

    // Calculate visible window so long lists keep the selected item on-screen
    let inner_h = list_area.height.saturating_sub(2) as usize; // borders
    let header_lines = 3; // spacer + title + spacer
    let visible = inner_h.saturating_sub(header_lines);
    let offset = selected.saturating_sub(visible.saturating_sub(1));
    let end = (offset + visible).min(choices.len());

    let mut lines: Vec<ListItem> = Vec::new();

    // Title spacer
    lines.push(ListItem::new(""));
    lines.push(ListItem::new(Line::from(Span::styled(
        format!("  Select {}", opt_name),
        Style::default()
            .fg(Colors::SECONDARY)
            .add_modifier(Modifier::BOLD),
    ))));
    lines.push(ListItem::new(""));

    for (i, choice) in choices.iter().enumerate().skip(offset).take(end - offset) {
        let is_sel = i == selected;
        if is_sel {
            let text = format!(" \u{25b8} {}", choice);
            let padded = format!("{:<width$}", text, width = w);
            lines.push(ListItem::new(padded).style(
                Style::default()
                    .fg(Colors::BG_PRIMARY)
                    .bg(Colors::SECONDARY)
                    .add_modifier(Modifier::BOLD),
            ));
        } else {
            lines.push(
                ListItem::new(format!("   {}", choice))
                    .style(Style::default().fg(Colors::FG_PRIMARY)),
            );
        }
    }

    let pos = format!("{}/{}", selected + 1, choices.len());
    let block = panel_active(opt_name).title_bottom(
        Line::from(vec![Span::styled(
            format!(" {} ", pos),
            Style::default().fg(Colors::FG_MUTED),
        )])
        .alignment(Alignment::Right),
    );

    let list = List::new(lines).block(block);
    f.render_widget(list, list_area);

    // Scrollbar on selection list
    if choices.len() + 3 > list_area.height.saturating_sub(2) as usize {
        let mut scrollbar_state = ScrollbarState::new(choices.len()).position(selected);
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None)
            .track_symbol(Some("\u{2502}"))
            .thumb_symbol("\u{2588}")
            .track_style(Style::default().fg(Colors::SCROLLBAR_TRACK))
            .thumb_style(Style::default().fg(Colors::SCROLLBAR_THUMB));
        f.render_stateful_widget(scrollbar, list_area, &mut scrollbar_state);
    }

    // Disk layout panel
    if let Some(layout_rect) = layout_area {
        let layout_lines: Vec<Line<'_>> = state
            .disk_layout
            .iter()
            .map(|line| {
                let style =
                    if line.contains("NAME") || line.starts_with("  \u{2500}") {
                        Style::default().fg(Colors::FG_MUTED)
                    } else if line.contains("Disk model:")
                        || line.contains("Disklabel type:")
                        || line.contains("Disk /dev/")
                    {
                        Style::default()
                            .fg(Colors::SECONDARY)
                            .add_modifier(Modifier::BOLD)
                    } else if line.contains("Device")
                        || line.contains("Start")
                        || line.contains("End")
                    {
                        Style::default()
                            .fg(Colors::PRIMARY)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Colors::FG_PRIMARY)
                    };
                Line::from(Span::styled(format!(" {}", line), style))
            })
            .collect();

        let layout_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Colors::BORDER_ACTIVE))
            .title(Line::from(vec![
                Span::styled(
                    "\u{2500}",
                    Style::default().fg(Colors::BORDER_ACTIVE),
                ),
                Span::styled(
                    " Disk Layout ",
                    Style::default()
                        .fg(Colors::PRIMARY)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "\u{2500}",
                    Style::default().fg(Colors::BORDER_ACTIVE),
                ),
            ]))
            .style(Style::default().bg(Colors::BG_PRIMARY));

        let layout_para = Paragraph::new(layout_lines)
            .block(layout_block)
            .wrap(Wrap { trim: false });
        f.render_widget(layout_para, layout_rect);
    }
}

/// Text/password input with visible cursor
fn render_detail_text_input(
    f: &mut Frame,
    area: Rect,
    state: &AppState,
    value: &str,
    cursor: usize,
    is_password: bool,
) {
    let sel = state.config_scroll.selected_index;
    let opt = state.config.options.get(sel);
    let opt_name = opt.map(|o| o.name.as_str()).unwrap_or("Input");
    let opt_desc = opt.map(|o| o.description.as_str()).unwrap_or("");

    let display_value = if is_password {
        "\u{2022}".repeat(value.len())
    } else {
        value.to_string()
    };

    // Build cursor display: insert block cursor at position (char-safe for UTF-8)
    let before: String = display_value.chars().take(cursor).collect();
    let cursor_char: String = display_value
        .chars()
        .nth(cursor)
        .map(|c| c.to_string())
        .unwrap_or_else(|| " ".to_string());
    let rest: String = display_value.chars().skip(cursor + 1).collect();

    let input_line = Line::from(vec![
        Span::styled("  ", Style::default()),
        Span::styled(before, Style::default().fg(Colors::FG_PRIMARY)),
        Span::styled(
            cursor_char,
            Style::default()
                .fg(Colors::BG_PRIMARY)
                .bg(Colors::SECONDARY),
        ),
        Span::styled(rest, Style::default().fg(Colors::FG_PRIMARY)),
    ]);

    let mut lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("  {}", opt_name),
            Style::default()
                .fg(Colors::SECONDARY)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    if !opt_desc.is_empty() {
        lines.push(Line::from(Span::styled(
            format!("  {}", opt_desc),
            Style::default().fg(Colors::FG_MUTED),
        )));
        lines.push(Line::from(""));
    }

    // Input box area
    lines.push(Line::from(Span::styled(
        "  \u{250c}".to_string()
            + &"\u{2500}".repeat(area.width.saturating_sub(6) as usize)
            + "\u{2510}",
        Style::default().fg(Colors::BORDER_ACTIVE),
    )));
    lines.push(input_line);
    lines.push(Line::from(Span::styled(
        "  \u{2514}".to_string()
            + &"\u{2500}".repeat(area.width.saturating_sub(6) as usize)
            + "\u{2518}",
        Style::default().fg(Colors::BORDER_ACTIVE),
    )));

    lines.push(Line::from(""));

    if is_password {
        lines.push(Line::from(vec![
            Span::styled("  \u{1f512} ", Style::default().fg(Colors::WARNING)),
            Span::styled(
                "Input is masked for security",
                Style::default().fg(Colors::FG_MUTED),
            ),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled(
            "  Enter",
            Style::default()
                .fg(Colors::SECONDARY)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" to confirm  ", Style::default().fg(Colors::FG_MUTED)),
        Span::styled(
            "Esc",
            Style::default()
                .fg(Colors::SECONDARY)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" to cancel", Style::default().fg(Colors::FG_MUTED)),
    ]));

    let block = panel_active(opt_name);
    let info = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false });
    f.render_widget(info, area);
}

/// Interactive package selection: output log + command input OR search results
#[allow(clippy::too_many_arguments)]
fn render_detail_package_input(
    f: &mut Frame,
    area: Rect,
    packages: &[String],
    current_input: &str,
    output_lines: &[String],
    is_pacman: bool,
    search_results: &[crate::app::PackageResult],
    results_selected: usize,
    show_search_results: bool,
) {
    let block_title = if is_pacman {
        "Pacman Packages"
    } else {
        "AUR Packages"
    };

    let block = panel_active(block_title);
    let inner = block.inner(area);
    f.render_widget(block, area);

    if show_search_results && !search_results.is_empty() {
        // Search results browsing mode
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Package count
                Constraint::Min(3),   // Results list
                Constraint::Length(1), // Hint
            ])
            .split(inner);

        // Package count header
        let count_text = if packages.is_empty() {
            format!(" {} results | No packages selected", search_results.len())
        } else {
            format!(
                " {} results | {} selected: {}",
                search_results.len(),
                packages.len(),
                packages.join(", ")
            )
        };
        let count_line =
            Paragraph::new(count_text).style(Style::default().fg(Colors::FG_SECONDARY));
        f.render_widget(count_line, chunks[0]);

        // Results list with selection indicators
        let visible_height = chunks[1].height as usize;
        let scroll_offset = if results_selected >= visible_height {
            results_selected - visible_height + 1
        } else {
            0
        };

        let w = chunks[1].width as usize;
        let items: Vec<ListItem> = search_results
            .iter()
            .enumerate()
            .skip(scroll_offset)
            .take(visible_height)
            .map(|(i, result)| {
                let is_selected_pkg = packages.contains(&result.name);
                let is_highlighted = i == results_selected;

                let indicator = if is_selected_pkg { "\u{2713}" } else { " " };
                let text = format!(
                    " [{}] {}/{} ({}) - {}",
                    indicator, result.repo, result.name, result.version, result.description
                );

                if is_highlighted {
                    let padded = format!("{:<width$}", text, width = w);
                    ListItem::new(padded).style(
                        Style::default()
                            .fg(Colors::BG_PRIMARY)
                            .bg(Colors::SECONDARY)
                            .add_modifier(Modifier::BOLD),
                    )
                } else if is_selected_pkg {
                    ListItem::new(text).style(Style::default().fg(Colors::SUCCESS))
                } else {
                    ListItem::new(text).style(Style::default().fg(Colors::FG_PRIMARY))
                }
            })
            .collect();

        let result_list = List::new(items);
        f.render_widget(result_list, chunks[1]);

        // Scrollbar
        if search_results.len() > visible_height {
            let mut scrollbar_state =
                ScrollbarState::new(search_results.len()).position(results_selected);
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(None)
                .end_symbol(None)
                .track_symbol(Some("\u{2502}"))
                .thumb_symbol("\u{2588}")
                .track_style(Style::default().fg(Colors::SCROLLBAR_TRACK))
                .thumb_style(Style::default().fg(Colors::SCROLLBAR_THUMB));
            f.render_stateful_widget(scrollbar, chunks[1], &mut scrollbar_state);
        }

        // Hint bar
        let hint = Line::from(vec![
            Span::styled(
                "\u{2191}\u{2193}",
                Style::default().fg(Colors::SECONDARY),
            ),
            Span::styled(" Browse  ", Style::default().fg(Colors::FG_MUTED)),
            Span::styled("Enter", Style::default().fg(Colors::SECONDARY)),
            Span::styled(" Toggle  ", Style::default().fg(Colors::FG_MUTED)),
            Span::styled("Esc", Style::default().fg(Colors::SECONDARY)),
            Span::styled(" Back to commands", Style::default().fg(Colors::FG_MUTED)),
        ]);
        f.render_widget(Paragraph::new(hint), chunks[2]);
    } else {
        // Command mode
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Package count
                Constraint::Min(3),   // Output log
                Constraint::Length(3), // Input box
                Constraint::Length(1), // Hint
            ])
            .split(inner);

        // Package count header
        let count_text = if packages.is_empty() {
            " No packages added".to_string()
        } else {
            format!(" {} package(s): {}", packages.len(), packages.join(", "))
        };
        let count_line =
            Paragraph::new(count_text).style(Style::default().fg(Colors::FG_SECONDARY));
        f.render_widget(count_line, chunks[0]);

        // Output log (scrolled to bottom)
        let visible_height = chunks[1].height as usize;
        let start = output_lines.len().saturating_sub(visible_height);
        let visible_output: Vec<ListItem> = output_lines[start..]
            .iter()
            .map(|line| {
                let style = if line.starts_with(">>>") || line.starts_with("Commands:") {
                    Style::default().fg(Colors::PRIMARY)
                } else if line.starts_with("  +") {
                    Style::default().fg(Colors::SUCCESS)
                } else if line.starts_with("  -") {
                    Style::default().fg(Colors::ERROR)
                } else if line.starts_with("  *") {
                    Style::default().fg(Colors::SECONDARY) // amber/yellow for listed packages
                } else if line.starts_with("  ") && line.contains('(') {
                    Style::default().fg(Colors::FG_SECONDARY)
                } else {
                    Style::default().fg(Colors::FG_PRIMARY)
                };
                ListItem::new(format!(" {}", line)).style(style)
            })
            .collect();
        let output_list = List::new(visible_output);
        f.render_widget(output_list, chunks[1]);

        // Input box with cursor
        let input_line = Line::from(vec![
            Span::styled(" > ", Style::default().fg(Colors::SECONDARY)),
            Span::styled(current_input, Style::default().fg(Colors::FG_PRIMARY)),
            Span::styled(
                " ",
                Style::default()
                    .fg(Colors::BG_PRIMARY)
                    .bg(Colors::SECONDARY),
            ),
        ]);
        let input_box = Paragraph::new(input_line).block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Colors::BORDER_ACTIVE)),
        );
        f.render_widget(input_box, chunks[2]);

        // Hint bar
        let hint = Line::from(vec![
            Span::styled(
                " search/add/remove/list/done",
                Style::default().fg(Colors::FG_MUTED),
            ),
            Span::styled("  Esc", Style::default().fg(Colors::SECONDARY)),
            Span::styled(" cancel", Style::default().fg(Colors::FG_MUTED)),
        ]);
        f.render_widget(Paragraph::new(hint), chunks[3]);
    }
}

// =============================================================================
// Automated Install
// =============================================================================

pub fn render_automated_install_ui(f: &mut Frame, _state: &AppState, area: Rect) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(1)])
        .split(area);

    render_breadcrumb(f, layout[0], &["Automated Install"]);

    let panes = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(layout[1]);

    // Left panel — overview
    let description_lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  \u{26a1} Quick, Reproducible Installs",
            Style::default()
                .fg(Colors::SECONDARY)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  Uses a config file for automated setup.",
            Style::default().fg(Colors::FG_PRIMARY),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  \u{2713} ", Style::default().fg(Colors::SUCCESS)),
            Span::styled(
                "Disk partitioning & formatting",
                Style::default().fg(Colors::FG_SECONDARY),
            ),
        ]),
        Line::from(vec![
            Span::styled("  \u{2713} ", Style::default().fg(Colors::SUCCESS)),
            Span::styled(
                "Bootloader installation",
                Style::default().fg(Colors::FG_SECONDARY),
            ),
        ]),
        Line::from(vec![
            Span::styled("  \u{2713} ", Style::default().fg(Colors::SUCCESS)),
            Span::styled(
                "User account creation",
                Style::default().fg(Colors::FG_SECONDARY),
            ),
        ]),
        Line::from(vec![
            Span::styled("  \u{2713} ", Style::default().fg(Colors::SUCCESS)),
            Span::styled(
                "Desktop environment setup",
                Style::default().fg(Colors::FG_SECONDARY),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Formats: ", Style::default().fg(Colors::FG_MUTED)),
            Span::styled(".toml, .json", Style::default().fg(Colors::PRIMARY)),
        ]),
    ];

    let desc = Paragraph::new(description_lines)
        .block(panel_active("Overview"))
        .wrap(Wrap { trim: false });
    f.render_widget(desc, panes[0]);

    // Right panel — config example
    let config_lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  # Example config.toml",
            Style::default().fg(Colors::FG_MUTED),
        )),
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
        .block(panel_inactive("Config Format"))
        .wrap(Wrap { trim: false });
    f.render_widget(config_example, panes[1]);
}

// =============================================================================
// Installation Progress
// =============================================================================

/// Phase names for step indicators
const INSTALL_PHASES: &[&str] = &[
    "Partitioning",
    "Formatting",
    "Base Packages",
    "Bootloader",
    "System Config",
    "Users",
    "Desktop",
    "Final",
];

pub fn render_installation_ui(f: &mut Frame, state: &AppState, area: Rect) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Breadcrumb
            Constraint::Length(3), // Progress bar
            Constraint::Length(1), // Phase steps
            Constraint::Length(1), // Status
            Constraint::Min(1),   // Output
        ])
        .split(area);

    render_breadcrumb(f, layout[0], &["Installation"]);
    render_progress_bar(f, layout[1], state.installation_progress as u16);

    // Phase step indicators
    let current_phase = (state.installation_progress as usize * INSTALL_PHASES.len()) / 100;
    let phase_spans: Vec<Span> = INSTALL_PHASES
        .iter()
        .enumerate()
        .flat_map(|(i, name)| {
            let (icon, style) = if i < current_phase {
                (
                    "\u{2713}",
                    Style::default().fg(Colors::SUCCESS),
                )
            } else if i == current_phase {
                (
                    "\u{25b8}",
                    Style::default()
                        .fg(Colors::SECONDARY)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                (
                    "\u{25cb}",
                    Style::default().fg(Colors::FG_MUTED),
                )
            };
            vec![
                Span::styled(format!(" {}{} ", icon, name), style),
                Span::styled("\u{2022}", Style::default().fg(Colors::FG_MUTED)),
            ]
        })
        .collect();
    let phases = Paragraph::new(Line::from(phase_spans))
        .style(Style::default().bg(Colors::BG_SECONDARY))
        .alignment(Alignment::Center);
    f.render_widget(phases, layout[2]);

    // Status line
    let status_style = if state.installation_progress >= 100 {
        Style::default().fg(Colors::SUCCESS).bg(Colors::BG_PRIMARY)
    } else {
        Style::default()
            .fg(Colors::SECONDARY)
            .bg(Colors::BG_PRIMARY)
    };
    let status_line = Paragraph::new(Line::from(vec![
        Span::styled(
            " Status: ",
            Style::default()
                .fg(Colors::FG_MUTED)
                .bg(Colors::BG_PRIMARY),
        ),
        Span::styled(&state.status_message, status_style),
    ]))
    .style(Style::default().bg(Colors::BG_PRIMARY));
    f.render_widget(status_line, layout[3]);

    render_installer_output(
        f,
        layout[4],
        &state.installer_output,
        state.installer_scroll_offset,
        state.installer_auto_scroll,
    );
}

// =============================================================================
// Completion
// =============================================================================

pub fn render_completion_ui(f: &mut Frame, state: &AppState, area: Rect) {
    let is_success = state.installation_progress >= 100
        && !state.status_message.to_lowercase().contains("fail")
        && !state.status_message.to_lowercase().contains("error");

    let crumb = if is_success { "Complete" } else { "Failed" };

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Breadcrumb
            Constraint::Length(5), // Status banner
            Constraint::Min(1),   // Output log
            Constraint::Length(1), // Hint
        ])
        .split(area);

    render_breadcrumb(f, layout[0], &["Installation", crumb]);

    // Status banner
    let (icon, status_color) = if is_success {
        ("\u{2713}", Colors::SUCCESS)
    } else {
        ("\u{2717}", Colors::ERROR)
    };

    let banner_lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("  {}  {}", icon, state.status_message),
            Style::default()
                .fg(status_color)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];
    let banner = Paragraph::new(banner_lines)
        .block(panel_active("Status"))
        .alignment(Alignment::Left);
    f.render_widget(banner, layout[1]);

    // Output log (tail)
    let total_lines = state.installer_output.len();
    let pos_text = format!(" {}/{} lines ", total_lines, total_lines);
    let output_block = panel_inactive("Installer Output").title_bottom(
        Line::from(vec![Span::styled(
            pos_text,
            Style::default().fg(Colors::FG_MUTED),
        )])
        .alignment(Alignment::Right),
    );
    let inner_area = output_block.inner(layout[2]);
    f.render_widget(output_block, layout[2]);

    let visible_height = inner_area.height as usize;
    let start = state.installer_output.len().saturating_sub(visible_height);
    let tail: Vec<ListItem> = state.installer_output[start..]
        .iter()
        .map(|line| {
            let style = if line.contains("ERROR") || line.contains("error") {
                Style::default().fg(Colors::ERROR)
            } else if line.contains("WARNING") || line.contains("warning") {
                Style::default().fg(Colors::WARNING)
            } else if line.starts_with("==>") || line.starts_with("::") {
                Style::default()
                    .fg(Colors::INFO)
                    .add_modifier(Modifier::BOLD)
            } else if line.contains("SUCCESS") {
                Style::default().fg(Colors::SUCCESS)
            } else {
                Style::default().fg(Colors::FG_PRIMARY)
            };
            ListItem::new(line.as_str()).style(style)
        })
        .collect();
    let output_list = List::new(tail);
    f.render_widget(output_list, inner_area);

    // Scrollbar on output
    if total_lines > visible_height {
        let mut scrollbar_state = ScrollbarState::new(total_lines).position(start);
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None)
            .track_symbol(Some("\u{2502}"))
            .thumb_symbol("\u{2588}")
            .track_style(Style::default().fg(Colors::SCROLLBAR_TRACK))
            .thumb_style(Style::default().fg(Colors::SCROLLBAR_THUMB));
        f.render_stateful_widget(scrollbar, layout[2], &mut scrollbar_state);
    }

    // Hint bar
    let hint = Line::from(vec![
        Span::styled(" Enter", Style::default().fg(Colors::SECONDARY)),
        Span::styled("/", Style::default().fg(Colors::FG_MUTED)),
        Span::styled("B", Style::default().fg(Colors::SECONDARY)),
        Span::styled(" Menu  ", Style::default().fg(Colors::FG_MUTED)),
        Span::styled("Q", Style::default().fg(Colors::SECONDARY)),
        Span::styled(" Quit  ", Style::default().fg(Colors::FG_MUTED)),
        Span::styled("Log: ", Style::default().fg(Colors::FG_MUTED)),
        Span::styled("/var/log/archtui/", Style::default().fg(Colors::FG_SECONDARY)),
    ]);
    let hint_para = Paragraph::new(hint)
        .style(Style::default().bg(Colors::BG_SECONDARY))
        .alignment(Alignment::Center);
    f.render_widget(hint_para, layout[3]);
}

// =============================================================================
// Dry Run Summary
// =============================================================================

pub fn render_dry_run_summary(f: &mut Frame, state: &AppState, area: Rect) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(1)])
        .split(area);

    render_breadcrumb(f, layout[0], &["Configuration", "Dry Run"]);

    let block = panel_active("Actions to be Performed").title_bottom(
        Line::from(vec![Span::styled(
            " \u{2191}\u{2193} scroll | B=back | Enter=dismiss ",
            Style::default().fg(Colors::FG_MUTED),
        )])
        .alignment(Alignment::Right),
    );
    let inner_area = block.inner(layout[1]);
    f.render_widget(block, layout[1]);

    let visible_height = inner_area.height as usize;

    let summary_lines: Vec<ListItem> = if let Some(ref summary) = state.dry_run_summary {
        let max_offset = summary.len().saturating_sub(visible_height);
        let offset = state.dry_run_scroll_offset.min(max_offset);
        summary
            .iter()
            .skip(offset)
            .take(visible_height)
            .map(|line| {
                let style = if line.starts_with("[DESTRUCTIVE]") {
                    Style::default()
                        .fg(Colors::ERROR)
                        .add_modifier(Modifier::BOLD)
                } else if line.starts_with("[SKIP]") {
                    Style::default().fg(Colors::FG_MUTED)
                } else if line.starts_with("  ->") {
                    Style::default().fg(Colors::FG_SECONDARY)
                } else {
                    Style::default().fg(Colors::FG_PRIMARY)
                };
                ListItem::new(format!("  {}", line)).style(style)
            })
            .collect()
    } else {
        vec![ListItem::new("  No actions to perform")
            .style(Style::default().fg(Colors::FG_MUTED))]
    };

    let summary_list = List::new(summary_lines);
    f.render_widget(summary_list, inner_area);
}

// =============================================================================
// Action Buttons
// =============================================================================

fn render_start_button(f: &mut Frame, area: Rect, state: &AppState) {
    let is_button_row = state.config_scroll.selected_index == state.config.options.len();

    let button_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(33),
            Constraint::Percentage(34),
            Constraint::Percentage(33),
        ])
        .split(area);

    let buttons = [
        ("TEST CONFIG", 0, Colors::PRIMARY),
        ("EXPORT CONFIG", 1, Colors::PRIMARY),
        ("START INSTALL", 2, Colors::SUCCESS),
    ];

    for (label, idx, color) in buttons {
        let selected = is_button_row && state.installer_button_selection == idx;
        let style = if selected {
            Style::default()
                .fg(Colors::BG_PRIMARY)
                .bg(color)
                .add_modifier(Modifier::BOLD)
        } else if is_button_row {
            Style::default().fg(color)
        } else {
            Style::default().fg(Colors::FG_MUTED)
        };

        let display = if selected {
            format!("  {} (Enter)  ", label)
        } else {
            format!("  {}  ", label)
        };

        let button = Paragraph::new(display)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(if selected {
                        color
                    } else {
                        Colors::BORDER_INACTIVE
                    })),
            )
            .alignment(Alignment::Center)
            .style(style);
        f.render_widget(button, button_chunks[idx]);
    }
}
