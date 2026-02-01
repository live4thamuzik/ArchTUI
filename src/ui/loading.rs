//! loading.rs - Renders a loading screen for the application.
//!
//! This module provides a function to draw a simple loading indicator
//! when the application is busy fetching data or executing background commands.

use ratatui::
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
;

use crate::app::AppState;

/// Renders a loading screen with a spinner and a message.
pub fn render_loading_screen(f: &mut Frame, state: &AppState) {
    let size = f.size();

    // Create a centered layout for the loading message
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(40), // Top padding
            Constraint::Length(5),      // Message area
            Constraint::Percentage(40), // Bottom padding
        ])
        .split(size);

    let message_area = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(20), // Left padding
            Constraint::Percentage(60), // Message area
            Constraint::Percentage(20), // Right padding
        ])
        .split(chunks[1])[1];

    // Construct the paragraph for the loading message
    let loading_message_text = state.status_message.clone();
    let title_text = "Loading Data";

    // Create a simple spinner animation
    let spinner_frames = vec!["-", "\\", "|", "/"];
    let spinner_frame = spinner_frames[(state.tick_count as usize / 5) % spinner_frames.len()];

    // Construct the paragraph for the loading message
    let paragraph = Paragraph::new(vec![
        Line::from(format!(" {} {}", spinner_frame, loading_message_text)),
        Line::raw(""),
        Line::styled(
            "Please wait...",
            Style::default().add_modifier(Modifier::ITALIC),
        ),
    ])
    .alignment(Alignment::Center)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title(Span::from(title_text).patch_style(Style::default().add_modifier(Modifier::BOLD))),
    );

    f.render_widget(paragraph, message_area);
}
