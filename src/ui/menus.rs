//! Redesigned menu rendering
//!
//! No more header/title eating 7+ rows. Every screen is a split-pane layout
//! with the identity embedded in the border title. Selected items get a
//! full-width highlight bar. Rounded borders everywhere.

use super::descriptions;
use crate::app::AppState;
use crate::theme::Colors;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation,
        ScrollbarState, Wrap,
    },
};

// =============================================================================
// Shared helpers
// =============================================================================

/// Active panel block (focused) — rounded, teal border, embedded title
fn panel_active<'a>(title: &'a str, position: Option<&'a str>) -> Block<'a> {
    let mut block = Block::default()
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
        .style(Style::default().bg(Colors::BG_PRIMARY));

    if let Some(pos) = position {
        block = block.title_bottom(
            Line::from(vec![Span::styled(
                format!(" {} ", pos),
                Style::default().fg(Colors::FG_MUTED),
            )])
            .alignment(Alignment::Right),
        );
    }
    block
}

/// Active panel with category accent color for the title
fn panel_active_accent<'a>(title: &'a str, position: Option<&'a str>, accent: Color) -> Block<'a> {
    let mut block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(Line::from(vec![
            Span::styled("\u{2500}", Style::default().fg(Colors::BORDER_ACTIVE)),
            Span::styled(
                format!(" {} ", title),
                Style::default().fg(accent).add_modifier(Modifier::BOLD),
            ),
            Span::styled("\u{2500}", Style::default().fg(Colors::BORDER_ACTIVE)),
        ]))
        .border_style(Style::default().fg(Colors::BORDER_ACTIVE))
        .style(Style::default().bg(Colors::BG_PRIMARY));

    if let Some(pos) = position {
        block = block.title_bottom(
            Line::from(vec![Span::styled(
                format!(" {} ", pos),
                Style::default().fg(Colors::FG_MUTED),
            )])
            .alignment(Alignment::Right),
        );
    }
    block
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

/// Build a menu item with full-width highlight bar when selected
fn highlight_item(index: usize, selected: usize, label: &str, width: u16) -> ListItem<'static> {
    let is_sel = index == selected;
    let w = width.saturating_sub(2) as usize; // inside borders

    if is_sel {
        // Full-width highlight bar
        let text = format!(" \u{25b8} {}", label);
        let padded = format!("{:<width$}", text, width = w);
        ListItem::new(padded).style(
            Style::default()
                .fg(Colors::BG_PRIMARY)
                .bg(Colors::PRIMARY)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        let text = format!("   {}", label);
        ListItem::new(text).style(Style::default().fg(Colors::FG_PRIMARY))
    }
}

/// Render a vertical scrollbar on the right edge of an area
fn render_scrollbar(f: &mut Frame, area: Rect, total: usize, position: usize) {
    if total <= area.height.saturating_sub(2) as usize {
        return; // No scrollbar needed
    }
    let mut scrollbar_state = ScrollbarState::new(total).position(position);
    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(None)
        .end_symbol(None)
        .track_symbol(Some("\u{2502}"))
        .thumb_symbol("\u{2588}")
        .track_style(Style::default().fg(Colors::SCROLLBAR_TRACK))
        .thumb_style(Style::default().fg(Colors::SCROLLBAR_THUMB));
    f.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
}

/// Two-column split: menu left (38%), description right (62%)
fn split_pane(area: Rect) -> (Rect, Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(area);
    (chunks[0], chunks[1])
}

/// Header bar: single line with app name + screen breadcrumb
pub fn render_breadcrumb(f: &mut Frame, area: Rect, breadcrumb: &[&str]) {
    let mut spans = vec![Span::styled(
        " ArchTUI",
        Style::default()
            .fg(Colors::PRIMARY)
            .add_modifier(Modifier::BOLD),
    )];
    for part in breadcrumb {
        spans.push(Span::styled(
            " \u{203a} ",
            Style::default().fg(Colors::FG_MUTED),
        ));
        spans.push(Span::styled(
            *part,
            Style::default().fg(Colors::FG_SECONDARY),
        ));
    }

    let line = Line::from(spans);
    let bar = Paragraph::new(line)
        .style(Style::default().bg(Colors::BG_SECONDARY))
        .alignment(Alignment::Left);
    f.render_widget(bar, area);
}

// =============================================================================
// MAIN MENU — split pane with info panel
// =============================================================================

pub fn render_main_menu(f: &mut Frame, state: &AppState, area: Rect) {
    // Breadcrumb (1 line) + content
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(1)])
        .split(area);

    render_breadcrumb(f, layout[0], &[]);

    let (left, right) = split_pane(layout[1]);
    let sel = state.main_menu_selection;
    let count = 4;
    let pos = format!("{}/{}", sel + 1, count);

    let items: Vec<ListItem> = [
        "Guided Installer",
        "Automated Install",
        "Arch Linux Tools",
        "Quit",
    ]
    .iter()
    .enumerate()
    .map(|(i, name)| highlight_item(i, sel, name, left.width))
    .collect();

    let menu = List::new(items).block(panel_active("Main Menu", Some(&pos)));
    f.render_widget(menu, left);

    // Right panel: contextual info about selected option
    let info_lines = match sel {
        0 => vec![
            Line::from(""),
            Line::from(Span::styled(
                "  Guided Installer",
                Style::default()
                    .fg(Colors::SECONDARY)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "  Step-by-step interactive installation",
                Style::default().fg(Colors::FG_PRIMARY),
            )),
            Line::from(Span::styled(
                "  of Arch Linux. Recommended for users",
                Style::default().fg(Colors::FG_PRIMARY),
            )),
            Line::from(Span::styled(
                "  new to Arch.",
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
            Line::from(vec![
                Span::styled("  \u{2713} ", Style::default().fg(Colors::SUCCESS)),
                Span::styled(
                    "Custom package selection",
                    Style::default().fg(Colors::FG_SECONDARY),
                ),
            ]),
        ],
        1 => vec![
            Line::from(""),
            Line::from(Span::styled(
                "  Automated Install",
                Style::default()
                    .fg(Colors::SECONDARY)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "  Run an unattended installation from a",
                Style::default().fg(Colors::FG_PRIMARY),
            )),
            Line::from(Span::styled(
                "  TOML or JSON configuration file.",
                Style::default().fg(Colors::FG_PRIMARY),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "  Great for reproducible deployments",
                Style::default().fg(Colors::FG_SECONDARY),
            )),
            Line::from(Span::styled(
                "  across multiple machines.",
                Style::default().fg(Colors::FG_SECONDARY),
            )),
        ],
        2 => vec![
            Line::from(""),
            Line::from(Span::styled(
                "  Arch Linux Tools",
                Style::default()
                    .fg(Colors::SECONDARY)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "  System repair and administration tools.",
                Style::default().fg(Colors::FG_PRIMARY),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled("  \u{25cf} ", Style::default().fg(Colors::PRIMARY)),
                Span::styled("Disk Tools   ", Style::default().fg(Colors::FG_PRIMARY)),
                Span::styled(
                    "partition, format, wipe, LUKS",
                    Style::default().fg(Colors::FG_MUTED),
                ),
            ]),
            Line::from(vec![
                Span::styled("  \u{25cf} ", Style::default().fg(Colors::PRIMARY)),
                Span::styled("System Tools ", Style::default().fg(Colors::FG_PRIMARY)),
                Span::styled(
                    "bootloader, fstab, chroot",
                    Style::default().fg(Colors::FG_MUTED),
                ),
            ]),
            Line::from(vec![
                Span::styled("  \u{25cf} ", Style::default().fg(Colors::PRIMARY)),
                Span::styled("User Tools   ", Style::default().fg(Colors::FG_PRIMARY)),
                Span::styled(
                    "accounts, SSH, security",
                    Style::default().fg(Colors::FG_MUTED),
                ),
            ]),
            Line::from(vec![
                Span::styled("  \u{25cf} ", Style::default().fg(Colors::PRIMARY)),
                Span::styled("Network Tools", Style::default().fg(Colors::FG_PRIMARY)),
                Span::styled(
                    " config, firewall, mirrors",
                    Style::default().fg(Colors::FG_MUTED),
                ),
            ]),
        ],
        _ => vec![
            Line::from(""),
            Line::from(Span::styled(
                "  Quit",
                Style::default()
                    .fg(Colors::SECONDARY)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "  Exit ArchTUI.",
                Style::default().fg(Colors::FG_SECONDARY),
            )),
        ],
    };

    let info = Paragraph::new(info_lines)
        .block(panel_inactive("Details"))
        .wrap(Wrap { trim: false });
    f.render_widget(info, right);
}

// =============================================================================
// TOOLS MENU
// =============================================================================

pub fn render_tools_menu(f: &mut Frame, state: &AppState, area: Rect) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(1)])
        .split(area);
    render_breadcrumb(f, layout[0], &["Tools"]);

    let (left, right) = split_pane(layout[1]);
    let sel = state.tools_menu_selection;
    let count = 5;
    let pos = format!("{}/{}", sel + 1, count);

    let items: Vec<ListItem> = [
        "Disk Tools",
        "System Tools",
        "User Tools",
        "Network Tools",
        "\u{25c0} Back",
    ]
    .iter()
    .enumerate()
    .map(|(i, name)| highlight_item(i, sel, name, left.width))
    .collect();

    let menu = List::new(items).block(panel_active("Select Category", Some(&pos)));
    f.render_widget(menu, left);
    render_scrollbar(f, left, count, sel);

    let description = descriptions::get_tools_category_description(sel);
    let desc = Paragraph::new(description)
        .block(panel_inactive("Category Overview"))
        .wrap(Wrap { trim: false });
    f.render_widget(desc, right);
}

