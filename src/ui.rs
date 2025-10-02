//! User interface rendering module
//!
//! Handles all UI rendering, layout, and visual components.

use crate::app::AppState;
use crate::input::InputHandler;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph, Wrap},
    Frame,
};

/// UI renderer for the application
pub struct UiRenderer {
    /// ASCII art header
    header_lines: Vec<Line<'static>>,
}

impl Default for UiRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl UiRenderer {
    /// Create a new UI renderer
    pub fn new() -> Self {
        Self {
            header_lines: Self::create_header(),
        }
    }

    /// Render the complete UI based on application state
    pub fn render(&self, f: &mut Frame, state: &AppState, input_handler: &mut InputHandler) {
        // If dialog is active, render ONLY the dialog - don't render main UI behind it
        if input_handler.is_dialog_active() {
            self.render_input_dialog(f, input_handler);
            return;
        }

        match state.mode {
            crate::app::AppMode::MainMenu => {
                self.render_main_menu(f, state);
            }
            crate::app::AppMode::GuidedInstaller => {
                self.render_configuration_ui(f, state, input_handler);
            }
            crate::app::AppMode::AutomatedInstall => {
                self.render_automated_install_ui(f, state);
            }
            crate::app::AppMode::ToolsMenu => {
                self.render_tools_menu(f, state);
            }
            crate::app::AppMode::DiskTools => {
                self.render_disk_tools_menu(f, state);
            }
            crate::app::AppMode::SystemTools => {
                self.render_system_tools_menu(f, state);
            }
            crate::app::AppMode::UserTools => {
                self.render_user_tools_menu(f, state);
            }
            crate::app::AppMode::NetworkTools => {
                self.render_network_tools_menu(f, state);
            }
            crate::app::AppMode::ToolDialog => {
                self.render_tool_dialog(f, state);
            }
            crate::app::AppMode::ToolExecution => {
                self.render_tool_execution(f, state);
            }
            crate::app::AppMode::Installation => {
                self.render_installation_ui(f, state);
            }
            crate::app::AppMode::Complete => {
                self.render_completion_ui(f, state);
            }
        }
    }

