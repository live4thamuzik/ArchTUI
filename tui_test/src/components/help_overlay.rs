//! Help overlay component (adapted for tui_test)
//!
//! Redesigned: uses FloatingWindow with styled key/action pairs matching nav bar.

#![allow(dead_code)]

use super::floating_window::{FloatingWindow, FloatingWindowConfig};
use super::keybindings::{HelpSection, KeybindingContext};
use crate::app::AppMode;
use crate::theme::Colors;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    Frame,
};

pub struct HelpOverlay {
    window: FloatingWindow,
    content: Vec<Line<'static>>,
}

impl HelpOverlay {
    pub fn new(mode: &AppMode, keybinding_ctx: &KeybindingContext) -> Self {
        let config = FloatingWindowConfig {
            title: "Help".to_string(),
            width_percent: 60,
            height_percent: 70,
            min_width: 50,
            min_height: 15,
            max_width: 80,
            max_height: 35,
            has_border: true,
            scrollable: true,
            show_scroll_indicator: true,
        };

        let sections = keybinding_ctx.get_help_content(mode);
        let content = Self::build_content(&sections, mode);

        Self {
            window: FloatingWindow::new(config),
            content,
        }
    }

    fn build_content(sections: &[HelpSection], mode: &AppMode) -> Vec<Line<'static>> {
        let mut lines: Vec<Line<'static>> = Vec::new();

        // Title
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled(
                "  ArchTUI Help",
                Style::default()
                    .fg(Colors::PRIMARY)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
        lines.push(Line::from(""));

        // Current mode
        let mode_name = match mode {
            AppMode::MainMenu => "Main Menu",
            AppMode::GuidedInstaller => "Guided Installer",
            AppMode::AutomatedInstall => "Automated Install",
            AppMode::ToolsMenu => "Tools Menu",
            AppMode::DiskTools => "Disk Tools",
            AppMode::SystemTools => "System Tools",
            AppMode::UserTools => "User Tools",
            AppMode::NetworkTools => "Network Tools",
            AppMode::ToolDialog => "Tool Configuration",
            AppMode::Installation => "Installation",
            AppMode::Complete => "Complete",
            AppMode::FloatingOutput => "Output View",
            AppMode::FileBrowser => "File Browser",
            AppMode::ConfirmDialog => "Confirmation",
            AppMode::DryRunSummary => "Dry Run Summary",
            AppMode::EmbeddedTerminal => "Embedded Terminal",
        };
        lines.push(Line::from(vec![
            Span::styled("  Context: ", Style::default().fg(Colors::FG_MUTED)),
            Span::styled(
                mode_name.to_string(),
                Style::default().fg(Colors::SECONDARY),
            ),
        ]));
        lines.push(Line::from(""));

        // Sections
        for section in sections {
            // Section header with accent underline
            lines.push(Line::from(vec![
                Span::styled(
                    format!("  \u{2500}\u{2500} {} ", section.title),
                    Style::default()
                        .fg(Colors::PRIMARY)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "\u{2500}".repeat(30),
                    Style::default().fg(Colors::BORDER_INACTIVE),
                ),
            ]));
            lines.push(Line::from(""));

            for (key, description) in &section.items {
                // Key styled like nav bar keys, action like nav bar actions
                lines.push(Line::from(vec![
                    Span::styled("    ", Style::default()),
                    Span::styled(
                        format!("{:<12}", key),
                        Style::default()
                            .fg(Colors::NAV_KEY)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        description.clone(),
                        Style::default().fg(Colors::FG_PRIMARY),
                    ),
                ]));
            }
            lines.push(Line::from(""));
        }

        // Footer hint
        lines.push(Line::from(vec![
            Span::styled("  Press ", Style::default().fg(Colors::FG_MUTED)),
            Span::styled(
                "?",
                Style::default()
                    .fg(Colors::SECONDARY)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" or ", Style::default().fg(Colors::FG_MUTED)),
            Span::styled(
                "Esc",
                Style::default()
                    .fg(Colors::SECONDARY)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" to close", Style::default().fg(Colors::FG_MUTED)),
        ]));

        lines
    }

    pub fn render(&self, f: &mut Frame, parent: Rect) {
        self.window.render_lines(
            f,
            parent,
            &self.content,
            Some("? or Esc to close"),
        );
    }
}
