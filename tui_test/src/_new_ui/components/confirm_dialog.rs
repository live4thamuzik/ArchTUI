//! Confirmation dialog component (adapted for tui_test)
//!
//! Redesigned: rounded borders, severity-colored chrome, filled/outlined buttons.

#![allow(dead_code)]

use crate::app::ConfirmDialogState;
use crate::theme::{Colors, Severity, Styles, Theme, UiText};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap},
    Frame,
};

pub struct ConfirmDialog;

impl ConfirmDialog {
    pub fn render(f: &mut Frame, state: &ConfirmDialogState) {
        let area = f.area();

        let dialog_width = 64u16.min(area.width.saturating_sub(4));
        let detail_rows = if state.details.is_empty() {
            0
        } else {
            state.details.len() as u16 + 1
        };
        let dialog_height = (10 + detail_rows).min(area.height.saturating_sub(4));
        let dialog_x = (area.width.saturating_sub(dialog_width)) / 2;
        let dialog_y = (area.height.saturating_sub(dialog_height)) / 2;
        let dialog_area = Rect::new(dialog_x, dialog_y, dialog_width, dialog_height);

        f.render_widget(Clear, dialog_area);

        let severity = match state.severity {
            crate::app::ConfirmSeverity::Info => Severity::Info,
            crate::app::ConfirmSeverity::Warning => Severity::Warning,
            crate::app::ConfirmSeverity::Danger => Severity::Danger,
        };
        let border_color = Theme::severity_color(severity);
        let icon = Theme::severity_icon(severity);

        // Background tint based on severity
        let bg_style = match severity {
            Severity::Danger => Style::default().bg(Colors::BG_DANGER),
            _ => Style::default().bg(Colors::BG_PRIMARY),
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title(Line::from(vec![
                Span::styled("\u{2500}", Style::default().fg(border_color)),
                Span::styled(
                    format!(" {} {} ", icon, state.title),
                    Style::default()
                        .fg(border_color)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("\u{2500}", Style::default().fg(border_color)),
            ]))
            .border_style(Style::default().fg(border_color))
            .style(bg_style);

        let inner = block.inner(dialog_area);
        f.render_widget(block, dialog_area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(2), // Message
                Constraint::Min(1),   // Details
                Constraint::Length(3), // Buttons
            ])
            .split(inner);

        // Message
        let message_style = Theme::severity_style(severity);
        let message = Paragraph::new(state.message.clone())
            .style(message_style)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });
        f.render_widget(message, chunks[0]);

        // Details
        if !state.details.is_empty() {
            let detail_lines: Vec<Line> = state
                .details
                .iter()
                .map(|d| {
                    Line::from(vec![
                        Span::styled(
                            "  \u{2022} ",
                            Style::default().fg(border_color),
                        ),
                        Span::styled(d.clone(), Styles::text_secondary()),
                    ])
                })
                .collect();
            let details = Paragraph::new(detail_lines).wrap(Wrap { trim: true });
            f.render_widget(details, chunks[1]);
        }

        // Buttons with filled/outlined styling
        let button_area = chunks[2];
        let button_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(10),
                Constraint::Percentage(35),
                Constraint::Percentage(10),
                Constraint::Percentage(35),
                Constraint::Percentage(10),
            ])
            .split(button_area);

        // No/Cancel button (index 1)
        let no_selected = state.selected == 0;
        let no_style = if no_selected {
            Style::default()
                .fg(Colors::BG_PRIMARY)
                .bg(Colors::FG_PRIMARY)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Colors::FG_SECONDARY)
        };
        let no_border_style = if no_selected {
            Style::default().fg(Colors::FG_PRIMARY)
        } else {
            Style::default().fg(Colors::BORDER_INACTIVE)
        };
        let no_button = Paragraph::new(UiText::BTN_NO_CANCEL)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(no_border_style),
            )
            .style(no_style)
            .alignment(Alignment::Center);
        f.render_widget(no_button, button_chunks[1]);

        // Yes/Confirm button (index 3)
        let yes_selected = state.selected == 1;
        let yes_style = if yes_selected {
            Theme::severity_button_active(severity)
        } else {
            Theme::severity_button_inactive(severity)
        };
        let yes_border_style = if yes_selected {
            Style::default().fg(border_color)
        } else {
            Style::default().fg(Colors::BORDER_INACTIVE)
        };
        let yes_text = match state.severity {
            crate::app::ConfirmSeverity::Info => UiText::BTN_YES_CONTINUE,
            crate::app::ConfirmSeverity::Warning => UiText::BTN_YES_PROCEED,
            crate::app::ConfirmSeverity::Danger => UiText::BTN_CONFIRM_DELETE,
        };
        let yes_button = Paragraph::new(yes_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(yes_border_style),
            )
            .style(yes_style)
            .alignment(Alignment::Center);
        f.render_widget(yes_button, button_chunks[3]);
    }
}
