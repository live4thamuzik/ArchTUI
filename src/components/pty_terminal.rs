//! PTY-based embedded terminal component
//!
//! Provides an embedded terminal widget for running interactive tools like cfdisk.

#![allow(dead_code)]

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use crate::theme::Colors;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::thread;

/// Result type for PTY operations
pub type PtyResult<T> = Result<T, PtyError>;

/// Errors that can occur during PTY operations
#[derive(Debug)]
pub enum PtyError {
    /// Failed to create PTY system
    SystemCreation(String),
    /// Failed to open PTY pair
    PtyOpen(String),
    /// Failed to spawn command
    Spawn(String),
    /// Failed to write to PTY
    Write(String),
    /// Failed to read from PTY
    Read(String),
    /// Failed to resize PTY
    Resize(String),
    /// PTY is not running
    NotRunning,
}

impl std::fmt::Display for PtyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PtyError::SystemCreation(e) => write!(f, "Failed to create PTY system: {}", e),
            PtyError::PtyOpen(e) => write!(f, "Failed to open PTY: {}", e),
            PtyError::Spawn(e) => write!(f, "Failed to spawn command: {}", e),
            PtyError::Write(e) => write!(f, "Failed to write to PTY: {}", e),
            PtyError::Read(e) => write!(f, "Failed to read from PTY: {}", e),
            PtyError::Resize(e) => write!(f, "Failed to resize PTY: {}", e),
            PtyError::NotRunning => write!(f, "PTY is not running"),
        }
    }
}

impl std::error::Error for PtyError {}

/// State for tracking embedded terminal
#[derive(Debug, Clone)]
pub struct PtyTerminalState {
    pub tool_name: String,
    pub return_mode: crate::app::AppMode,
    pub return_menu_selection: usize,
}

/// PTY-based embedded terminal
pub struct PtyTerminal {
    /// VT100 parser for terminal emulation
    parser: vt100::Parser,
    /// Terminal size
    size: PtySize,
    /// Output buffer shared with reader thread
    output_buffer: Arc<Mutex<Vec<u8>>>,
    /// Writer to send input to PTY
    writer: Option<Box<dyn Write + Send>>,
    /// Child process handle
    child: Option<Box<dyn portable_pty::Child + Send + Sync>>,
    /// Whether the terminal is still running
    running: Arc<Mutex<bool>>,
    /// Exit status when complete
    exit_status: Arc<Mutex<Option<portable_pty::ExitStatus>>>,
}

