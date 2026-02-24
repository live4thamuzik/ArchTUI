//! Header and common widget rendering
//!
//! This module contains the ASCII art header, title rendering,
//! progress bars, and other common UI elements.

use crate::app::AppState;
use crate::components::help_overlay::HelpOverlay;
use crate::components::keybindings::KeybindingContext;
use crate::components::nav_bar::NavBar;
use crate::theme::Colors;
use ratatui::{
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph},
    Frame,
};

/// Header renderer containing the ASCII art header
pub struct HeaderRenderer {
    /// ASCII art header lines
    header_lines: Vec<Line<'static>>,
}

impl Default for HeaderRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl HeaderRenderer {
    /// Create a new header renderer
    pub fn new() -> Self {
        Self {
            header_lines: Self::create_header(),
        }
    }

    /// Render the ASCII art header
    pub fn render_header(&self, f: &mut Frame, area: Rect) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        let header = Paragraph::new(self.header_lines.clone())
            .block(Block::default().borders(Borders::NONE))
            .alignment(Alignment::Center);
        f.render_widget(header, area);
    }

    /// Render a title section
    pub fn render_title(&self, f: &mut Frame, area: Rect, title: &str) {
        let title_widget = Paragraph::new(title)
            .block(Block::default().borders(Borders::ALL))
            .alignment(Alignment::Center)
            .style(Style::default().fg(Colors::PRIMARY));
        f.render_widget(title_widget, area);
    }

    /// Create the ASCII art header
    fn create_header() -> Vec<Line<'static>> {
        vec![
            Line::from(vec![Span::styled(
                "   █████████                      █████      ███████████ █████  █████ █████",
                Style::default().fg(Colors::PRIMARY),
            )]),
            Line::from(vec![Span::styled(
                "  ███▒▒▒▒▒███                    ▒▒███      ▒█▒▒▒███▒▒▒█▒▒███  ▒▒███ ▒▒███ ",
                Style::default().fg(Colors::PRIMARY),
            )]),
            Line::from(vec![Span::styled(
                " ▒███    ▒███  ████████   ██████  ▒███████  ▒   ▒███  ▒  ▒███   ▒███  ▒███ ",
                Style::default().fg(Colors::PRIMARY),
            )]),
            Line::from(vec![Span::styled(
                " ▒███████████ ▒▒███▒▒███ ███▒▒███ ▒███▒▒███     ▒███     ▒███   ▒███  ▒███ ",
                Style::default().fg(Colors::PRIMARY),
            )]),
            Line::from(vec![Span::styled(
                " ▒███▒▒▒▒▒███  ▒███ ▒▒▒ ▒███ ▒▒▒  ▒███ ▒███     ▒███     ▒███   ▒███  ▒███ ",
                Style::default().fg(Colors::PRIMARY),
            )]),
            Line::from(vec![Span::styled(
                " ▒███    ▒███  ▒███     ▒███  ███ ▒███ ▒███     ▒███     ▒███   ▒███  ▒███ ",
                Style::default().fg(Colors::PRIMARY),
            )]),
            Line::from(vec![Span::styled(
                " █████   █████ █████    ▒▒██████  ████ █████    █████    ▒▒████████   █████",
                Style::default().fg(Colors::PRIMARY),
            )]),
            Line::from(vec![Span::styled(
                "▒▒▒▒▒   ▒▒▒▒▒ ▒▒▒▒▒      ▒▒▒▒▒▒  ▒▒▒▒ ▒▒▒▒▒    ▒▒▒▒▒      ▒▒▒▒▒▒▒▒   ▒▒▒▒▒ ",
                Style::default().fg(Colors::PRIMARY),
            )]),
        ]
    }
}

/// Render instructions text
pub fn render_instructions(f: &mut Frame, area: Rect, text: &str) {
    let instructions = Paragraph::new(text)
        .block(Block::default().borders(Borders::NONE))
        .alignment(Alignment::Center)
        .style(Style::default().fg(Colors::SECONDARY));
    f.render_widget(instructions, area);
}

/// Render progress bar
pub fn render_progress_bar(f: &mut Frame, area: Rect, progress: u16) {
    let gauge = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Installation Progress"),
        )
        .gauge_style(Style::default().fg(Colors::INFO))
        .percent(progress);
    f.render_widget(gauge, area);
}

/// Render installer output with auto-scroll and manual scroll support.
/// Uses manual slicing (like FloatingOutput) to avoid wrap-offset mismatch.
pub fn render_installer_output(
    f: &mut Frame,
    area: Rect,
    output: &[String],
    scroll_offset: usize,
    auto_scroll: bool,
) {
    let content_block = Block::default()
        .borders(Borders::ALL)
        .title("Installer Output (↑↓ scroll)");
    let inner_area = content_block.inner(area);
    let visible_height = inner_area.height as usize;

    let start = if auto_scroll {
        output.len().saturating_sub(visible_height)
    } else {
        scroll_offset.min(output.len().saturating_sub(visible_height))
    };
    let end = (start + visible_height).min(output.len());

    let visible_content: Vec<ListItem> = output[start..end]
        .iter()
        .map(|line| {
            let style = if line.contains("ERROR") || line.contains("FATAL") {
                Style::default().fg(Colors::ERROR)
            } else if line.contains("WARNING") || line.contains("WARN:") {
                Style::default().fg(Colors::WARNING)
            } else if line.contains("SUCCESS:") {
                Style::default().fg(Colors::SUCCESS)
            } else if line.starts_with("==>") || line.starts_with("::") || line.contains("Phase ") {
                Style::default()
                    .fg(Colors::INFO)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Colors::FG_PRIMARY)
            };
            ListItem::new(line.as_str()).style(style)
        })
        .collect();

    let list = List::new(visible_content).block(content_block);
    f.render_widget(list, area);
}

/// Render the navigation bar
pub fn render_nav_bar(
    f: &mut Frame,
    state: &AppState,
    keybinding_ctx: &KeybindingContext,
    area: Rect,
) {
    let nav_items = keybinding_ctx.get_nav_items(&state.mode);
    let nav_bar = NavBar::new(nav_items);
    nav_bar.render(f, area);
}

/// Render the help overlay
pub fn render_help_overlay(f: &mut Frame, state: &AppState, keybinding_ctx: &KeybindingContext) {
    let help_overlay = HelpOverlay::new(&state.mode, keybinding_ctx);
    help_overlay.render(f, f.area());
}
