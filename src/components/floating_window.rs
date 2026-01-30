//! Floating window component
//!
//! A reusable centered overlay window for dialogs, output display, and help.

#![allow(dead_code)]

use crate::scrolling::ScrollState;
use crate::theme::Colors;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::Line,
    widgets::{Block, Borders, Clear, Gauge, List, ListItem, Paragraph, Wrap},
    Frame,
};

/// Configuration for a floating window
#[derive(Debug, Clone)]
pub struct FloatingWindowConfig {
    pub title: String,
    pub width_percent: u16,
    pub height_percent: u16,
    pub min_width: u16,
    pub min_height: u16,
    pub max_width: u16,
    pub max_height: u16,
    pub has_border: bool,
    pub scrollable: bool,
    pub show_scroll_indicator: bool,
}

impl Default for FloatingWindowConfig {
    fn default() -> Self {
        Self {
            title: String::new(),
            width_percent: 80,
            height_percent: 70,
            min_width: 40,
            min_height: 10,
            max_width: 120,
            max_height: 40,
            has_border: true,
            scrollable: true,
            show_scroll_indicator: true,
        }
    }
}

/// Floating window component
pub struct FloatingWindow {
    config: FloatingWindowConfig,
    scroll_state: ScrollState,
}

impl FloatingWindow {
    /// Create a new floating window with the given configuration
    pub fn new(config: FloatingWindowConfig) -> Self {
        Self {
            config,
            scroll_state: ScrollState::new(0, 10),
        }
    }

    /// Create a floating window with default configuration and a title
    pub fn with_title(title: &str) -> Self {
        let config = FloatingWindowConfig {
            title: title.to_string(),
            ..Default::default()
        };
        Self::new(config)
    }

    /// Calculate the centered area for this window
    pub fn calculate_area(&self, parent: Rect) -> Rect {
        let width = ((parent.width as u32 * self.config.width_percent as u32) / 100) as u16;
        let height = ((parent.height as u32 * self.config.height_percent as u32) / 100) as u16;

        let width = width.clamp(self.config.min_width, self.config.max_width);
        let height = height.clamp(self.config.min_height, self.config.max_height);

        // Ensure we don't exceed parent bounds
        let width = width.min(parent.width.saturating_sub(2));
        let height = height.min(parent.height.saturating_sub(2));

        let x = parent.x + (parent.width.saturating_sub(width)) / 2;
        let y = parent.y + (parent.height.saturating_sub(height)) / 2;

        Rect::new(x, y, width, height)
    }