    /// Render the configuration UI
    fn render_configuration_ui(
        &self,
        f: &mut Frame,
        state: &AppState,
        _input_handler: &InputHandler,
    ) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(7), // Header
                Constraint::Length(3), // Title
                Constraint::Min(10),   // Configuration options
                Constraint::Length(3), // Instructions
                Constraint::Length(3), // Start button
            ])
            .split(f.area());

        // Render header
        self.render_header(f, chunks[0]);

        // Render title
        self.render_title(f, chunks[1], "Arch Linux Installation Configuration");

        // Render configuration options
        self.render_config_options(f, chunks[2], state);

        // Render instructions
        self.render_instructions(
            f,
            chunks[3],
            "Use ↑↓ to navigate, Enter to configure, Space to install, 'B' to go back, 'q' to quit",
        );

        // Render start button
        self.render_start_button(f, chunks[4], state);
    }

    /// Render the installation UI
    fn render_installation_ui(&self, f: &mut Frame, state: &AppState) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(7), // Header
                Constraint::Length(3), // Title
                Constraint::Length(3), // Progress bar
                Constraint::Min(0),    // Installer output
            ])
            .split(f.area());

        // Render header
        self.render_header(f, chunks[0]);

        // Render title
        self.render_title(f, chunks[1], "Arch Linux Installation Progress");

        // Render progress bar with real progress
        self.render_progress_bar(f, chunks[2], state.installation_progress as u16);

        // Render installer output with real output
        self.render_installer_output(f, chunks[3], &state.installer_output);
    }

    /// Render the completion UI
    fn render_completion_ui(&self, f: &mut Frame, state: &AppState) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(7), // Header
                Constraint::Length(3), // Title
                Constraint::Min(0),    // Completion message
            ])
            .split(f.area());

        // Render header
        self.render_header(f, chunks[0]);

        // Render title
        self.render_title(f, chunks[1], "Installation Complete");

        // Render completion message
        let message = Paragraph::new(state.status_message.clone())
            .block(Block::default().borders(Borders::ALL).title("Status"))
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::Green));
        f.render_widget(message, chunks[2]);
    }

    /// Render the ASCII art header
    fn render_header(&self, f: &mut Frame, area: Rect) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        let header = Paragraph::new(self.header_lines.clone())
            .block(Block::default().borders(Borders::NONE))
            .alignment(Alignment::Center);
        f.render_widget(header, area);
    }

    /// Render a title section
    fn render_title(&self, f: &mut Frame, area: Rect, title: &str) {
        let title_widget = Paragraph::new(title)
            .block(Block::default().borders(Borders::ALL))
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::Cyan));
        f.render_widget(title_widget, area);
    }

    /// Render configuration options list with professional scrolling
    fn render_config_options(&self, f: &mut Frame, area: Rect, state: &AppState) {
        // Calculate actual visible height from the area (used by the app state)
        let _visible_height = crate::scrolling::ScrollState::visible_items_from_rect(area);

        // Update scroll state with actual visible height (this will be handled by the app state)
        // For now, we'll use the current visible range
        let (start_idx, end_idx) = state.config_scroll.visible_range();

        // Create visible items with proper styling
        let visible_items: Vec<ListItem> = state
            .config
            .options
            .iter()
            .enumerate()
            .skip(start_idx)
            .take(end_idx - start_idx)
            .map(|(index, option)| {
                self.create_config_item(option, index, state.config_scroll.selected_index)
            })
            .collect();

        // Create title with page info
        let title = if let Some((current_page, total_pages)) = state.config_scroll.page_info() {
            format!(
                "Configuration Options (Page {}/{} - ↑↓ Scroll, PgUp/PgDn, Home/End)",
                current_page, total_pages
            )
        } else {
            "Configuration Options".to_string()
        };

        let list =
            List::new(visible_items).block(Block::default().borders(Borders::ALL).title(title));
        f.render_widget(list, area);
    }

    /// Create a configuration item with proper styling
    fn create_config_item(
        &self,
        option: &crate::config::ConfigOption,
        index: usize,
        current_step: usize,
    ) -> ListItem<'_> {
        let display_value = if option.value.is_empty() {
            "[Press Enter]".to_string()
        } else {
            // Special display logic for different field types
            match option.name.as_str() {
                "User Password" | "Root Password" => "***".to_string(),
                "Additional Pacman Packages" | "Additional AUR Packages" => {
                    if option.value.is_empty() {
                        "[Press Enter]".to_string()
                    } else {
                        option.value.clone()
                    }
                }
                _ => option.value.clone(),
            }
        };

        let text = format!("{}: {}", option.name, display_value);
        let style = if index == current_step {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        };

        ListItem::new(text).style(style)
    }

    /// Render instructions text
    fn render_instructions(&self, f: &mut Frame, area: Rect, text: &str) {
        let instructions = Paragraph::new(text)
            .block(Block::default().borders(Borders::NONE))
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::Yellow));
        f.render_widget(instructions, area);
    }

    /// Render start button
    fn render_start_button(&self, f: &mut Frame, area: Rect, state: &AppState) {
        let is_selected = state.config_scroll.selected_index == state.config.options.len();
        let button_text = if is_selected {
            "  START INSTALLATION (Press Enter)  "
        } else {
            "  START INSTALLATION  "
        };

        let style = if is_selected {
            Style::default().fg(Color::Yellow).bg(Color::Rgb(0, 100, 0))
        } else {
            Style::default().fg(Color::Green)
        };

        let button = Paragraph::new(button_text)
            .block(Block::default().borders(Borders::ALL))
            .alignment(Alignment::Center)
            .style(style);
        f.render_widget(button, area);
    }

    /// Render progress bar
    fn render_progress_bar(&self, f: &mut Frame, area: Rect, progress: u16) {
        let gauge = Gauge::default()
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Installation Progress"),
            )
            .gauge_style(Style::default().fg(Color::Blue))
            .percent(progress);
        f.render_widget(gauge, area);
    }

    /// Render installer output
    fn render_installer_output(&self, f: &mut Frame, area: Rect, output: &[String]) {
        let output_lines: Vec<Line> = output.iter().map(|line| Line::from(line.clone())).collect();

        let output_widget = Paragraph::new(output_lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Installer Output"),
            )
            .wrap(Wrap { trim: true });
        f.render_widget(output_widget, area);
    }

    /// Create the ASCII art header
    fn create_header() -> Vec<Line<'static>> {
        vec![
            Line::from(vec![
                Span::styled("  █████╗ ██████╗  ██████╗██╗  ██╗██╗███╗   ██╗███████╗████████╗ █████╗ ██╗     ██╗     ", Style::default().fg(Color::Cyan)),
            ]),
            Line::from(vec![
                Span::styled(" ██╔══██╗██╔══██╗██╔════╝██║  ██║██║████╗  ██║██╔════╝╚══██╔══╝██╔══██╗██║     ██║     ", Style::default().fg(Color::Cyan)),
            ]),
            Line::from(vec![
                Span::styled(" ███████║██████╔╝██║     ███████║██║██╔██╗ ██║███████╗   ██║   ███████║██║     ██║     ", Style::default().fg(Color::Cyan)),
            ]),
            Line::from(vec![
                Span::styled(" ██╔══██║██╔══██╗██║     ██╔══██║██║██║╚██╗██║╚════██║   ██║   ██╔══██║██║     ██║     ", Style::default().fg(Color::Cyan)),
            ]),
            Line::from(vec![
                Span::styled(" ██║  ██║██║  ██║╚██████╗██║  ██║██║██║ ╚████║███████║   ██║   ██║  ██║███████╗███████╗", Style::default().fg(Color::Cyan)),
            ]),
            Line::from(vec![
                Span::styled(" ╚═╝  ╚═╝╚═╝  ╚═╝ ╚═════╝╚═╝  ╚═╝╚═╝╚═╝  ╚═══╝╚══════╝   ╚═╝   ╚═╝  ╚═╝╚══════╝╚══════╝", Style::default().fg(Color::Cyan)),
            ]),
        ]
    }

    /// Render input dialog overlay
    fn render_input_dialog(&self, f: &mut Frame, input_handler: &mut InputHandler) {
        if let Some(ref mut dialog) = input_handler.current_dialog {
            let area = f.area();

            // Fill entire screen with black background
            let background = Block::default()
                .borders(Borders::NONE)
                .style(Style::default().bg(Color::Black).fg(Color::Black));
            f.render_widget(background, area);

            // Calculate dialog size and position (centered)
            let dialog_width = 80;
            let dialog_height = 25;
            let x = (area.width.saturating_sub(dialog_width)) / 2;
            let y = (area.height.saturating_sub(dialog_height)) / 2;

            let dialog_area = Rect::new(x, y, dialog_width, dialog_height);

            // Create dialog layout
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3), // Title
                    Constraint::Length(3), // Instructions
                    Constraint::Min(0),    // Content
                    Constraint::Length(3), // Buttons/status
                ])
                .split(dialog_area);

            // Render dialog with black background and white border
            let dialog_bg = Block::default()
                .borders(Borders::ALL)
                .style(Style::default().bg(Color::Black).fg(Color::White));
            f.render_widget(dialog_bg, dialog_area);

            // Title
            let title = Paragraph::new(dialog.title.clone())
                .block(Block::default().borders(Borders::NONE))
                .alignment(Alignment::Center)
                .style(Style::default().fg(Color::Yellow));
            f.render_widget(title, chunks[0]);

            // Instructions
            let instructions = Paragraph::new(dialog.instructions.clone())
                .block(Block::default().borders(Borders::NONE))
                .alignment(Alignment::Center)
                .style(Style::default().fg(Color::White));
            f.render_widget(instructions, chunks[1]);

            // Content based on input type
            let selected_index = dialog.input_type.get_selected_index();
            match &mut dialog.input_type {
                crate::input::InputType::TextInput { .. } => {
                    let input_text = dialog.get_display_value();
                    let input_display = if input_text.is_empty() {
                        "Enter value...".to_string()
                    } else {
                        input_text
                    };

                    let input_widget = Paragraph::new(input_display)
                        .block(Block::default().borders(Borders::ALL).title("Input"))
                        .style(Style::default().fg(Color::Green));
                    f.render_widget(input_widget, chunks[2]);
                }
                crate::input::InputType::Selection {
                    scroll_state,
                    options,
                    ..
                } => {
                    let (start, end) = scroll_state.visible_range();
                    let items: Vec<ListItem> = options
                        .iter()
                        .enumerate()
                        .skip(start)
                        .take(end - start)
                        .map(|(index, option)| {
                            let style = if index == selected_index {
                                Style::default().fg(Color::Yellow)
                            } else {
                                Style::default()
                            };
                            ListItem::new(option.clone()).style(style)
                        })
                        .collect();

                    let list = List::new(items)
                        .block(Block::default().borders(Borders::ALL).title("Options"));
                    f.render_widget(list, chunks[2]);
                }
                crate::input::InputType::DiskSelection {
                    available_disks, ..
                } => {
                    let items: Vec<ListItem> = available_disks
                        .iter()
                        .enumerate()
                        .map(|(index, disk)| {
                            let style = if index == selected_index {
                                Style::default().fg(Color::Yellow)
                            } else {
                                Style::default()
                            };
                            ListItem::new(disk.clone()).style(style)
                        })
                        .collect();

                    let list = List::new(items).block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title("Available Disks"),
                    );
                    f.render_widget(list, chunks[2]);
                }
                crate::input::InputType::PackageSelection {
                    current_input,
                    output_lines,
                    scroll_offset,
                    package_list,
                    show_search_results,
                    search_results,
                    list_state,
                    is_pacman,
                    ..
                } => {
                    let title = if *is_pacman {
                        "Interactive Pacman Package Selection"
                    } else {
                        "Interactive AUR Package Selection"
                    };

                    let block = Block::default()
                        .borders(Borders::ALL)
                        .title(title)
                        .title_style(
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                        )
                        .title_bottom("Type commands, Enter to execute, Esc to exit")
                        .style(Style::default().bg(Color::Black).fg(Color::White));

                    if *show_search_results && !search_results.is_empty() {
                        // Display search results with scrolling
                        let package_items: Vec<ListItem> = search_results
                            .iter()
                            .map(|p| {
                                let status = if p.installed { "[I]" } else { "[ ]" };

                                // Check if this package is already selected in our config
                                let is_selected = package_list.contains(&p.name);
                                let selection_indicator = if is_selected { "✓" } else { " " };

                                let text = format!(
                                    "{} {} {}/{} ({}) - {}",
                                    status,
                                    selection_indicator,
                                    p.repo,
                                    p.name,
                                    p.version,
                                    p.description
                                );

                                // Style selected packages differently
                                let style = if is_selected {
                                    Style::default()
                                        .fg(Color::Green)
                                        .add_modifier(Modifier::BOLD)
                                } else {
                                    Style::default()
                                };

                                ListItem::new(text).style(style)
                            })
                            .collect();

                        let search_list = List::new(package_items)
                            .block(block.title(
                                "Search Results - ↑↓ Navigate | Enter Toggle Selection | Esc Exit",
                            ))
                            .highlight_style(
                                Style::default()
                                    .fg(Color::LightGreen)
                                    .add_modifier(Modifier::BOLD),
                            )
                            .highlight_symbol(">> ");

                        f.render_stateful_widget(search_list, chunks[2], list_state);
                    } else {
                        // Display normal command interface (like old version)
                        let max_visible_lines: usize = 15; // Approximate visible lines in content area
                        let mut list_items: Vec<ListItem> = output_lines
                            .iter()
                            .skip(*scroll_offset)
                            .take(max_visible_lines.saturating_sub(1)) // Reserve 1 line for input prompt
                            .map(|line| ListItem::new(line.as_str()))
                            .collect();

                        // Add current input line
                        let prompt = if *is_pacman {
                            "Package selection> "
                        } else {
                            "AUR package selection> "
                        };
                        let input_line = format!("{}{}", prompt, current_input);
                        list_items.push(
                            ListItem::new(input_line).style(Style::default().fg(Color::Yellow)),
                        );

                        let list = List::new(list_items)
                            .block(block)
                            .style(Style::default().bg(Color::Black).fg(Color::White));

                        f.render_widget(list, chunks[2]);
                    }
                }
                crate::input::InputType::Warning { message, .. } => {
                    // Render warning message with proper formatting
                    let warning_text = message.join("\n");
                    let warning_widget = Paragraph::new(warning_text)
                        .block(Block::default().borders(Borders::ALL).title("⚠️  WARNING"))
                        .style(Style::default().fg(Color::Red))
                        .alignment(Alignment::Center)
                        .wrap(ratatui::widgets::Wrap { trim: true });
                    f.render_widget(warning_widget, chunks[2]);
                }
                crate::input::InputType::PasswordInput { .. } => {
                    let input_text = dialog.get_display_value();
                    let input_display = if input_text.is_empty() {
                        "Enter password...".to_string()
                    } else {
                        input_text
                    };

                    let input_widget = Paragraph::new(input_display)
                        .block(Block::default().borders(Borders::ALL).title("Password"))
                        .style(Style::default().fg(Color::Green));
                    f.render_widget(input_widget, chunks[2]);
                }
                crate::input::InputType::MultiDiskSelection {
                    selected_disks,
                    available_disks,
                    scroll_state,
                    min_disks,
                    max_disks,
                    ..
                } => {
                    // Create list items with selection status
                    let items: Vec<ListItem> = available_disks
                        .iter()
                        .enumerate()
                        .map(|(i, disk)| {
                            let is_selected = selected_disks.contains(disk);
                            let status = if is_selected { "[X]" } else { "[ ]" };
                            let item_text = format!("{} {}", status, disk);

                            ListItem::new(item_text).style(if i == scroll_state.selected_index {
                                Style::default().fg(Color::Yellow).bg(Color::DarkGray)
                            } else if is_selected {
                                Style::default().fg(Color::Green)
                            } else {
                                Style::default().fg(Color::White)
                            })
                        })
                        .collect();

                    let list = List::new(items)
                        .block(Block::default().borders(Borders::ALL).title(format!(
                            "Selected: {}/{} (Min: {}, Max: {})",
                            selected_disks.len(),
                            max_disks,
                            min_disks,
                            max_disks
                        )))
                        .highlight_style(Style::default().fg(Color::Yellow).bg(Color::DarkGray));

                    f.render_widget(list, chunks[2]);
                }
            }

            // Status/buttons
            let status = Paragraph::new("Enter: Confirm | Esc: Cancel")
                .block(Block::default().borders(Borders::NONE))
                .alignment(Alignment::Center)
                .style(Style::default().fg(Color::Cyan));
            f.render_widget(status, chunks[3]);
        }
    }

    /// Render the main menu
    fn render_main_menu(&self, f: &mut Frame, state: &AppState) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(7), // Header
                Constraint::Length(3), // Title
                Constraint::Min(10),   // Menu
                Constraint::Length(3), // Instructions
            ])
            .split(f.area());

        // Render header
        self.render_header(f, chunks[0]);

        // Render title
        self.render_title(f, chunks[1], "Arch Linux Toolkit");

        // Render main menu
        let menu_items = [
            " ▶ Guided Installer  (Recommended for new users)",
            " ▶ Automated Install (Run from configuration file)",
            " ▶ Arch Linux Tools  (System repair and administration)",
            " ▶ Quit",
        ];

        let menu_items: Vec<ListItem> = menu_items
            .iter()
            .enumerate()
            .map(|(index, item)| {
                let style = if index == state.main_menu_selection {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                ListItem::new(*item).style(style)
            })
            .collect();

        let menu = List::new(menu_items)
            .block(Block::default().borders(Borders::ALL).title("Main Menu"))
            .highlight_style(
                Style::default()
                    .bg(Color::Blue)
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol(">> ");

        f.render_widget(menu, chunks[2]);

        // Render instructions
        self.render_instructions(
            f,
            chunks[3],
            "Use ↑↓ to navigate, Enter to select, 'b' to go back, 'q' to quit",
        );
    }

    /// Render the tools menu
    fn render_tools_menu(&self, f: &mut Frame, state: &AppState) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(7), // Header
                Constraint::Length(3), // Title
                Constraint::Min(10),   // Menu
                Constraint::Length(3), // Instructions
            ])
            .split(f.area());

        // Render header
        self.render_header(f, chunks[0]);

        // Render title
        self.render_title(f, chunks[1], "Arch Linux Tools");

        // Render tools menu
        let menu_items = [
            " ▶ Disk & Filesystem Tools",
            " ▶ System & Boot Tools",
            " ▶ User & Security Tools",
            " ▶ Network Tools",
            " ▶ Back to Main Menu",
        ];

        let menu_items: Vec<ListItem> = menu_items
            .iter()
            .enumerate()
            .map(|(index, item)| {
                let style = if index == state.tools_menu_selection {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                ListItem::new(*item).style(style)
            })
            .collect();

        let menu = List::new(menu_items)
            .block(Block::default().borders(Borders::ALL).title("Tools Menu"))
            .highlight_style(
                Style::default()
                    .bg(Color::Blue)
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol(">> ");

        f.render_widget(menu, chunks[2]);

        // Render instructions
        self.render_instructions(
            f,
            chunks[3],
            "Use ↑↓ to navigate, Enter to select, 'b' to go back",
        );
    }

    /// Render disk tools menu
    fn render_disk_tools_menu(&self, f: &mut Frame, state: &AppState) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(7), // Header
                Constraint::Length(3), // Title
                Constraint::Min(10),   // Menu
                Constraint::Length(3), // Instructions
            ])
            .split(f.area());

        // Render header
        self.render_header(f, chunks[0]);

        // Render title
        self.render_title(f, chunks[1], "Disk & Filesystem Tools");

        // Render disk tools menu
        let menu_items = [
            " ▶ Partition Disk (Manual)",
            " ▶ Format Partition",
            " ▶ Wipe Disk",
            " ▶ Check Disk Health",
            " ▶ Mount/Unmount Partitions",
            " ▶ Back to Tools Menu",
        ];

        let menu_items: Vec<ListItem> = menu_items
            .iter()
            .enumerate()
            .map(|(index, item)| {
                let style = if index == state.tools_menu_selection {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                ListItem::new(*item).style(style)
            })
            .collect();

        let menu = List::new(menu_items)
            .block(Block::default().borders(Borders::ALL).title("Disk Tools"))
            .highlight_style(
                Style::default()
                    .bg(Color::Blue)
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol(">> ");

        f.render_widget(menu, chunks[2]);

        // Render instructions
        self.render_instructions(
            f,
            chunks[3],
            "Use ↑↓ to navigate, Enter to select, 'b' to go back",
        );
    }

    /// Render system tools menu
    fn render_system_tools_menu(&self, f: &mut Frame, state: &AppState) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(7), // Header
                Constraint::Length(3), // Title
                Constraint::Min(10),   // Menu
                Constraint::Length(3), // Instructions
            ])
            .split(f.area());

        // Render header
        self.render_header(f, chunks[0]);

        // Render title
        self.render_title(f, chunks[1], "System & Boot Tools");

        // Render system tools menu
        let menu_items = [
            " ▶ Install/Repair Bootloader",
            " ▶ Generate fstab",
            " ▶ Chroot into System",
            " ▶ Enable/Disable Services",
            " ▶ System Information",
            " ▶ Back to Tools Menu",
        ];

        let menu_items: Vec<ListItem> = menu_items
            .iter()
            .enumerate()
            .map(|(index, item)| {
                let style = if index == state.tools_menu_selection {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                ListItem::new(*item).style(style)
            })
            .collect();

        let menu = List::new(menu_items)
            .block(Block::default().borders(Borders::ALL).title("System Tools"))
            .highlight_style(
                Style::default()
                    .bg(Color::Blue)
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol(">> ");

        f.render_widget(menu, chunks[2]);

        // Render instructions
        self.render_instructions(
            f,
            chunks[3],
            "Use ↑↓ to navigate, Enter to select, 'b' to go back",
        );
    }

    /// Render user tools menu
    fn render_user_tools_menu(&self, f: &mut Frame, state: &AppState) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(7), // Header
                Constraint::Length(3), // Title
                Constraint::Min(10),   // Menu
                Constraint::Length(3), // Instructions
            ])
            .split(f.area());

        // Render header
        self.render_header(f, chunks[0]);

        // Render title
        self.render_title(f, chunks[1], "User & Security Tools");

        // Render user tools menu
        let menu_items = [
            " ▶ Add New User",
            " ▶ Reset Password",
            " ▶ Manage User Groups",
            " ▶ Configure SSH",
            " ▶ Security Audit",
            " ▶ Back to Tools Menu",
        ];

        let menu_items: Vec<ListItem> = menu_items
            .iter()
            .enumerate()
            .map(|(index, item)| {
                let style = if index == state.tools_menu_selection {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                ListItem::new(*item).style(style)
            })
            .collect();

        let menu = List::new(menu_items)
            .block(Block::default().borders(Borders::ALL).title("User Tools"))
            .highlight_style(
                Style::default()
                    .bg(Color::Blue)
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol(">> ");

        f.render_widget(menu, chunks[2]);

        // Render instructions
        self.render_instructions(
            f,
            chunks[3],
            "Use ↑↓ to navigate, Enter to select, 'b' to go back",
        );
    }

    /// Render network tools menu
    fn render_network_tools_menu(&self, f: &mut Frame, state: &AppState) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(7), // Header
                Constraint::Length(3), // Title
                Constraint::Min(10),   // Menu
                Constraint::Length(3), // Instructions
            ])
            .split(f.area());

        // Render header
        self.render_header(f, chunks[0]);

        // Render title
        self.render_title(f, chunks[1], "Network Tools");

        // Render network tools menu
        let menu_items = [
            " ▶ Configure Network Interface",
            " ▶ Test Network Connectivity",
            " ▶ Configure Firewall",
            " ▶ Network Diagnostics",
            " ▶ Back to Tools Menu",
        ];

        let menu_items: Vec<ListItem> = menu_items
            .iter()
            .enumerate()
            .map(|(index, item)| {
                let style = if index == state.tools_menu_selection {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                ListItem::new(*item).style(style)
            })
            .collect();

        let menu = List::new(menu_items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Network Tools"),
            )
            .highlight_style(
                Style::default()
                    .bg(Color::Blue)
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol(">> ");

        f.render_widget(menu, chunks[2]);

        // Render instructions
        self.render_instructions(
            f,
            chunks[3],
            "Use ↑↓ to navigate, Enter to select, 'b' to go back",
        );
    }

    /// Render automated install UI
    fn render_automated_install_ui(&self, f: &mut Frame, _state: &AppState) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(7), // Header
                Constraint::Length(3), // Title
                Constraint::Min(10),   // Content
                Constraint::Length(3), // Instructions
            ])
            .split(f.area());

        // Render header
        self.render_header(f, chunks[0]);

        // Render title
        self.render_title(f, chunks[1], "Automated Installation");

        // Render content
        let content = Paragraph::new("Automated installation from configuration file")
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Configuration"),
            )
            .alignment(Alignment::Center);
        f.render_widget(content, chunks[2]);

        // Render instructions
        self.render_instructions(
            f,
            chunks[3],
            "Press Enter to select config file, 'b' to go back",
        );
    }

    /// Render tool parameter dialog
    fn render_tool_dialog(&self, f: &mut Frame, state: &AppState) {
        if let Some(ref dialog) = state.tool_dialog {
            let area = f.area();

            // Create a centered dialog box
            let dialog_width = (area.width * 3 / 4).min(80);
            let dialog_height = (area.height * 3 / 4).min(20);
            let dialog_x = (area.width - dialog_width) / 2;
            let dialog_y = (area.height - dialog_height) / 2;

            let dialog_rect =
                ratatui::layout::Rect::new(dialog_x, dialog_y, dialog_width, dialog_height);

            // Draw dialog background
            f.render_widget(
                ratatui::widgets::Block::default()
                    .borders(ratatui::widgets::Borders::ALL)
                    .title(format!("Configure {}", dialog.tool_name))
                    .style(ratatui::style::Style::default().bg(ratatui::style::Color::DarkGray)),
                dialog_rect,
            );

            // Render parameter list
            let param_area = ratatui::layout::Rect::new(
                dialog_x + 2,
                dialog_y + 2,
                dialog_width - 4,
                dialog_height - 6,
            );

            let mut param_items = Vec::new();
            for (i, param) in dialog.parameters.iter().enumerate() {
                let style = if i == dialog.current_param {
                    ratatui::style::Style::default().fg(ratatui::style::Color::Yellow)
                } else {
                    ratatui::style::Style::default()
                };

                let value = if i < dialog.param_values.len() {
                    &dialog.param_values[i]
                } else {
                    ""
                };

                param_items.push(ratatui::widgets::ListItem::new(ratatui::text::Line::from(
                    vec![
                        ratatui::text::Span::styled(
                            format!("{}: ", param.name),
                            ratatui::style::Style::default().fg(ratatui::style::Color::Cyan),
                        ),
                        ratatui::text::Span::styled(value.to_string(), style),
                    ],
                )));
            }

            let param_list = ratatui::widgets::List::new(param_items).highlight_style(
                ratatui::style::Style::default().fg(ratatui::style::Color::Yellow),
            );

            f.render_widget(param_list, param_area);

            // Render instructions
            let instruction_area = ratatui::layout::Rect::new(
                dialog_x + 2,
                dialog_y + dialog_height - 3,
                dialog_width - 4,
                1,
            );

            f.render_widget(
                ratatui::widgets::Paragraph::new("Enter: Next parameter | b: Back to tools")
                    .style(ratatui::style::Style::default().fg(ratatui::style::Color::Gray)),
                instruction_area,
            );
        }
    }

    /// Render tool execution screen
    fn render_tool_execution(&self, f: &mut Frame, state: &AppState) {
        let area = f.area();

        // Title
        let title_area = ratatui::layout::Rect::new(0, 0, area.width, 3);
        f.render_widget(
            ratatui::widgets::Paragraph::new("Tool Execution")
                .style(ratatui::style::Style::default().fg(ratatui::style::Color::Yellow))
                .alignment(ratatui::layout::Alignment::Center),
            title_area,
        );

        // Status message
        let status_area = ratatui::layout::Rect::new(0, 3, area.width, 1);
        f.render_widget(
            ratatui::widgets::Paragraph::new(state.status_message.as_str())
                .style(ratatui::style::Style::default().fg(ratatui::style::Color::Green)),
            status_area,
        );

        // Tool output
        if !state.tool_output.is_empty() {
            let output_area = ratatui::layout::Rect::new(2, 5, area.width - 4, area.height - 8);
            let output_items: Vec<ratatui::widgets::ListItem> = state
                .tool_output
                .iter()
                .map(|line| ratatui::widgets::ListItem::new(line.as_str()))
                .collect();

            let output_list = ratatui::widgets::List::new(output_items)
                .style(ratatui::style::Style::default().fg(ratatui::style::Color::White));

            f.render_widget(output_list, output_area);
        }

        // Instructions
        let instruction_area = ratatui::layout::Rect::new(0, area.height - 2, area.width, 1);
        f.render_widget(
            ratatui::widgets::Paragraph::new("b: Back to tools | q: Quit")
                .style(ratatui::style::Style::default().fg(ratatui::style::Color::Gray))
                .alignment(ratatui::layout::Alignment::Center),
            instruction_area,
        );
    }
}
