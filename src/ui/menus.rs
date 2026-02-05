//! Menu rendering module
//!
//! This module handles rendering of all menus: main menu, tools menu,
//! and tool category menus (disk, system, user, network).

use super::descriptions;
use super::header::HeaderRenderer;
use crate::app::AppState;
use crate::theme::Colors;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

/// Render main menu in specified area
pub fn render_main_menu_in_area(
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
            Constraint::Min(10),   // Menu
        ])
        .split(area);

    header.render_header(f, chunks[0]);
    header.render_title(f, chunks[1], "Arch Linux Toolkit");

    let menu_items = [
        " ▶ Guided Installer  (Recommended for new users)",
        " ▶ Automated Install (Run from configuration file)",
        " ▶ Arch Linux Tools  (System repair and administration)",
        " ▶ Quit",
    ];

    let menu_items: Vec<ListItem> = menu_items
        .iter()
        .enumerate()
        .map(|(index, item)| {
            let style = if index == state.main_menu_selection {
                Style::default()
                    .fg(Colors::SECONDARY)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(*item).style(style)
        })
        .collect();

    let menu = List::new(menu_items)
        .block(Block::default().borders(Borders::ALL).title("Main Menu"))
        .highlight_style(
            Style::default()
                .bg(Colors::INFO)
                .fg(Colors::FG_PRIMARY)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

    f.render_widget(menu, chunks[2]);
}

/// Render tools menu in specified area
pub fn render_tools_menu_in_area(
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
            Constraint::Min(10),   // Content
        ])
        .split(area);

    header.render_header(f, chunks[0]);
    header.render_title(f, chunks[1], "Arch Linux Tools");

    // Split content into menu and description
    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
        .split(chunks[2]);

    let menu_items = [
        ("💾", "Disk Tools"),
        ("🔧", "System Tools"),
        ("👥", "User Tools"),
        ("🌐", "Network Tools"),
        ("◀️ ", "Back to Main Menu"),
    ];

    let menu_items: Vec<ListItem> = menu_items
        .iter()
        .enumerate()
        .map(|(index, (icon, name))| {
            let style = if index == state.tools_menu_selection {
                Style::default()
                    .fg(Colors::SECONDARY)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Colors::FG_PRIMARY)
            };
            let prefix = if index == state.tools_menu_selection {
                "▸ "
            } else {
                "  "
            };
            ListItem::new(format!("{}{} {}", prefix, icon, name)).style(style)
        })
        .collect();

    let menu = List::new(menu_items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Select Category ")
                .title_style(
                    Style::default()
                        .fg(Colors::PRIMARY)
                        .add_modifier(Modifier::BOLD),
                )
                .border_style(Style::default().fg(Colors::PRIMARY)),
        )
        .style(Style::default().bg(Colors::BG_PRIMARY));

    f.render_widget(menu, content_chunks[0]);

    // Description panel
    let description = descriptions::get_tools_category_description(state.tools_menu_selection);
    let desc_widget = Paragraph::new(description)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Category Overview ")
                .title_style(
                    Style::default()
                        .fg(Colors::PRIMARY)
                        .add_modifier(Modifier::BOLD),
                )
                .border_style(Style::default().fg(Colors::PRIMARY)),
        )
        .style(Style::default().bg(Colors::BG_PRIMARY))
        .wrap(Wrap { trim: false });

    f.render_widget(desc_widget, content_chunks[1]);
}

