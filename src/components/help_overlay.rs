//! Help overlay component
//!
//! Displays context-sensitive help using a floating window.

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

/// Help overlay component
pub struct HelpOverlay {
    window: FloatingWindow,
    content: Vec<Line<'static>>,
}

impl HelpOverlay {
    /// Create a new help overlay for the given mode
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
            show_scroll_indicator: false,
        };

        let sections = keybinding_ctx.get_help_content(mode);
        let content = Self::build_content(&sections, mode);

        Self {
            window: FloatingWindow::new(config),
            content,
        }
    }

    /// Build the help content from sections
    fn build_content(sections: &[HelpSection], mode: &AppMode) -> Vec<Line<'static>> {
        let mut lines: Vec<Line<'static>> = Vec::new();

        // Header
        lines.push(Line::from(vec![Span::styled(
            "  Arch Linux Toolkit Help  ",
            Style::default()
                .fg(Colors::PRIMARY)
                .add_modifier(Modifier::BOLD),
        )]));
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
            AppMode::ToolExecution => "Tool Execution",
            AppMode::Installation => "Installation",
            AppMode::Complete => "Complete",
            AppMode::EmbeddedTerminal => "Terminal",
            AppMode::FloatingOutput => "Output View",
            AppMode::FileBrowser => "File Browser",
            AppMode::ConfirmDialog => "Confirmation",
            AppMode::DryRunSummary => "Dry Run Summary",
        };
        lines.push(Line::from(vec![
            Span::styled("Current: ", Style::default().fg(Colors::FG_MUTED)),
            Span::styled(
                mode_name.to_string(),
                Style::default().fg(Colors::SECONDARY),
            ),
        ]));
        lines.push(Line::from(""));

        // Sections
        for section in sections {
            // Section title
            lines.push(Line::from(vec![Span::styled(
                format!("  {}  ", section.title),
                Style::default()
                    .fg(Colors::SUCCESS)
                    .add_modifier(Modifier::BOLD),
            )]));
            lines.push(Line::from(""));

            // Items
            for (key, description) in &section.items {
                lines.push(Line::from(vec![
                    Span::styled("    ", Style::default()),
                    Span::styled(
                        format!("{:<10}", key),
                        Style::default()
                            .fg(Colors::PRIMARY)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(description.clone(), Style::default().fg(Colors::FG_PRIMARY)),
                ]));
            }
            lines.push(Line::from(""));
        }

        // Footer
        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled(
            "Press ? or Esc to close",
            Style::default().fg(Colors::FG_MUTED),
        )]));

        lines
    }

    /// Render the help overlay
    pub fn render(&self, f: &mut Frame, parent: Rect) {
        self.window.render_lines(
            f,
            parent,
            &self.content,
            Some("Press ? or Esc to close"),
        );
    }

    /// Update help content for a new mode
    pub fn update_mode(&mut self, mode: &AppMode, keybinding_ctx: &KeybindingContext) {
        let sections = keybinding_ctx.get_help_content(mode);
        self.content = Self::build_content(&sections, mode);
    }
}

/// Quick help builder for generating help content
pub fn build_quick_help(mode: &AppMode) -> Vec<String> {
    let keybinding_ctx = KeybindingContext::new();
    let sections = keybinding_ctx.get_help_content(mode);

    let mut lines = Vec::new();
    for section in sections {
        lines.push(format!("-- {} --", section.title));
        for (key, desc) in section.items {
            lines.push(format!("  {}: {}", key, desc));
        }
        lines.push(String::new());
    }
    lines
}
