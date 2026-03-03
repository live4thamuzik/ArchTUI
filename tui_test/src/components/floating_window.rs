//! Floating window component (adapted for tui_test)
//!
//! Redesigned: rounded borders, scrollbar, output line coloring, embedded titles.

#![allow(dead_code)]

use crate::scrolling::ScrollState;
use crate::theme::Colors;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Clear, Gauge, List, ListItem, Paragraph, Scrollbar,
        ScrollbarOrientation, ScrollbarState, Wrap,
    },
    Frame,
};

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

pub struct FloatingWindow {
    config: FloatingWindowConfig,
    scroll_state: ScrollState,
}

impl FloatingWindow {
    pub fn new(config: FloatingWindowConfig) -> Self {
        Self {
            config,
            scroll_state: ScrollState::new(0, 10),
        }
    }

    pub fn with_title(title: &str) -> Self {
        let config = FloatingWindowConfig {
            title: title.to_string(),
            ..Default::default()
        };
        Self::new(config)
    }

    pub fn calculate_area(&self, parent: Rect) -> Rect {
        let width = ((parent.width as u32 * self.config.width_percent as u32) / 100) as u16;
        let height = ((parent.height as u32 * self.config.height_percent as u32) / 100) as u16;
        let width = width.clamp(self.config.min_width, self.config.max_width);
        let height = height.clamp(self.config.min_height, self.config.max_height);
        let width = width.min(parent.width.saturating_sub(2));
        let height = height.min(parent.height.saturating_sub(2));
        let x = parent.x + (parent.width.saturating_sub(width)) / 2;
        let y = parent.y + (parent.height.saturating_sub(height)) / 2;
        Rect::new(x, y, width, height)
    }