/// Render disk tools menu in specified area
pub fn render_disk_tools_menu_in_area(
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
            Constraint::Min(10),   // Content
        ])
        .split(area);

    header.render_header(f, chunks[0]);
    header.render_title(f, chunks[1], "Disk & Filesystem Tools");

    // Split content into menu and description
    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
        .split(chunks[2]);

    let menu_items = [
        ("💾", "Partition Disk", "cfdisk"),
        ("📀", "Format Partition", "mkfs"),
        ("🗑️ ", "Wipe Disk", "secure erase"),
        ("🔍", "Check Disk Health", "SMART"),
        ("📁", "Mount/Unmount", "mount"),
        ("◀️ ", "Back to Tools Menu", ""),
    ];

    let menu_items: Vec<ListItem> = menu_items
        .iter()
        .enumerate()
        .map(|(index, (icon, name, _))| {
            let style = if index == state.tools_menu_selection {
                Style::default()
                    .fg(Colors::SECONDARY)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Colors::FG_PRIMARY)
            };
            let prefix = if index == state.tools_menu_selection {
                "▸ "
            } else {
                "  "
            };
            ListItem::new(format!("{}{} {}", prefix, icon, name)).style(style)
        })
        .collect();

    let menu = List::new(menu_items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Select Tool ")
                .title_style(
                    Style::default()
                        .fg(Colors::PRIMARY)
                        .add_modifier(Modifier::BOLD),
                )
                .border_style(Style::default().fg(Colors::PRIMARY)),
        )
        .style(Style::default().bg(Colors::BG_PRIMARY));

    f.render_widget(menu, content_chunks[0]);

    // Description panel
    let description = descriptions::get_disk_tool_description(state.tools_menu_selection);
    let desc_widget = Paragraph::new(description)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Tool Information ")
                .title_style(
                    Style::default()
                        .fg(Colors::PRIMARY)
                        .add_modifier(Modifier::BOLD),
                )
                .border_style(Style::default().fg(Colors::PRIMARY)),
        )
        .style(Style::default().bg(Colors::BG_PRIMARY))
        .wrap(Wrap { trim: false });

    f.render_widget(desc_widget, content_chunks[1]);
}

/// Render system tools menu in specified area
pub fn render_system_tools_menu_in_area(
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
            Constraint::Min(10),   // Content
        ])
        .split(area);

    header.render_header(f, chunks[0]);
    header.render_title(f, chunks[1], "System Configuration Tools");

    // Split content into menu and description
    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
        .split(chunks[2]);

    let menu_items = [
        ("🔧", "Install Bootloader"),
        ("📋", "Generate fstab"),
        ("🖥️ ", "Chroot into System"),
        ("⚙️ ", "Manage Services"),
        ("ℹ️ ", "System Info"),
        ("◀️ ", "Back to Tools Menu"),
    ];

    let menu_items: Vec<ListItem> = menu_items
        .iter()
        .enumerate()
        .map(|(index, (icon, name))| {
            let style = if index == state.tools_menu_selection {
                Style::default()
                    .fg(Colors::SECONDARY)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Colors::FG_PRIMARY)
            };
            let prefix = if index == state.tools_menu_selection {
                "▸ "
            } else {
                "  "
            };
            ListItem::new(format!("{}{} {}", prefix, icon, name)).style(style)
        })
        .collect();

    let menu = List::new(menu_items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Select Tool ")
                .title_style(
                    Style::default()
                        .fg(Colors::PRIMARY)
                        .add_modifier(Modifier::BOLD),
                )
                .border_style(Style::default().fg(Colors::PRIMARY)),
        )
        .style(Style::default().bg(Colors::BG_PRIMARY));

    f.render_widget(menu, content_chunks[0]);

    // Description panel
    let description = descriptions::get_system_tool_description(state.tools_menu_selection);
    let desc_widget = Paragraph::new(description)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Tool Information ")
                .title_style(
                    Style::default()
                        .fg(Colors::PRIMARY)
                        .add_modifier(Modifier::BOLD),
                )
                .border_style(Style::default().fg(Colors::PRIMARY)),
        )
        .style(Style::default().bg(Colors::BG_PRIMARY))
        .wrap(Wrap { trim: false });

    f.render_widget(desc_widget, content_chunks[1]);
}