impl PtyTerminal {
    /// Create a new PTY terminal with the given size
    pub fn new(cols: u16, rows: u16) -> PtyResult<Self> {
        Ok(Self {
            parser: vt100::Parser::new(rows, cols, 1000), // 1000 lines scrollback
            size: PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            },
            output_buffer: Arc::new(Mutex::new(Vec::new())),
            writer: None,
            child: None,
            running: Arc::new(Mutex::new(false)),
            exit_status: Arc::new(Mutex::new(None)),
        })
    }

    /// Spawn a command in the PTY
    pub fn spawn_command(&mut self, cmd: &str, args: &[&str]) -> PtyResult<()> {
        // Create PTY system
        let pty_system = native_pty_system();

        // Open PTY pair
        let pair = pty_system
            .openpty(self.size)
            .map_err(|e| PtyError::PtyOpen(e.to_string()))?;

        // Build command
        let mut cmd_builder = CommandBuilder::new(cmd);
        for arg in args {
            cmd_builder.arg(*arg);
        }

        // Set environment variables for proper terminal behavior
        cmd_builder.env("TERM", "xterm-256color");
        cmd_builder.env("COLORTERM", "truecolor");

        // Spawn the command
        let child = pair
            .slave
            .spawn_command(cmd_builder)
            .map_err(|e| PtyError::Spawn(e.to_string()))?;

        // Get writer for input
        let writer = pair
            .master
            .take_writer()
            .map_err(|e| PtyError::Write(e.to_string()))?;

        // Get reader for output
        let mut reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| PtyError::Read(e.to_string()))?;

        // Set up shared state
        self.writer = Some(writer);
        self.child = Some(child);
        *self.running.lock().unwrap() = true;

        // Spawn reader thread
        let output_buffer = Arc::clone(&self.output_buffer);
        let running = Arc::clone(&self.running);

        thread::spawn(move || {
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => {
                        // EOF - process ended
                        *running.lock().unwrap() = false;
                        break;
                    }
                    Ok(n) => {
                        let mut buffer = output_buffer.lock().unwrap();
                        buffer.extend_from_slice(&buf[..n]);
                    }
                    Err(e) => {
                        log::warn!("PTY read error: {}", e);
                        *running.lock().unwrap() = false;
                        break;
                    }
                }
            }
        });

        Ok(())
    }

    /// Process any pending output and update the parser
    pub fn process_output(&mut self) {
        let data: Vec<u8> = {
            let mut buffer = self.output_buffer.lock().unwrap();
            std::mem::take(&mut *buffer)
        };

        if !data.is_empty() {
            self.parser.process(&data);
        }
    }

    /// Send input to the PTY
    pub fn send_input(&mut self, data: &[u8]) -> PtyResult<()> {
        if let Some(ref mut writer) = self.writer {
            writer
                .write_all(data)
                .map_err(|e| PtyError::Write(e.to_string()))?;
            writer.flush().map_err(|e| PtyError::Write(e.to_string()))?;
            Ok(())
        } else {
            Err(PtyError::NotRunning)
        }
    }

    /// Send a key event to the PTY
    pub fn send_key(&mut self, key: KeyEvent) -> PtyResult<()> {
        let bytes = key_event_to_bytes(key);
        if !bytes.is_empty() {
            self.send_input(&bytes)?;
        }
        Ok(())
    }

    /// Check if the terminal is still running
    pub fn is_running(&mut self) -> bool {
        // First check our flag
        let flag = *self.running.lock().unwrap();
        if !flag {
            return false;
        }

        // Also check child process status
        if let Some(ref mut child) = self.child {
            // try_wait returns Ok(Some(status)) if exited, Ok(None) if still running
            match child.try_wait() {
                Ok(Some(status)) => {
                    *self.exit_status.lock().unwrap() = Some(status);
                    *self.running.lock().unwrap() = false;
                    false
                }
                Ok(None) => true,
                Err(_) => {
                    *self.running.lock().unwrap() = false;
                    false
                }
            }
        } else {
            false
        }
    }

    /// Get the exit status if the process has completed
    pub fn exit_status(&self) -> Option<portable_pty::ExitStatus> {
        self.exit_status.lock().unwrap().clone()
    }

    /// Resize the PTY
    pub fn resize(&mut self, cols: u16, rows: u16) -> PtyResult<()> {
        self.size = PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        };
        self.parser.set_size(rows, cols);
        // Note: Resizing the actual PTY would require keeping a reference to the master
        // For now, we just update the parser size
        Ok(())
    }

    /// Render the terminal content
    pub fn render(&mut self, f: &mut Frame, area: Rect, title: &str) {
        // Process any pending output
        self.process_output();

        // Clear the area
        f.render_widget(Clear, area);

        // Create the terminal block
        let block = Block::default()
            .borders(Borders::ALL)
            .title(title)
            .title_style(
                Style::default()
                    .fg(Colors::PRIMARY)
                    .add_modifier(Modifier::BOLD),
            )
            .border_style(Style::default().fg(Colors::PRIMARY))
            .style(Style::default().bg(Colors::BG_PRIMARY));

        let inner = block.inner(area);
        f.render_widget(block, area);

        // Get the screen content from the parser
        let screen = self.parser.screen();
        let mut lines: Vec<Line> = Vec::new();

        for row in 0..inner.height {
            let mut spans: Vec<Span> = Vec::new();
            for col in 0..inner.width {
                let cell = screen.cell(row, col);
                if let Some(cell) = cell {
                    let contents = cell.contents();
                    let ch = if contents.is_empty() {
                        " ".to_string()
                    } else {
                        contents.to_string()
                    };

                    // Convert vt100 colors to ratatui colors
                    let fg = convert_color(cell.fgcolor());
                    let bg = convert_color(cell.bgcolor());

                    let mut style = Style::default().fg(fg).bg(bg);

                    if cell.bold() {
                        style = style.add_modifier(Modifier::BOLD);
                    }
                    if cell.italic() {
                        style = style.add_modifier(Modifier::ITALIC);
                    }
                    if cell.underline() {
                        style = style.add_modifier(Modifier::UNDERLINED);
                    }
                    if cell.inverse() {
                        style = Style::default().fg(bg).bg(fg);
                    }

                    spans.push(Span::styled(ch, style));
                } else {
                    spans.push(Span::raw(" "));
                }
            }
            lines.push(Line::from(spans));
        }

        let paragraph = Paragraph::new(lines);
        f.render_widget(paragraph, inner);

        // Render cursor position
        let cursor_pos = screen.cursor_position();
        let cursor_x = inner.x + cursor_pos.1;
        let cursor_y = inner.y + cursor_pos.0;
        if cursor_x < inner.x + inner.width && cursor_y < inner.y + inner.height {
            f.set_cursor_position((cursor_x, cursor_y));
        }
    }

    /// Kill the PTY process
    pub fn kill(&mut self) {
        if let Some(ref mut child) = self.child {
            let _ = child.kill();
        }
        *self.running.lock().unwrap() = false;
    }
}

impl Drop for PtyTerminal {
    fn drop(&mut self) {
        self.kill();
    }
}

