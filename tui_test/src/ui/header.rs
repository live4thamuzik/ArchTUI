//! Utility renderers for installation screens.
//! The big ASCII header and separate title bar are GONE.
//! Screen identity is now embedded in border titles.
//!
//! `HeaderRenderer` is kept as a no-op struct for API compatibility.

#![allow(dead_code)]

use crate::theme::Colors;
use ratatui::{
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Clear, Gauge, List, ListItem, Scrollbar,
        ScrollbarOrientation, ScrollbarState,
    },
    Frame,
};

/// Render progress bar with rounded border
pub fn render_progress_bar(f: &mut Frame, area: Rect, progress: u16) {
    let gauge = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(" Progress ")
                .title_style(Style::default().fg(Colors::PRIMARY).add_modifier(Modifier::BOLD))
                .border_style(Style::default().fg(Colors::BORDER_ACTIVE)),
        )
        .gauge_style(Style::default().fg(Colors::SUCCESS).bg(Colors::BG_GAUGE))
        .percent(progress);
    f.render_widget(gauge, area);
}

/// Render installer output with colored log lines, scrollbar, and position indicator
pub fn render_installer_output(
    f: &mut Frame,
    area: Rect,
    output: &[String],
    scroll_offset: usize,
    auto_scroll: bool,
) {
    f.render_widget(Clear, area);

    let total = output.len();

    let content_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(Line::from(vec![
            Span::styled("\u{2500}", Style::default().fg(Colors::BORDER_ACTIVE)),
            Span::styled(
                " Installer Output ",
                Style::default()
                    .fg(Colors::PRIMARY)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("\u{2500}", Style::default().fg(Colors::BORDER_ACTIVE)),
        ]))
        .border_style(Style::default().fg(Colors::BORDER_ACTIVE))
        .style(Style::default().bg(Colors::BG_PRIMARY));

    let inner_area = content_block.inner(area);
    let visible_height = inner_area.height as usize;

    let start = if auto_scroll {
        output.len().saturating_sub(visible_height)
    } else {
        scroll_offset.min(output.len().saturating_sub(visible_height))
    };
    let end = (start + visible_height).min(output.len());

    // Position indicator in bottom-right
    let pos_text = if total > visible_height {
        format!(
            " {}-{}/{} ",
            start + 1,
            end.min(total),
            total
        )
    } else {
        format!(" {}/{} ", total, total)
    };
    let content_block = content_block.title_bottom(
        Line::from(vec![Span::styled(
            pos_text,
            Style::default().fg(Colors::FG_MUTED),
        )])
        .alignment(Alignment::Right),
    );

    f.render_widget(content_block, area);

    let visible_content: Vec<ListItem> = output[start..end]
        .iter()
        .map(|line| {
            let style = if line.contains("ERROR") || line.contains("FATAL") {
                Style::default().fg(Colors::ERROR).bg(Colors::BG_PRIMARY)
            } else if line.contains("WARNING") || line.contains("WARN:") {
                Style::default().fg(Colors::WARNING).bg(Colors::BG_PRIMARY)
            } else if line.contains("SUCCESS") {
                Style::default().fg(Colors::SUCCESS).bg(Colors::BG_PRIMARY)
            } else if line.starts_with("==>")
                || line.starts_with("::")
                || line.contains("Phase ")
            {
                Style::default()
                    .fg(Colors::INFO)
                    .bg(Colors::BG_PRIMARY)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
                    .fg(Colors::FG_PRIMARY)
                    .bg(Colors::BG_PRIMARY)
            };
            ListItem::new(line.as_str()).style(style)
        })
        .collect();

    let list = List::new(visible_content).style(Style::default().bg(Colors::BG_PRIMARY));
    f.render_widget(list, inner_area);

    // Scrollbar
    if total > visible_height {
        let mut scrollbar_state = ScrollbarState::new(total).position(start);
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

/// No-op header renderer kept for API compatibility.
///
/// The redesigned UI embeds identity in border titles instead of a
/// separate header bar. This struct exists so that `UiRenderer::new()`
/// and any code that constructs a `HeaderRenderer` still compiles.
pub struct HeaderRenderer;

impl HeaderRenderer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for HeaderRenderer {
    fn default() -> Self {
        Self::new()
    }
}

/// Legacy nav bar render — now delegates to the redesigned NavBar component.
/// Kept so that code calling `header::render_nav_bar` still compiles.
pub fn render_nav_bar(
    f: &mut Frame,
    state: &crate::app::AppState,
    keybinding_ctx: &crate::components::keybindings::KeybindingContext,
    area: Rect,
) {
    let nav_items = keybinding_ctx.get_nav_items(&state.mode);
    let nav_bar = crate::components::nav_bar::NavBar::new(nav_items);
    nav_bar.render(f, area);
}

/// Legacy help overlay render — now delegates to the redesigned HelpOverlay component.
pub fn render_help_overlay(
    f: &mut Frame,
    state: &crate::app::AppState,
    keybinding_ctx: &crate::components::keybindings::KeybindingContext,
) {
    let help = crate::components::help_overlay::HelpOverlay::new(&state.mode, keybinding_ctx);
    help.render(f, f.area());
}
