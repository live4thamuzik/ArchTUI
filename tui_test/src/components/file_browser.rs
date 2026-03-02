//! File browser component for selecting configuration files
//!
//! Provides a TUI file browser for navigating directories and selecting files.

use std::fs;
use std::path::{Path, PathBuf};

/// File entry in the browser
#[derive(Debug, Clone)]
pub struct FileEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub size: u64,
}

impl FileEntry {
    /// Create a parent directory entry (..)
    pub fn parent_dir(parent_path: PathBuf) -> Self {
        Self {
            name: "..".to_string(),
            path: parent_path,
            is_dir: true,
            size: 0,
        }
    }
}

/// State for the file browser
#[derive(Debug, Clone)]
pub struct FileBrowserState {
    /// Current directory being browsed
    pub current_dir: PathBuf,
    /// List of entries in current directory
    pub entries: Vec<FileEntry>,
    /// Currently selected index
    pub selected: usize,
    /// File extension filter (e.g., vec!["toml", "json"])
    pub extensions: Vec<String>,
    /// Error message if any
    pub error: Option<String>,
    /// Whether browsing is complete (file selected or cancelled)
    pub complete: bool,
    /// Selected file path (if any)
    pub selected_file: Option<PathBuf>,
    /// Scroll offset for long lists
    pub scroll_offset: usize,
}

impl FileBrowserState {
    /// Create a new file browser starting at the given directory
    pub fn new(start_dir: &Path, extensions: Vec<String>) -> Self {
        let current_dir = if start_dir.exists() && start_dir.is_dir() {
            start_dir.to_path_buf()
        } else {
            std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"))
        };

        let mut state = Self {
            current_dir: current_dir.clone(),
            entries: Vec::new(),
            selected: 0,
            extensions,
            error: None,
            complete: false,
            selected_file: None,
            scroll_offset: 0,
        };

        state.refresh_entries();
        state
    }

    /// Refresh the list of entries in the current directory
    pub fn refresh_entries(&mut self) {
        self.entries.clear();
        self.error = None;

        // Add parent directory entry if not at root
        if let Some(parent) = self.current_dir.parent() {
            self.entries.push(FileEntry::parent_dir(parent.to_path_buf()));
        }

        // Read directory contents
        match fs::read_dir(&self.current_dir) {
            Ok(read_dir) => {
                let mut dirs: Vec<FileEntry> = Vec::new();
                let mut files: Vec<FileEntry> = Vec::new();

                for entry in read_dir.flatten() {
                    let path = entry.path();
                    let name = entry.file_name().to_string_lossy().to_string();

                    // Skip hidden files
                    if name.starts_with('.') {
                        continue;
                    }

                    let is_dir = path.is_dir();
                    let size = entry.metadata().map(|m| m.len()).unwrap_or(0);

                    if is_dir {
                        dirs.push(FileEntry {
                            name,
                            path,
                            is_dir: true,
                            size,
                        });
                    } else {
                        // Filter by extension if extensions are specified
                        if self.extensions.is_empty() || self.matches_extension(&path) {
                            files.push(FileEntry {
                                name,
                                path,
                                is_dir: false,
                                size,
                            });
                        }
                    }
                }

                // Sort directories and files alphabetically
                dirs.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
                files.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

                // Add directories first, then files
                self.entries.extend(dirs);
                self.entries.extend(files);
            }
            Err(e) => {
                self.error = Some(format!("Failed to read directory: {}", e));
            }
        }

        // Reset selection if out of bounds
        if self.selected >= self.entries.len() {
            self.selected = self.entries.len().saturating_sub(1);
        }
    }

    /// Check if a file matches the extension filter
    fn matches_extension(&self, path: &Path) -> bool {
        if let Some(ext) = path.extension() {
            let ext_str = ext.to_string_lossy().to_lowercase();
            self.extensions.iter().any(|e| e.to_lowercase() == ext_str)
        } else {
            false
        }
    }

    /// Move selection up
    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
            self.adjust_scroll();
        }
    }

    /// Move selection down
    pub fn move_down(&mut self) {
        if self.selected < self.entries.len().saturating_sub(1) {
            self.selected += 1;
            self.adjust_scroll();
        }
    }

    /// Adjust scroll offset to keep selection visible
    fn adjust_scroll(&mut self) {
        let visible_items = 15; // Approximate visible items
        if self.selected < self.scroll_offset {
            self.scroll_offset = self.selected;
        } else if self.selected >= self.scroll_offset + visible_items {
            self.scroll_offset = self.selected - visible_items + 1;
        }
    }

    /// Handle enter key - navigate into directory or select file
    pub fn handle_enter(&mut self) {
        if let Some(entry) = self.entries.get(self.selected) {
            if entry.is_dir {
                // Navigate into directory
                self.current_dir = entry.path.clone();
                self.selected = 0;
                self.scroll_offset = 0;
                self.refresh_entries();
            } else {
                // Select file
                self.selected_file = Some(entry.path.clone());
                self.complete = true;
            }
        }
    }

    /// Cancel file selection
    pub fn cancel(&mut self) {
        self.complete = true;
        self.selected_file = None;
    }

    /// Go to home directory
    pub fn go_home(&mut self) {
        if let Ok(home) = std::env::var("HOME") {
            self.current_dir = PathBuf::from(home);
            self.selected = 0;
            self.scroll_offset = 0;
            self.refresh_entries();
        }
    }

    /// Go to root directory
    pub fn go_root(&mut self) {
        self.current_dir = PathBuf::from("/");
        self.selected = 0;
        self.scroll_offset = 0;
        self.refresh_entries();
    }
}

