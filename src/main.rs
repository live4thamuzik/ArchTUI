//! ArchInstall TUI - Main entry point
//!
//! A clean, modular TUI for Arch Linux installation with proper separation of concerns.

mod app;
mod config;
mod error;
mod input;
mod installer;
mod package_utils;
mod scrolling;
mod ui;

use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::stdout;

/// Main application entry point
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize terminal
    enable_raw_mode()
        .map_err(|e| error::general_error(format!("Failed to enable raw mode: {}", e)))?;
    crossterm::execute!(stdout(), crossterm::terminal::EnterAlternateScreen)
        .map_err(|e| error::general_error(format!("Failed to enter alternate screen: {}", e)))?;

    // Create terminal backend
    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend)
        .map_err(|e| error::general_error(format!("Failed to create terminal: {}", e)))?;

    // Create and run application
    let mut app = app::App::new();
    let result = app.run(&mut terminal);

    // Cleanup terminal (always attempt cleanup, even if app failed)
    let _ = disable_raw_mode();
    let _ = crossterm::execute!(stdout(), crossterm::terminal::LeaveAlternateScreen);

    result
}
