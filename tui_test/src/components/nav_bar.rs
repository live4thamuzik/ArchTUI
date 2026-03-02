//! Redesigned navigation bar
//!
//! New style: [Key->Action] with muted brackets, bright key, dim action.

#![allow(dead_code)]

use super::keybindings::NavBarItem;
use crate::theme::Colors;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub struct NavBar {
    items: Vec<NavBarItem>,
}

impl NavBar {
    pub fn new(items: Vec<NavBarItem>) -> Self {
        Self { items }
    }

    pub fn render(&self, f: &mut Frame, area: Rect) {
        if area.height == 0 || area.width == 0 {
            return;
        }

        let mut spans = Vec::new();
        spans.push(Span::styled(" ", Style::default()));

        for (i, item) in self.items.iter().enumerate() {
            if i > 0 {
                spans.push(Span::styled(
                    " ",
                    Style::default().fg(Colors::NAV_BRACKET),
                ));
            }

            // [Key->Action]
            spans.push(Span::styled("[", Style::default().fg(Colors::NAV_BRACKET)));
            spans.push(Span::styled(
                &item.key_display,
                Style::default()
                    .fg(Colors::NAV_KEY)
                    .add_modifier(Modifier::BOLD),
            ));
            spans.push(Span::styled(
                "\u{2192}",
                Style::default().fg(Colors::NAV_ARROW),
            ));
            spans.push(Span::styled(
                &item.action_label,
                Style::default().fg(Colors::NAV_ACTION),
            ));
            spans.push(Span::styled("]", Style::default().fg(Colors::NAV_BRACKET)));
        }

        let line = Line::from(spans);
        let paragraph = Paragraph::new(line)
            .block(
                Block::default()
                    .borders(Borders::NONE)
                    .style(Style::default().bg(Colors::BG_SECONDARY)),
            )
            .style(Style::default().bg(Colors::BG_SECONDARY));

        f.render_widget(paragraph, area);
    }

    pub fn height() -> u16 {
        1
    }
}