// =============================================================================
// Category menus — all use the same pattern
// =============================================================================

#[allow(clippy::too_many_arguments)]
fn render_category_menu(
    f: &mut Frame,
    state: &AppState,
    area: Rect,
    breadcrumb: &[&str],
    title: &str,
    accent: Option<Color>,
    items_data: &[&str],
    get_desc: fn(usize) -> Vec<Line<'static>>,
) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(1)])
        .split(area);
    render_breadcrumb(f, layout[0], breadcrumb);

    let (left, right) = split_pane(layout[1]);
    let sel = state.tools_menu_selection;
    let count = items_data.len();
    let pos = format!("{}/{}", sel + 1, count);

    let items: Vec<ListItem> = items_data
        .iter()
        .enumerate()
        .map(|(i, name)| highlight_item(i, sel, name, left.width))
        .collect();

    // Use accent color for panel title if provided
    let block = if let Some(acc) = accent {
        panel_active_accent(title, Some(&pos), acc)
    } else {
        panel_active(title, Some(&pos))
    };

    let menu = List::new(items).block(block);
    f.render_widget(menu, left);
    render_scrollbar(f, left, count, sel);

    let description = get_desc(sel);
    let desc = Paragraph::new(description)
        .block(panel_inactive("Tool Information"))
        .wrap(Wrap { trim: false });
    f.render_widget(desc, right);
}

