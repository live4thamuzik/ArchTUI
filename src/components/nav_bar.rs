//! Navigation bar component
//!
//! A persistent bottom bar showing context-sensitive keybindings.

#![allow(dead_code)]

use super::keybindings::NavBarItem;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

/// Navigation bar component
pub struct NavBar {
    items: Vec<NavBarItem>,
}

impl NavBar {
    /// Create a new navigation bar with the given items
    pub fn new(items: Vec<NavBarItem>) -> Self {
        Self { items }
    }

    /// Render the navigation bar
    pub fn render(&self, f: &mut Frame, area: Rect) {
        if area.height == 0 || area.width == 0 {
            return;
        }

        let mut spans = Vec::new();

        for (i, item) in self.items.iter().enumerate() {
            if i > 0 {
                spans.push(Span::styled(
                    "  ",
                    Style::default().fg(Color::DarkGray),
                ));
            }

            // Key in brackets with cyan color
            spans.push(Span::styled(
                "[",
                Style::default().fg(Color::DarkGray),
            ));
            spans.push(Span::styled(
                &item.key_display,
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ));
            spans.push(Span::styled(
                "]",
                Style::default().fg(Color::DarkGray),
            ));

            // Action label
            spans.push(Span::styled(
                " ",
                Style::default(),
            ));
            spans.push(Span::styled(
                &item.action_label,
                Style::default().fg(Color::White),
            ));
        }

        let line = Line::from(spans);
        let paragraph = Paragraph::new(line)
            .block(
                Block::default()
                    .borders(Borders::NONE)
                    .style(Style::default().bg(Color::Rgb(30, 30, 40))),
            )
            .style(Style::default().bg(Color::Rgb(30, 30, 40)));

        f.render_widget(paragraph, area);
    }

    /// Get the required height for the navigation bar
    pub fn height() -> u16 {
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nav_bar_creation() {
        let items = vec![
            NavBarItem {
                key_display: "Up/Dn".to_string(),
                action_label: "Navigate".to_string(),
            },
            NavBarItem {
                key_display: "Enter".to_string(),
                action_label: "Select".to_string(),
            },
        ];
        let nav_bar = NavBar::new(items);
        assert_eq!(nav_bar.items.len(), 2);
    }
}