    /// Style a line based on its content (ERROR, WARNING, SUCCESS, phase markers)
    fn style_output_line(line: &str) -> Style {
        if line.contains("ERROR") || line.contains("error") || line.contains("FATAL") {
            Style::default().fg(Colors::ERROR)
        } else if line.contains("WARNING") || line.contains("warning") || line.contains("WARN:") {
            Style::default().fg(Colors::WARNING)
        } else if line.contains("SUCCESS") {
            Style::default().fg(Colors::SUCCESS)
        } else if line.starts_with("==>") || line.starts_with("::") || line.contains("Phase ") {
            Style::default()
                .fg(Colors::INFO)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Colors::FG_PRIMARY)
        }
    }

    /// Build the standard embedded-title block
    fn make_block(&self) -> Block<'_> {
        if self.config.has_border {
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(Line::from(vec![
                    Span::styled("\u{2500}", Style::default().fg(Colors::BORDER_ACTIVE)),
                    Span::styled(
                        format!(" {} ", self.config.title),
                        Style::default()
                            .fg(Colors::PRIMARY)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled("\u{2500}", Style::default().fg(Colors::BORDER_ACTIVE)),
                ]))
                .border_style(Style::default().fg(Colors::BORDER_ACTIVE))
                .style(Style::default().bg(Colors::BG_PRIMARY))
        } else {
            Block::default().style(Style::default().bg(Colors::BG_PRIMARY))
        }
    }

    /// Render a scrollbar on the right edge of an area
    fn render_scrollbar(f: &mut Frame, area: Rect, total: usize, position: usize) {
        if total <= area.height.saturating_sub(2) as usize {
            return;
        }
        let mut scrollbar_state = ScrollbarState::new(total).position(position);
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None)
            .track_symbol(Some("\u{2502}"))
            .thumb_symbol("\u{2588}")
            .track_style(Style::default().fg(Colors::SCROLLBAR_TRACK))
            .thumb_style(Style::default().fg(Colors::SCROLLBAR_THUMB));
        f.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
    }

    pub fn render_text(
        &self,
        f: &mut Frame,
        parent: Rect,
        content: &[String],
        footer: Option<&str>,
    ) {
        let area = self.calculate_area(parent);
        f.render_widget(Clear, area);

        let has_footer = footer.is_some();
        let chunks = if has_footer {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(1), Constraint::Length(1)])
                .split(area)
        } else {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(1)])
                .split(area)
        };

        let content_area = chunks[0];

        // Position indicator in bottom-right
        let mut block = self.make_block();
        if self.config.show_scroll_indicator && content.len() > content_area.height as usize {
            let visible = content_area.height.saturating_sub(2) as usize;
            let current_page = (self.scroll_state.offset / visible.max(1)) + 1;
            let total_pages = (content.len() / visible.max(1)) + 1;
            block = block.title_bottom(
                Line::from(vec![Span::styled(
                    format!(" {}/{} ", current_page, total_pages),
                    Style::default().fg(Colors::FG_MUTED),
                )])
                .alignment(Alignment::Right),
            );
        }

        let inner_area = block.inner(content_area);
        f.render_widget(block, content_area);

        let visible_height = inner_area.height as usize;
        let max_offset = content.len().saturating_sub(visible_height);
        let start = self.scroll_state.offset.min(max_offset);
        let end = (start + visible_height).min(content.len());

        let pad_width = inner_area.width as usize;
        let visible_content: Vec<ListItem> = content[start..end]
            .iter()
            .map(|line| {
                let padded = format!("{:<pad_width$}", line);
                ListItem::new(padded).style(Self::style_output_line(line))
            })
            .collect();

        let list = List::new(visible_content);
        f.render_widget(list, inner_area);

        // Scrollbar
        Self::render_scrollbar(f, content_area, content.len(), start);

        if let Some(footer_text) = footer {
            let footer_line = Line::from(vec![Span::styled(
                footer_text,
                Style::default().fg(Colors::FG_MUTED),
            )]);
            let footer_para = Paragraph::new(footer_line)
                .style(Style::default().bg(Colors::BG_SECONDARY))
                .alignment(Alignment::Center);
            f.render_widget(footer_para, chunks[1]);
        }
    }

    pub fn render_lines(
        &self,
        f: &mut Frame,
        parent: Rect,
        content: &[Line],
        footer: Option<&str>,
    ) {
        let area = self.calculate_area(parent);
        f.render_widget(Clear, area);

        let has_footer = footer.is_some();
        let chunks = if has_footer {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(1), Constraint::Length(1)])
                .split(area)
        } else {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(1)])
                .split(area)
        };

        let content_area = chunks[0];
        let block = self.make_block();

        let inner_area = block.inner(content_area);
        f.render_widget(block, content_area);

        let visible_height = inner_area.height as usize;
        let max_offset = content.len().saturating_sub(visible_height);
        let start = self.scroll_state.offset.min(max_offset);
        let end = (start + visible_height).min(content.len());
        let visible_lines: Vec<Line> = content[start..end].to_vec();

        let paragraph = Paragraph::new(visible_lines)
            .style(Style::default())
            .wrap(Wrap { trim: false });
        f.render_widget(paragraph, inner_area);

        // Scrollbar
        Self::render_scrollbar(f, content_area, content.len(), start);

        if let Some(footer_text) = footer {
            let footer_line = Line::from(vec![Span::styled(
                footer_text,
                Style::default().fg(Colors::FG_MUTED),
            )]);
            let footer_para = Paragraph::new(footer_line)
                .style(Style::default().bg(Colors::BG_SECONDARY))
                .alignment(Alignment::Center);
            f.render_widget(footer_para, chunks[1]);
        }
    }

    pub fn render_with_progress(
        &self,
        f: &mut Frame,
        parent: Rect,
        content: &[String],
        progress: u8,
        status: &str,
    ) {
        let area = self.calculate_area(parent);
        f.render_widget(Clear, area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Gauge
                Constraint::Min(1),   // Content
                Constraint::Length(2), // Status bar
            ])
            .split(area);

        // Progress gauge with embedded title
        let gauge = Gauge::default()
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .title(Line::from(vec![
                        Span::styled(
                            "\u{2500}",
                            Style::default().fg(Colors::BORDER_ACTIVE),
                        ),
                        Span::styled(
                            format!(" {} ", self.config.title),
                            Style::default()
                                .fg(Colors::PRIMARY)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(
                            "\u{2500}",
                            Style::default().fg(Colors::BORDER_ACTIVE),
                        ),
                    ]))
                    .border_style(Style::default().fg(Colors::BORDER_ACTIVE))
                    .style(Style::default().bg(Colors::BG_PRIMARY)),
            )
            .gauge_style(Style::default().fg(Colors::SUCCESS).bg(Colors::BG_GAUGE))
            .percent(progress as u16)
            .label(format!("{}%", progress));
        f.render_widget(gauge, chunks[0]);

        // Content area with side borders
        let content_block = Block::default()
            .borders(Borders::LEFT | Borders::RIGHT)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Colors::BORDER_ACTIVE))
            .style(Style::default().bg(Colors::BG_PRIMARY));
        let inner_area = content_block.inner(chunks[1]);
        f.render_widget(content_block, chunks[1]);

        let visible_height = inner_area.height as usize;
        let max_offset = content.len().saturating_sub(visible_height);
        let start = self.scroll_state.offset.min(max_offset);
        let end = (start + visible_height).min(content.len());

        let pad_width = inner_area.width as usize;
        let visible_content: Vec<ListItem> = content[start..end]
            .iter()
            .map(|line| {
                let padded = format!("{:<pad_width$}", line);
                ListItem::new(padded).style(Self::style_output_line(line))
            })
            .collect();

        let list = List::new(visible_content);
        f.render_widget(list, inner_area);

        // Status bar at bottom with rounded border
        let status_block = Block::default()
            .borders(Borders::LEFT | Borders::RIGHT | Borders::BOTTOM)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Colors::BORDER_ACTIVE))
            .style(Style::default().bg(Colors::BG_SECONDARY));
        let status_inner = status_block.inner(chunks[2]);
        f.render_widget(status_block, chunks[2]);

        let status_color = if progress >= 100 {
            if status.contains("Error") || status.contains("failed") {
                Colors::ERROR
            } else {
                Colors::SUCCESS
            }
        } else {
            Colors::WARNING
        };
        let status_para = Paragraph::new(Line::from(vec![
            Span::styled(
                " Status: ",
                Style::default()
                    .fg(Colors::FG_MUTED)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(status, Style::default().fg(status_color)),
        ]))
        .alignment(Alignment::Center);
        f.render_widget(status_para, status_inner);
    }

    pub fn set_scroll_offset(&mut self, offset: usize) {
        self.scroll_state.offset = offset;
    }

    pub fn scroll_up(&mut self) {
        self.scroll_state.move_up();
    }

    pub fn scroll_down(&mut self, content_len: usize) {
        self.scroll_state.total_items = content_len;
        self.scroll_state.move_down();
    }

    pub fn page_up(&mut self) {
        let page = self.scroll_state.visible_items;
        self.scroll_state.offset = self.scroll_state.offset.saturating_sub(page);
    }

    pub fn page_down(&mut self, content_len: usize) {
        let page = self.scroll_state.visible_items;
        self.scroll_state.offset = (self.scroll_state.offset + page).min(content_len.saturating_sub(1));
    }

    pub fn scroll_to_top(&mut self) {
        self.scroll_state.offset = 0;
    }

    pub fn scroll_to_bottom(&mut self, content_len: usize) {
        let visible = self.scroll_state.visible_items;
        self.scroll_state.offset = content_len.saturating_sub(visible);
    }

    pub fn update_visible_items(&mut self, visible: usize) {
        self.scroll_state.visible_items = visible;
    }

    pub fn scroll_offset(&self) -> usize {
        self.scroll_state.offset
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
    pub fn new(title: &str) -> Self {
        Self {
            title: title.to_string(),
            ..Default::default()
        }
    }

    pub fn append_line(&mut self, line: String) {
        self.content.push(line);
        if self.content.len() > 1000 {
            self.content.remove(0);
        }
    }

    pub fn set_progress(&mut self, progress: u8) {
        self.progress = Some(progress.min(100));
    }

    pub fn mark_complete(&mut self) {
        self.complete = true;
        self.progress = Some(100);
    }
}