    /// Render the floating window with text content
    pub fn render_text(&self, f: &mut Frame, parent: Rect, content: &[String], footer: Option<&str>) {
        let area = self.calculate_area(parent);

        // Clear the area behind the window
        f.render_widget(Clear, area);

        // Draw background
        let bg_block = Block::default()
            .style(Style::default().bg(Colors::BG_PRIMARY));
        f.render_widget(bg_block, area);

        // Create layout
        let has_footer = footer.is_some();
        let chunks = if has_footer {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(1),    // Content
                    Constraint::Length(1), // Footer
                ])
                .split(area)
        } else {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(1)])
                .split(area)
        };

        // Draw content area with border
        let content_area = chunks[0];
        let title = if self.config.show_scroll_indicator && content.len() > content_area.height as usize {
            let visible = content_area.height.saturating_sub(2) as usize;
            let current_page = (self.scroll_state.offset / visible.max(1)) + 1;
            let total_pages = (content.len() / visible.max(1)) + 1;
            format!("{} ({}/{})", self.config.title, current_page, total_pages)
        } else {
            self.config.title.clone()
        };

        let block = if self.config.has_border {
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .title_style(Style::default().fg(Colors::PRIMARY).add_modifier(Modifier::BOLD))
                .border_style(Style::default().fg(Colors::PRIMARY))
                .style(Style::default().bg(Colors::BG_PRIMARY))
        } else {
            Block::default().style(Style::default().bg(Colors::BG_PRIMARY))
        };

        // Calculate inner area for content
        let inner_area = block.inner(content_area);

        // Render the block
        f.render_widget(block, content_area);

        // Render scrollable content
        let visible_height = inner_area.height as usize;
        let start = self.scroll_state.offset;
        let end = (start + visible_height).min(content.len());

        let visible_content: Vec<ListItem> = content[start..end]
            .iter()
            .map(|line| ListItem::new(line.as_str()))
            .collect();

        let list = List::new(visible_content).style(Style::default().fg(Colors::FG_PRIMARY));
        f.render_widget(list, inner_area);

        // Render footer if present
        if let Some(footer_text) = footer {
            let footer_para = Paragraph::new(footer_text)
                .style(Style::default().fg(Colors::FG_MUTED))
                .alignment(Alignment::Center);
            f.render_widget(footer_para, chunks[1]);
        }
    }

    /// Render the floating window with styled lines
    pub fn render_lines(&self, f: &mut Frame, parent: Rect, content: &[Line], footer: Option<&str>) {
        let area = self.calculate_area(parent);

        // Clear the area behind the window
        f.render_widget(Clear, area);

        // Draw background
        let bg_block = Block::default()
            .style(Style::default().bg(Colors::BG_PRIMARY));
        f.render_widget(bg_block, area);

        // Create layout
        let has_footer = footer.is_some();
        let chunks = if has_footer {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(1),
                    Constraint::Length(1),
                ])
                .split(area)
        } else {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(1)])
                .split(area)
        };

        let content_area = chunks[0];

        let block = if self.config.has_border {
            Block::default()
                .borders(Borders::ALL)
                .title(self.config.title.clone())
                .title_style(Style::default().fg(Colors::PRIMARY).add_modifier(Modifier::BOLD))
                .border_style(Style::default().fg(Colors::PRIMARY))
                .style(Style::default().bg(Colors::BG_PRIMARY))
        } else {
            Block::default().style(Style::default().bg(Colors::BG_PRIMARY))
        };

        let inner_area = block.inner(content_area);
        f.render_widget(block, content_area);

        // Render content
        let visible_height = inner_area.height as usize;
        let start = self.scroll_state.offset;
        let end = (start + visible_height).min(content.len());

        let visible_lines: Vec<Line> = content[start..end].to_vec();
        let paragraph = Paragraph::new(visible_lines)
            .style(Style::default())
            .wrap(Wrap { trim: false });
        f.render_widget(paragraph, inner_area);

        // Render footer if present
        if let Some(footer_text) = footer {
            let footer_para = Paragraph::new(footer_text)
                .style(Style::default().fg(Colors::FG_MUTED))
                .alignment(Alignment::Center);
            f.render_widget(footer_para, chunks[1]);
        }
    }

    /// Render the floating window with a progress bar (for installation)
    pub fn render_with_progress(
        &self,
        f: &mut Frame,
        parent: Rect,
        content: &[String],
        progress: u8,
        status: &str,
    ) {
        let area = self.calculate_area(parent);

        // Clear the area behind the window
        f.render_widget(Clear, area);

        // Draw background
        let bg_block = Block::default()
            .style(Style::default().bg(Colors::BG_PRIMARY));
        f.render_widget(bg_block, area);

        // Create layout with progress bar
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Progress bar
                Constraint::Min(1),    // Content
                Constraint::Length(1), // Status
            ])
            .split(area);

        // Render progress bar
        let gauge = Gauge::default()
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(self.config.title.clone())
                    .title_style(Style::default().fg(Colors::PRIMARY).add_modifier(Modifier::BOLD))
                    .border_style(Style::default().fg(Colors::PRIMARY))
                    .style(Style::default().bg(Colors::BG_PRIMARY)),
            )
            .gauge_style(Style::default().fg(Colors::SUCCESS).bg(Colors::BG_SECONDARY))
            .percent(progress as u16)
            .label(format!("{}%", progress));
        f.render_widget(gauge, chunks[0]);

        // Render content
        let content_block = Block::default()
            .borders(Borders::LEFT | Borders::RIGHT)
            .border_style(Style::default().fg(Colors::PRIMARY))
            .style(Style::default().bg(Colors::BG_PRIMARY));
        let inner_area = content_block.inner(chunks[1]);
        f.render_widget(content_block, chunks[1]);

        let visible_height = inner_area.height as usize;
        let start = if content.len() > visible_height {
            content.len() - visible_height // Auto-scroll to bottom
        } else {
            0
        };
        let end = content.len();

        let visible_content: Vec<ListItem> = content[start..end]
            .iter()
            .map(|line| {
                let style = if line.contains("ERROR") || line.contains("error") {
                    Style::default().fg(Colors::ERROR)
                } else if line.contains("WARNING") || line.contains("warning") {
                    Style::default().fg(Colors::WARNING)
                } else if line.starts_with("==>") || line.starts_with("::") {
                    Style::default().fg(Colors::INFO).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Colors::FG_PRIMARY)
                };
                ListItem::new(line.as_str()).style(style)
            })
            .collect();

        let list = List::new(visible_content);
        f.render_widget(list, inner_area);

        // Render status
        let status_block = Block::default()
            .borders(Borders::LEFT | Borders::RIGHT | Borders::BOTTOM)
            .border_style(Style::default().fg(Colors::PRIMARY))
            .style(Style::default().bg(Colors::BG_PRIMARY));
        let status_inner = status_block.inner(chunks[2]);
        f.render_widget(status_block, chunks[2]);

        let status_para = Paragraph::new(status)
            .style(Style::default().fg(Colors::WARNING))
            .alignment(Alignment::Center);
        f.render_widget(status_para, status_inner);
    }

    /// Scroll up by one line
    pub fn scroll_up(&mut self) {
        self.scroll_state.move_up();
    }

    /// Scroll down by one line
    pub fn scroll_down(&mut self, content_len: usize) {
        self.scroll_state.total_items = content_len;
        self.scroll_state.move_down();
    }

    /// Page up
    pub fn page_up(&mut self) {
        self.scroll_state.page_up();
    }

    /// Page down
    pub fn page_down(&mut self, content_len: usize) {
        self.scroll_state.total_items = content_len;
        self.scroll_state.page_down();
    }

    /// Scroll to top
    pub fn scroll_to_top(&mut self) {
        self.scroll_state.move_to_first();
    }

    /// Scroll to bottom
    pub fn scroll_to_bottom(&mut self, content_len: usize) {
        self.scroll_state.total_items = content_len;
        self.scroll_state.move_to_last();
    }

    /// Update visible items count (call when window is resized)
    pub fn update_visible_items(&mut self, visible: usize) {
        self.scroll_state.visible_items = visible;
    }

    /// Get current scroll offset
    pub fn scroll_offset(&self) -> usize {
        self.scroll_state.offset
    }

    /// Set scroll offset
    pub fn set_scroll_offset(&mut self, offset: usize) {
        self.scroll_state.offset = offset;
    }
}

/// State for a floating output window
#[derive(Debug, Clone)]
pub struct FloatingOutputState {
    pub title: String,
    pub content: Vec<String>,
    pub scroll_offset: usize,
    pub auto_scroll: bool,
    pub complete: bool,
    pub progress: Option<u8>,
    pub status: String,
}

impl Default for FloatingOutputState {
    fn default() -> Self {
        Self {
            title: "Output".to_string(),
            content: Vec::new(),
            scroll_offset: 0,
            auto_scroll: true,
            complete: false,
            progress: None,
            status: String::new(),
        }
    }
}

impl FloatingOutputState {
    /// Create a new floating output state with a title
    pub fn new(title: &str) -> Self {
        Self {
            title: title.to_string(),
            ..Default::default()
        }
    }

    /// Append a line to the content
    pub fn append_line(&mut self, line: String) {
        self.content.push(line);
        // Keep a reasonable buffer size
        if self.content.len() > 1000 {
            self.content.remove(0);
        }
    }

    /// Set progress percentage
    pub fn set_progress(&mut self, progress: u8) {
        self.progress = Some(progress.min(100));
    }

    /// Mark as complete
    pub fn mark_complete(&mut self) {
        self.complete = true;
        self.progress = Some(100);
    }
}