pub fn render_disk_tools_menu(f: &mut Frame, state: &AppState, area: Rect) {
    render_category_menu(
        f,
        state,
        area,
        &["Tools", "Disk"],
        "Disk Tools",
        Some(Colors::CAT_DISK),
        &[
            "Partition Disk",
            "Format Partition",
            "Wipe Disk",
            "Check Disk Health",
            "Mount/Unmount",
            "LUKS Encryption",
            "\u{25c0} Back",
        ],
        descriptions::get_disk_tool_description,
    );
}

pub fn render_system_tools_menu(f: &mut Frame, state: &AppState, area: Rect) {
    render_category_menu(
        f,
        state,
        area,
        &["Tools", "System"],
        "System Tools",
        Some(Colors::CAT_SYSTEM),
        &[
            "Install Bootloader",
            "Generate fstab",
            "Chroot into System",
            "Manage Services",
            "System Info",
            "Enable Services",
            "Install AUR Helper",
            "Rebuild Initramfs",
            "View Install Logs",
            "\u{25c0} Back",
        ],
        descriptions::get_system_tool_description,
    );
}

pub fn render_user_tools_menu(f: &mut Frame, state: &AppState, area: Rect) {
    render_category_menu(
        f,
        state,
        area,
        &["Tools", "User"],
        "User Tools",
        Some(Colors::CAT_USER),
        &[
            "Add User",
            "Reset Password",
            "Manage Groups",
            "Configure SSH",
            "Security Audit",
            "Install Dotfiles",
            "Run As User",
            "\u{25c0} Back",
        ],
        descriptions::get_user_tool_description,
    );
}

pub fn render_network_tools_menu(f: &mut Frame, state: &AppState, area: Rect) {
    render_category_menu(
        f,
        state,
        area,
        &["Tools", "Network"],
        "Network Tools",
        Some(Colors::CAT_NETWORK),
        &[
            "Configure Network",
            "Test Connectivity",
            "Firewall Rules",
            "Network Info",
            "Update Mirrors",
            "\u{25c0} Back",
        ],
        descriptions::get_network_tool_description,
    );
}