/// Render user tools menu in specified area
pub fn render_user_tools_menu_in_area(
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
            Constraint::Min(10),   // Content
        ])
        .split(area);

    header.render_header(f, chunks[0]);
    header.render_title(f, chunks[1], "User & Security Tools");

    // Split content into menu and description
    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
        .split(chunks[2]);

    let menu_items = [
        ("👤", "Add User"),
        ("🔑", "Reset Password"),
        ("👥", "Manage Groups"),
        ("🔒", "Configure SSH"),
        ("🛡️ ", "Security Audit"),
        ("◀️ ", "Back to Tools Menu"),
    ];

    let menu_items: Vec<ListItem> = menu_items
        .iter()
        .enumerate()
        .map(|(index, (icon, name))| {
            let style = if index == state.tools_menu_selection {
                Style::default()
                    .fg(Colors::SECONDARY)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Colors::FG_PRIMARY)
            };
            let prefix = if index == state.tools_menu_selection {
                "▸ "
            } else {
                "  "
            };
            ListItem::new(format!("{}{} {}", prefix, icon, name)).style(style)
        })
        .collect();

    let menu = List::new(menu_items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Select Tool ")
                .title_style(
                    Style::default()
                        .fg(Colors::PRIMARY)
                        .add_modifier(Modifier::BOLD),
                )
                .border_style(Style::default().fg(Colors::PRIMARY)),
        )
        .style(Style::default().bg(Colors::BG_PRIMARY));

    f.render_widget(menu, content_chunks[0]);

    // Description panel
    let description = descriptions::get_user_tool_description(state.tools_menu_selection);
    let desc_widget = Paragraph::new(description)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Tool Information ")
                .title_style(
                    Style::default()
                        .fg(Colors::PRIMARY)
                        .add_modifier(Modifier::BOLD),
                )
                .border_style(Style::default().fg(Colors::PRIMARY)),
        )
        .style(Style::default().bg(Colors::BG_PRIMARY))
        .wrap(Wrap { trim: false });

    f.render_widget(desc_widget, content_chunks[1]);
}

/// Render network tools menu in specified area
pub fn render_network_tools_menu_in_area(
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
            Constraint::Min(10),   // Content
        ])
        .split(area);

    header.render_header(f, chunks[0]);
    header.render_title(f, chunks[1], "Network Configuration Tools");

    // Split content into menu and description
    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
        .split(chunks[2]);

    let menu_items = [
        ("🌐", "Configure Network"),
        ("📡", "Test Connectivity"),
        ("🔥", "Firewall Rules"),
        ("📊", "Network Info"),
        ("◀️ ", "Back to Tools Menu"),
    ];

    let menu_items: Vec<ListItem> = menu_items
        .iter()
        .enumerate()
        .map(|(index, (icon, name))| {
            let style = if index == state.tools_menu_selection {
                Style::default()
                    .fg(Colors::SECONDARY)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Colors::FG_PRIMARY)
            };
            let prefix = if index == state.tools_menu_selection {
                "▸ "
            } else {
                "  "
            };
            ListItem::new(format!("{}{} {}", prefix, icon, name)).style(style)
        })
        .collect();

    let menu = List::new(menu_items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Select Tool ")
                .title_style(
                    Style::default()
                        .fg(Colors::PRIMARY)
                        .add_modifier(Modifier::BOLD),
                )
                .border_style(Style::default().fg(Colors::PRIMARY)),
        )
        .style(Style::default().bg(Colors::BG_PRIMARY));

    f.render_widget(menu, content_chunks[0]);

    // Description panel
    let description = descriptions::get_network_tool_description(state.tools_menu_selection);
    let desc_widget = Paragraph::new(description)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Tool Information ")
                .title_style(
                    Style::default()
                        .fg(Colors::PRIMARY)
                        .add_modifier(Modifier::BOLD),
                )
                .border_style(Style::default().fg(Colors::PRIMARY)),
        )
        .style(Style::default().bg(Colors::BG_PRIMARY))
        .wrap(Wrap { trim: false });

    f.render_widget(desc_widget, content_chunks[1]);
}