/// Convert a vt100 color to a ratatui color
fn convert_color(color: vt100::Color) -> Color {
    match color {
        vt100::Color::Default => Color::Reset,
        vt100::Color::Idx(0) => Color::Black,
        vt100::Color::Idx(1) => Color::Red,
        vt100::Color::Idx(2) => Color::Green,
        vt100::Color::Idx(3) => Color::Yellow,
        vt100::Color::Idx(4) => Color::Blue,
        vt100::Color::Idx(5) => Color::Magenta,
        vt100::Color::Idx(6) => Color::Cyan,
        vt100::Color::Idx(7) => Color::White,
        vt100::Color::Idx(8) => Color::DarkGray,
        vt100::Color::Idx(9) => Color::LightRed,
        vt100::Color::Idx(10) => Color::LightGreen,
        vt100::Color::Idx(11) => Color::LightYellow,
        vt100::Color::Idx(12) => Color::LightBlue,
        vt100::Color::Idx(13) => Color::LightMagenta,
        vt100::Color::Idx(14) => Color::LightCyan,
        vt100::Color::Idx(15) => Color::White,
        vt100::Color::Idx(idx) => Color::Indexed(idx),
        vt100::Color::Rgb(r, g, b) => Color::Rgb(r, g, b),
    }
}

/// Convert a key event to bytes for the PTY
fn key_event_to_bytes(key: KeyEvent) -> Vec<u8> {
    let mut bytes = Vec::new();

    // Handle modifiers
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    let alt = key.modifiers.contains(KeyModifiers::ALT);

    match key.code {
        KeyCode::Char(c) => {
            if ctrl {
                // Control characters
                let c = c.to_ascii_lowercase();
                if c.is_ascii_lowercase() {
                    bytes.push((c as u8) - b'a' + 1);
                }
            } else if alt {
                bytes.push(0x1b);
                bytes.push(c as u8);
            } else {
                let mut buf = [0u8; 4];
                let s = c.encode_utf8(&mut buf);
                bytes.extend_from_slice(s.as_bytes());
            }
        }
        KeyCode::Enter => bytes.push(b'\r'),
        KeyCode::Backspace => bytes.push(0x7f),
        KeyCode::Tab => bytes.push(b'\t'),
        KeyCode::Esc => bytes.push(0x1b),
        KeyCode::Up => bytes.extend_from_slice(b"\x1b[A"),
        KeyCode::Down => bytes.extend_from_slice(b"\x1b[B"),
        KeyCode::Right => bytes.extend_from_slice(b"\x1b[C"),
        KeyCode::Left => bytes.extend_from_slice(b"\x1b[D"),
        KeyCode::Home => bytes.extend_from_slice(b"\x1b[H"),
        KeyCode::End => bytes.extend_from_slice(b"\x1b[F"),
        KeyCode::PageUp => bytes.extend_from_slice(b"\x1b[5~"),
        KeyCode::PageDown => bytes.extend_from_slice(b"\x1b[6~"),
        KeyCode::Insert => bytes.extend_from_slice(b"\x1b[2~"),
        KeyCode::Delete => bytes.extend_from_slice(b"\x1b[3~"),
        KeyCode::F(1) => bytes.extend_from_slice(b"\x1bOP"),
        KeyCode::F(2) => bytes.extend_from_slice(b"\x1bOQ"),
        KeyCode::F(3) => bytes.extend_from_slice(b"\x1bOR"),
        KeyCode::F(4) => bytes.extend_from_slice(b"\x1bOS"),
        KeyCode::F(5) => bytes.extend_from_slice(b"\x1b[15~"),
        KeyCode::F(6) => bytes.extend_from_slice(b"\x1b[17~"),
        KeyCode::F(7) => bytes.extend_from_slice(b"\x1b[18~"),
        KeyCode::F(8) => bytes.extend_from_slice(b"\x1b[19~"),
        KeyCode::F(9) => bytes.extend_from_slice(b"\x1b[20~"),
        KeyCode::F(10) => bytes.extend_from_slice(b"\x1b[21~"),
        KeyCode::F(11) => bytes.extend_from_slice(b"\x1b[23~"),
        KeyCode::F(12) => bytes.extend_from_slice(b"\x1b[24~"),
        _ => {}
    }

    bytes
}

/// Try to spawn a PTY terminal, or return an indication to use fallback
pub enum PtySpawnResult {
    /// Successfully created PTY terminal
    Success(Box<PtyTerminal>),
    /// Failed, should use fallback passthrough mode
    Fallback(String),
}

/// Attempt to spawn a command in a PTY, with fallback support
pub fn spawn_or_fallback(cmd: &str, args: &[&str], cols: u16, rows: u16) -> PtySpawnResult {
    match PtyTerminal::new(cols, rows) {
        Ok(mut pty) => match pty.spawn_command(cmd, args) {
            Ok(()) => PtySpawnResult::Success(Box::new(pty)),
            Err(e) => PtySpawnResult::Fallback(format!("Failed to spawn: {}", e)),
        },
        Err(e) => PtySpawnResult::Fallback(format!("Failed to create PTY: {}", e)),
    }
}
