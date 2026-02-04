//! Input handling module
//!
//! Handles different types of user input including popups, text input, and selection dialogs.

use crate::config::Package;
use crate::process_guard::CommandProcessGroup;
use crate::types::{
    AurHelper, AutoToggle, Bootloader, BootMode, DesktopEnvironment, DisplayManager, Filesystem,
    GpuDriver, GrubTheme, Kernel, PartitionScheme, PlymouthTheme, SnapshotFrequency, Toggle,
};
use ratatui::widgets::ListState;
use strum::IntoEnumIterator;

/// Information about a partition
#[derive(Debug, Clone)]
pub struct PartitionInfo {
    #[allow(dead_code)]
    pub name: String,
    #[allow(dead_code)]
    pub size: String,
}

/// Layout information for manually partitioned disks
#[derive(Debug, Clone)]
pub struct PartitionLayout {
    pub partitions: Vec<PartitionInfo>,
    pub table_type: String,
    #[allow(dead_code)]
    pub has_esp: bool,
    #[allow(dead_code)]
    pub has_boot: bool,
    #[allow(dead_code)]
    pub has_root: bool,
}

/// Types of input dialogs
#[derive(Debug, Clone, PartialEq)]
pub enum InputType {
    /// Text input for values like hostname, username, passwords
    TextInput {
        field_name: String,
        current_value: String,
        placeholder: String,
    },
    /// Selection from predefined options
    Selection {
        field_name: String,
        options: Vec<String>,
        scroll_state: crate::scrolling::ScrollState,
    },
    /// Disk selection with detection
    DiskSelection {
        current_value: String,
        available_disks: Vec<String>,
        scroll_state: crate::scrolling::ScrollState,
    },
    /// Multi-disk selection for RAID and manual partitioning
    MultiDiskSelection {
        selected_disks: Vec<String>,
        available_disks: Vec<String>,
        scroll_state: crate::scrolling::ScrollState,
        min_disks: usize,
        max_disks: usize,
    },
    /// Package selection (for additional packages)
    PackageSelection {
        field_name: String,
        current_input: String,
        output_lines: Vec<String>,
        scroll_offset: usize,
        package_list: String,
        is_pacman: bool,
        search_results: Vec<Package>,
        list_state: ListState,
        show_search_results: bool,
    },
    /// Warning dialog with information and acknowledgment
    Warning {
        title: String,
        message: Vec<String>,
        acknowledged: bool,
    },
    /// Password input with obscuring
    PasswordInput {
        field_name: String,
        current_value: String,
        placeholder: String,
    },
}

/// Input dialog state
#[derive(Debug, Clone)]
pub struct InputDialog {
    /// Type of input dialog
    pub input_type: InputType,
    /// Title of the dialog
    pub title: String,
    /// Instructions for the user
    pub instructions: String,
}

impl InputDialog {
    /// Create a new input dialog
    pub fn new(input_type: InputType, title: String, instructions: String) -> Self {
        Self {
            input_type,
            title,
            instructions,
        }
    }

    /// Handle keyboard input for the dialog
    pub fn handle_input(&mut self, key_event: crossterm::event::KeyEvent) -> InputResult {
        match &mut self.input_type {
            InputType::TextInput { current_value, .. } => match key_event.code {
                crossterm::event::KeyCode::Enter => {
                    return InputResult::Confirm(current_value.clone());
                }
                crossterm::event::KeyCode::Esc => {
                    return InputResult::Cancel;
                }
                crossterm::event::KeyCode::Backspace => {
                    current_value.pop();
                }
                crossterm::event::KeyCode::Char(c) => {
                    current_value.push(c);
                }
                _ => {}
            },
            InputType::Selection {
                scroll_state,
                options,
                ..
            } => match key_event.code {
                crossterm::event::KeyCode::Up => {
                    scroll_state.move_up();
                }
                crossterm::event::KeyCode::Down => {
                    scroll_state.move_down();
                }
                crossterm::event::KeyCode::PageUp => {
                    scroll_state.page_up();
                }
                crossterm::event::KeyCode::PageDown => {
                    scroll_state.page_down();
                }
                crossterm::event::KeyCode::Enter => {
                    let selected_option = &options[scroll_state.selected_index];
                    // Skip warning lines - they start with ⚠️ or are empty or contain warning text
                    if selected_option.starts_with("⚠️")
                        || selected_option.is_empty()
                        || selected_option.contains("WARNING")
                        || selected_option.contains("Make sure")
                        || selected_option.contains("See:")
                        || selected_option.contains("BIOS/Legacy")
                        || selected_option.contains("motherboard")
                    {
                        // Don't select warning lines, just move to next option
                        scroll_state.move_down();
                    } else {
                        return InputResult::Confirm(selected_option.clone());
                    }
                }
                crossterm::event::KeyCode::Esc => {
                    return InputResult::Cancel;
                }
                _ => {}
            },
            InputType::DiskSelection {
                scroll_state,
                available_disks,
                ..
            } => match key_event.code {
                crossterm::event::KeyCode::Up => {
                    scroll_state.move_up();
                }
                crossterm::event::KeyCode::Down => {
                    scroll_state.move_down();
                }
                crossterm::event::KeyCode::Enter => {
                    return InputResult::Confirm(
                        available_disks[scroll_state.selected_index].clone(),
                    );
                }
                crossterm::event::KeyCode::Esc => {
                    return InputResult::Cancel;
                }
                _ => {}
            },
            InputType::MultiDiskSelection {
                selected_disks,
                available_disks,
                scroll_state,
                min_disks,
                max_disks,
                ..
            } => match key_event.code {
                crossterm::event::KeyCode::Up => {
                    scroll_state.move_up();
                }
                crossterm::event::KeyCode::Down => {
                    scroll_state.move_down();
                }
                crossterm::event::KeyCode::Char(' ') => {
                    // Toggle selection
                    let selected_disk = &available_disks[scroll_state.selected_index];
                    if selected_disks.contains(selected_disk) {
                        selected_disks.retain(|d| d != selected_disk);
                    } else if selected_disks.len() < *max_disks {
                        selected_disks.push(selected_disk.clone());
                    }
                }
                crossterm::event::KeyCode::Enter => {
                    // Validate selection
                    if selected_disks.len() < *min_disks {
                        // Show error - need more disks
                        return InputResult::Continue;
                    }

                    // For RAID strategies, validate disk compatibility
                    // Note: We'll pass partitioning strategy through the dialog context
                    // For now, we'll validate based on the number of disks selected
                    if selected_disks.len() >= 2 {
                        if let Err(_error) = InputHandler::validate_raid_disks(selected_disks) {
                            // Show RAID validation error - disks not compatible
                            return InputResult::Continue;
                        }
                    }

                    return InputResult::Confirm(selected_disks.join(","));
                }
                crossterm::event::KeyCode::Esc => {
                    return InputResult::Cancel;
                }
                _ => {}
            },
            InputType::PackageSelection {
                current_input,
                output_lines,
                scroll_offset,
                package_list,
                is_pacman,
                search_results,
                list_state,
                show_search_results,
                ..
            } => {
                match key_event.code {
                    crossterm::event::KeyCode::Up => {
                        if *show_search_results && !search_results.is_empty() {
                            // Handle list navigation for search results
                            if let Some(selected) = list_state.selected() {
                                if selected > 0 {
                                    list_state.select(Some(selected - 1));
                                }
                            }
                        } else {
                            // Handle command mode scrolling
                            if *scroll_offset > 0 {
                                *scroll_offset -= 1;
                            }
                        }
                        return InputResult::Continue;
                    }
                    crossterm::event::KeyCode::Down => {
                        if *show_search_results && !search_results.is_empty() {
                            // Handle list navigation for search results
                            if let Some(selected) = list_state.selected() {
                                if selected < search_results.len() - 1 {
                                    list_state.select(Some(selected + 1));
                                }
                            } else if !search_results.is_empty() {
                                list_state.select(Some(0));
                            }
                        } else {
                            // Handle command mode scrolling - ensure input prompt stays visible
                            let max_visible: usize = 15; // Approximate visible lines in content area
                            if *scroll_offset
                                < output_lines
                                    .len()
                                    .saturating_sub(max_visible.saturating_sub(2))
                            {
                                *scroll_offset += 1;
                            }
                        }
                        return InputResult::Continue;
                    }
                    crossterm::event::KeyCode::Enter => {
                        if *show_search_results {
                            // Toggle package selection
                            if let Some(selected) = list_state.selected() {
                                if let Some(selected_result) = search_results.get(selected) {
                                    // Extract package name from search result
                                    let package_name = &selected_result.name;

                                    // Toggle selection
                                    let is_already_selected = package_list.contains(package_name);
                                    if is_already_selected {
                                        // Remove package
                                        let new_list = package_list
                                            .split_whitespace()
                                            .filter(|&p| p != package_name)
                                            .collect::<Vec<&str>>()
                                            .join(" ");
                                        *package_list = new_list;
                                        output_lines
                                            .push(format!("✓ Removed package: {}", package_name));
                                    } else {
                                        // Add package
                                        if !package_list.is_empty() {
                                            package_list.push(' ');
                                        }
                                        package_list.push_str(package_name);
                                        output_lines
                                            .push(format!("✓ Added package: {}", package_name));
                                    }
                                }
                            }
                        } else {
                            // Execute command
                            let command = current_input.trim();

                            // Echo the command to the terminal
                            let prompt = if *is_pacman {
                                "Package selection> "
                            } else {
                                "AUR package selection> "
                            };
                            output_lines.push(format!("{}{}", prompt, current_input));
                            // Auto-scroll to show latest content
                            let max_visible: usize = 15;
                            if output_lines.len() > max_visible.saturating_sub(2) {
                                *scroll_offset = output_lines
                                    .len()
                                    .saturating_sub(max_visible.saturating_sub(2));
                            }

                            if command == "done" {
                                if !package_list.trim().is_empty() {
                                    output_lines.push(format!(
                                        "Selected packages: {}",
                                        package_list.trim()
                                    ));
                                }
                                return InputResult::Confirm(package_list.clone());
                            } else if let Some(search_term) = command.strip_prefix("search ") {
                                // Execute package search using utility functions
                                search_results.clear();

                                let result = if *is_pacman {
                                    crate::package_utils::search_pacman_packages(search_term)
                                } else {
                                    crate::package_utils::search_aur_packages(search_term)
                                };

                                match result {
                                    Ok(packages) => {
                                        if packages.is_empty() {
                                            output_lines.push(format!(
                                                "No packages found matching: {}",
                                                search_term
                                            ));
                                        } else {
                                            search_results.extend(packages);
                                        }
                                    }
                                    Err(e) => {
                                        output_lines.push(format!("Search failed: {}", e));
                                    }
                                }

                                if !search_results.is_empty() {
                                    output_lines.push(format!("Found {} packages. Use ↑↓ to navigate, Enter to add, Esc to exit", search_results.len()));
                                    // Auto-scroll to show latest content
                                    let max_visible: usize = 15;
                                    if output_lines.len() > max_visible.saturating_sub(2) {
                                        *scroll_offset = output_lines
                                            .len()
                                            .saturating_sub(max_visible.saturating_sub(2));
                                    }
                                    *show_search_results = true;
                                    list_state.select(Some(0)); // Select first result
                                }
                            } else if let Some(package_name) = command.strip_prefix("add ") {
                                if package_list.is_empty() {
                                    package_list.push_str(package_name);
                                } else {
                                    package_list.push(' ');
                                    package_list.push_str(package_name);
                                }
                                output_lines.push(format!("✓ Added: {}", package_name));
                                // Auto-scroll to show latest content
                                let max_visible: usize = 15;
                                if output_lines.len() > max_visible.saturating_sub(2) {
                                    *scroll_offset = output_lines
                                        .len()
                                        .saturating_sub(max_visible.saturating_sub(2));
                                }
                            } else if let Some(package_name) = command.strip_prefix("remove ") {
                                let packages: Vec<&str> = package_list.split(' ').collect();
                                let filtered: Vec<&str> = packages
                                    .into_iter()
                                    .filter(|&p| p != package_name)
                                    .collect();
                                *package_list = filtered.join(" ");
                                output_lines.push(format!("✓ Removed: {}", package_name));
                                // Auto-scroll to show latest content
                                let max_visible: usize = 15;
                                if output_lines.len() > max_visible.saturating_sub(2) {
                                    *scroll_offset = output_lines
                                        .len()
                                        .saturating_sub(max_visible.saturating_sub(2));
                                }
                            } else if command == "list" {
                                if package_list.trim().is_empty() {
                                    output_lines.push("No packages selected".to_string());
                                } else {
                                    output_lines.push(format!(
                                        "Selected packages: {}",
                                        package_list.trim()
                                    ));
                                }
                                // Auto-scroll to show latest content
                                let max_visible: usize = 15;
                                if output_lines.len() > max_visible.saturating_sub(2) {
                                    *scroll_offset = output_lines
                                        .len()
                                        .saturating_sub(max_visible.saturating_sub(2));
                                }
                            } else if command.is_empty() {
                                // Empty command, just show prompt
                            } else {
                                output_lines.push(format!("Unknown command: '{}'", command));
                                output_lines.push("Type 'done' to finish, 'search <term>' to search, or 'list' to see current packages".to_string());
                                // Auto-scroll to show latest content
                                let max_visible: usize = 15;
                                if output_lines.len() > max_visible.saturating_sub(2) {
                                    *scroll_offset = output_lines
                                        .len()
                                        .saturating_sub(max_visible.saturating_sub(2));
                                }
                            }
                            current_input.clear();
                        }
                    }
                    crossterm::event::KeyCode::PageUp => {
                        if !*show_search_results {
                            // Page up in command mode
                            *scroll_offset = scroll_offset.saturating_sub(10);
                        }
                        return InputResult::Continue;
                    }
                    crossterm::event::KeyCode::PageDown => {
                        if !*show_search_results {
                            // Page down in command mode
                            let max_visible: usize = 15; // Approximate visible lines in content area
                            *scroll_offset = (*scroll_offset + 10).min(
                                output_lines
                                    .len()
                                    .saturating_sub(max_visible.saturating_sub(2)),
                            );
                        }
                        return InputResult::Continue;
                    }
                    crossterm::event::KeyCode::Esc => {
                        if *show_search_results {
                            // Exit search results view
                            *show_search_results = false;
                            search_results.clear();
                            list_state.select(None);
                            return InputResult::Continue;
                        } else {
                            return InputResult::Cancel;
                        }
                    }
                    crossterm::event::KeyCode::Char(c) => {
                        current_input.push(c);
                    }
                    crossterm::event::KeyCode::Backspace => {
                        current_input.pop();
                    }
                    _ => {}
                }
            }
            InputType::Warning { acknowledged, .. } => match key_event.code {
                crossterm::event::KeyCode::Enter => {
                    *acknowledged = true;
                    return InputResult::Confirm("acknowledged".to_string());
                }
                crossterm::event::KeyCode::Esc => {
                    return InputResult::Cancel;
                }
                _ => {}
            },
            InputType::PasswordInput { current_value, .. } => match key_event.code {
                crossterm::event::KeyCode::Enter => {
                    return InputResult::Confirm(current_value.clone());
                }
                crossterm::event::KeyCode::Esc => {
                    return InputResult::Cancel;
                }
                crossterm::event::KeyCode::Backspace => {
                    current_value.pop();
                }
                crossterm::event::KeyCode::Char(c) => {
                    current_value.push(c);
                }
                _ => {}
            },
        }
        InputResult::Continue
    }

    /// Get the current display value for the input
    pub fn get_display_value(&self) -> String {
        match &self.input_type {
            InputType::TextInput {
                current_value,
                placeholder,
                ..
            } => {
                if current_value.is_empty() {
                    placeholder.clone()
                } else {
                    current_value.clone()
                }
            }
            InputType::Selection {
                scroll_state,
                options,
                ..
            } => options
                .get(scroll_state.selected_index)
                .cloned()
                .unwrap_or_default(),
            InputType::DiskSelection {
                scroll_state,
                available_disks,
                ..
            } => available_disks
                .get(scroll_state.selected_index)
                .cloned()
                .unwrap_or_default(),
            InputType::MultiDiskSelection { selected_disks, .. } => {
                if selected_disks.is_empty() {
                    "No disks selected".to_string()
                } else {
                    format!(
                        "{} disk(s) selected: {}",
                        selected_disks.len(),
                        selected_disks.join(", ")
                    )
                }
            }
            InputType::PackageSelection { package_list, .. } => package_list.clone(),
            InputType::Warning { .. } => "Press Enter to acknowledge".to_string(),
            InputType::PasswordInput {
                current_value,
                placeholder,
                ..
            } => {
                if current_value.is_empty() {
                    placeholder.clone()
                } else {
                    "*".repeat(current_value.len())
                }
            }
        }
    }
}

impl InputType {
    /// Get the selected index for input types that support selection
    pub fn get_selected_index(&self) -> usize {
        match self {
            InputType::Selection { scroll_state, .. } => scroll_state.selected_index,
            InputType::DiskSelection { scroll_state, .. } => scroll_state.selected_index,
            InputType::MultiDiskSelection { scroll_state, .. } => scroll_state.selected_index,
            InputType::PackageSelection { list_state, .. } => list_state.selected().unwrap_or(0),
            _ => 0,
        }
    }
}

/// Result of input handling
#[derive(Debug, Clone)]
pub enum InputResult {
    /// User confirmed with a value
    Confirm(String),
    /// User cancelled
    Cancel,
    /// Continue processing input
    Continue,
}

/// Input handler for managing all input dialogs
pub struct InputHandler {
    /// Current active dialog
    pub current_dialog: Option<InputDialog>,
}

impl Default for InputHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl InputHandler {
    /// Create a new input handler
    pub fn new() -> Self {
        Self {
            current_dialog: None,
        }
    }

    /// Get timezones for a specific region
    pub fn get_timezones_for_region(region: &str) -> Vec<String> {
        match region {
            "Africa" => vec![
                "Abidjan".to_string(),
                "Accra".to_string(),
                "Addis_Ababa".to_string(),
                "Algiers".to_string(),
                "Asmara".to_string(),
                "Bamako".to_string(),
                "Bangui".to_string(),
                "Banjul".to_string(),
                "Bissau".to_string(),
                "Blantyre".to_string(),
                "Brazzaville".to_string(),
                "Bujumbura".to_string(),
                "Cairo".to_string(),
                "Casablanca".to_string(),
                "Ceuta".to_string(),
                "Conakry".to_string(),
                "Dakar".to_string(),
                "Dar_es_Salaam".to_string(),
                "Djibouti".to_string(),
                "Douala".to_string(),
                "El_Aaiun".to_string(),
                "Freetown".to_string(),
                "Gaborone".to_string(),
                "Harare".to_string(),
                "Johannesburg".to_string(),
                "Juba".to_string(),
                "Kampala".to_string(),
                "Khartoum".to_string(),
                "Kigali".to_string(),
                "Kinshasa".to_string(),
                "Lagos".to_string(),
                "Libreville".to_string(),
                "Lome".to_string(),
                "Luanda".to_string(),
                "Lubumbashi".to_string(),
                "Lusaka".to_string(),
                "Malabo".to_string(),
                "Maputo".to_string(),
                "Maseru".to_string(),
                "Mbabane".to_string(),
                "Mogadishu".to_string(),
                "Monrovia".to_string(),
                "Nairobi".to_string(),
                "Ndjamena".to_string(),
                "Niamey".to_string(),
                "Nouakchott".to_string(),
                "Ouagadougou".to_string(),
                "Porto-Novo".to_string(),
                "Sao_Tome".to_string(),
                "Tripoli".to_string(),
                "Tunis".to_string(),
                "Windhoek".to_string(),
            ],
            "America" => vec![
                "Adak".to_string(),
                "Anchorage".to_string(),
                "Anguilla".to_string(),
                "Antigua".to_string(),
                "Araguaina".to_string(),
                "Argentina/Buenos_Aires".to_string(),
                "Argentina/Catamarca".to_string(),
                "Argentina/Cordoba".to_string(),
                "Argentina/Jujuy".to_string(),
                "Argentina/La_Rioja".to_string(),
                "Argentina/Mendoza".to_string(),
                "Argentina/Rio_Gallegos".to_string(),
                "Argentina/Salta".to_string(),
                "Argentina/San_Juan".to_string(),
                "Argentina/San_Luis".to_string(),
                "Argentina/Tucuman".to_string(),
                "Argentina/Ushuaia".to_string(),
                "Aruba".to_string(),
                "Asuncion".to_string(),
                "Atikokan".to_string(),
                "Bahia".to_string(),
                "Bahia_Banderas".to_string(),
                "Barbados".to_string(),
                "Belem".to_string(),
                "Belize".to_string(),
                "Blanc-Sablon".to_string(),
                "Boa_Vista".to_string(),
                "Bogota".to_string(),
                "Boise".to_string(),
                "Cambridge_Bay".to_string(),
                "Campo_Grande".to_string(),
                "Cancun".to_string(),
                "Caracas".to_string(),
                "Cayenne".to_string(),
                "Cayman".to_string(),
                "Chicago".to_string(),
                "Chihuahua".to_string(),
                "Costa_Rica".to_string(),
                "Creston".to_string(),
                "Cuiaba".to_string(),
                "Curacao".to_string(),
                "Danmarkshavn".to_string(),
                "Dawson".to_string(),
                "Dawson_Creek".to_string(),
                "Denver".to_string(),
                "Detroit".to_string(),
                "Dominica".to_string(),
                "Edmonton".to_string(),
                "Eirunepe".to_string(),
                "El_Salvador".to_string(),
                "Fort_Nelson".to_string(),
                "Fortaleza".to_string(),
                "Glace_Bay".to_string(),
                "Goose_Bay".to_string(),
                "Grand_Turk".to_string(),
                "Grenada".to_string(),
                "Guadeloupe".to_string(),
                "Guatemala".to_string(),
                "Guayaquil".to_string(),
                "Guyana".to_string(),
                "Halifax".to_string(),
                "Havana".to_string(),
                "Hermosillo".to_string(),
                "Indiana/Indianapolis".to_string(),
                "Indiana/Knox".to_string(),
                "Indiana/Marengo".to_string(),
                "Indiana/Petersburg".to_string(),
                "Indiana/Tell_City".to_string(),
                "Indiana/Vevay".to_string(),
                "Indiana/Vincennes".to_string(),
                "Indiana/Winamac".to_string(),
                "Inuvik".to_string(),
                "Iqaluit".to_string(),
                "Jamaica".to_string(),
                "Juneau".to_string(),
                "Kentucky/Louisville".to_string(),
                "Kentucky/Monticello".to_string(),
                "Kralendijk".to_string(),
                "La_Paz".to_string(),
                "Lima".to_string(),
                "Los_Angeles".to_string(),
                "Lower_Princes".to_string(),
                "Maceio".to_string(),
                "Managua".to_string(),
                "Manaus".to_string(),
                "Marigot".to_string(),
                "Martinique".to_string(),
                "Matamoros".to_string(),
                "Mazatlan".to_string(),
                "Menominee".to_string(),
                "Merida".to_string(),
                "Metlakatla".to_string(),
                "Mexico_City".to_string(),
                "Miquelon".to_string(),
                "Moncton".to_string(),
                "Monterrey".to_string(),
                "Montevideo".to_string(),
                "Montserrat".to_string(),
                "Nassau".to_string(),
                "New_York".to_string(),
                "Nipigon".to_string(),
                "Nome".to_string(),
                "Noronha".to_string(),
                "North_Dakota/Beulah".to_string(),
                "North_Dakota/Center".to_string(),
                "North_Dakota/New_Salem".to_string(),
                "Ojinaga".to_string(),
                "Panama".to_string(),
                "Pangnirtung".to_string(),
                "Paramaribo".to_string(),
                "Phoenix".to_string(),
                "Port-au-Prince".to_string(),
                "Port_of_Spain".to_string(),
                "Porto_Velho".to_string(),
                "Puerto_Rico".to_string(),
                "Punta_Arenas".to_string(),
                "Rainy_River".to_string(),
                "Rankin_Inlet".to_string(),
                "Recife".to_string(),
                "Regina".to_string(),
                "Resolute".to_string(),
                "Rio_Branco".to_string(),
                "Santarem".to_string(),
                "Santiago".to_string(),
                "Santo_Domingo".to_string(),
                "Sao_Paulo".to_string(),
                "Scoresbysund".to_string(),
                "Sitka".to_string(),
                "St_Barthelemy".to_string(),
                "St_Johns".to_string(),
                "St_Kitts".to_string(),
                "St_Lucia".to_string(),
                "St_Thomas".to_string(),
                "St_Vincent".to_string(),
                "Swift_Current".to_string(),
                "Tegucigalpa".to_string(),
                "Thule".to_string(),
                "Thunder_Bay".to_string(),
                "Tijuana".to_string(),
                "Toronto".to_string(),
                "Tortola".to_string(),
                "Vancouver".to_string(),
                "Whitehorse".to_string(),
                "Winnipeg".to_string(),
                "Yakutat".to_string(),
                "Yellowknife".to_string(),
            ],
            "Antarctica" => vec![
                "Casey".to_string(),
                "Davis".to_string(),
                "DumontDUrville".to_string(),
                "Macquarie".to_string(),
                "Mawson".to_string(),
                "McMurdo".to_string(),
                "Palmer".to_string(),
                "Rothera".to_string(),
                "Syowa".to_string(),
                "Troll".to_string(),
                "Vostok".to_string(),
            ],
            "Arctic" => vec!["Longyearbyen".to_string()],
            "Asia" => vec![
                "Aden".to_string(),
                "Almaty".to_string(),
                "Amman".to_string(),
                "Anadyr".to_string(),
                "Aqtau".to_string(),
                "Aqtobe".to_string(),
                "Ashgabat".to_string(),
                "Atyrau".to_string(),
                "Baghdad".to_string(),
                "Bahrain".to_string(),
                "Baku".to_string(),
                "Bangkok".to_string(),
                "Barnaul".to_string(),
                "Beirut".to_string(),
                "Bishkek".to_string(),
                "Brunei".to_string(),
                "Chita".to_string(),
                "Choibalsan".to_string(),
                "Colombo".to_string(),
                "Damascus".to_string(),
                "Dhaka".to_string(),
                "Dili".to_string(),
                "Dubai".to_string(),
                "Dushanbe".to_string(),
                "Famagusta".to_string(),
                "Gaza".to_string(),
                "Hebron".to_string(),
                "Ho_Chi_Minh".to_string(),
                "Hong_Kong".to_string(),
                "Hovd".to_string(),
                "Irkutsk".to_string(),
                "Istanbul".to_string(),
                "Jakarta".to_string(),
                "Jayapura".to_string(),
                "Jerusalem".to_string(),
                "Kabul".to_string(),
                "Kamchatka".to_string(),
                "Karachi".to_string(),
                "Kathmandu".to_string(),
                "Khandyga".to_string(),
                "Kolkata".to_string(),
                "Krasnoyarsk".to_string(),
                "Kuala_Lumpur".to_string(),
                "Kuching".to_string(),
                "Kuwait".to_string(),
                "Macau".to_string(),
                "Magadan".to_string(),
                "Makassar".to_string(),
                "Manila".to_string(),
                "Muscat".to_string(),
                "Nicosia".to_string(),
                "Novokuznetsk".to_string(),
                "Novosibirsk".to_string(),
                "Omsk".to_string(),
                "Oral".to_string(),
                "Phnom_Penh".to_string(),
                "Pontianak".to_string(),
                "Pyongyang".to_string(),
                "Qatar".to_string(),
                "Qyzylorda".to_string(),
                "Riyadh".to_string(),
                "Sakhalin".to_string(),
                "Samarkand".to_string(),
                "Seoul".to_string(),
                "Shanghai".to_string(),
                "Singapore".to_string(),
                "Srednekolymsk".to_string(),
                "Taipei".to_string(),
                "Tashkent".to_string(),
                "Tbilisi".to_string(),
                "Tehran".to_string(),
                "Thimphu".to_string(),
                "Tokyo".to_string(),
                "Tomsk".to_string(),
                "Ulaanbaatar".to_string(),
                "Urumqi".to_string(),
                "Ust-Nera".to_string(),
                "Vientiane".to_string(),
                "Vladivostok".to_string(),
                "Yakutsk".to_string(),
                "Yangon".to_string(),
                "Yekaterinburg".to_string(),
                "Yerevan".to_string(),
            ],
            "Atlantic" => vec![
                "Azores".to_string(),
                "Bermuda".to_string(),
                "Canary".to_string(),
                "Cape_Verde".to_string(),
                "Faroe".to_string(),
                "Madeira".to_string(),
                "Reykjavik".to_string(),
                "South_Georgia".to_string(),
                "St_Helena".to_string(),
                "Stanley".to_string(),
            ],
            "Australia" => vec![
                "Adelaide".to_string(),
                "Brisbane".to_string(),
                "Broken_Hill".to_string(),
                "Currie".to_string(),
                "Darwin".to_string(),
                "Eucla".to_string(),
                "Hobart".to_string(),
                "Lindeman".to_string(),
                "Lord_Howe".to_string(),
                "Melbourne".to_string(),
                "Perth".to_string(),
                "Sydney".to_string(),
            ],
            "Europe" => vec![
                "Amsterdam".to_string(),
                "Andorra".to_string(),
                "Astrakhan".to_string(),
                "Athens".to_string(),
                "Belgrade".to_string(),
                "Berlin".to_string(),
                "Bratislava".to_string(),
                "Brussels".to_string(),
                "Bucharest".to_string(),
                "Budapest".to_string(),
                "Busingen".to_string(),
                "Chisinau".to_string(),
                "Copenhagen".to_string(),
                "Dublin".to_string(),
                "Gibraltar".to_string(),
                "Guernsey".to_string(),
                "Helsinki".to_string(),
                "Isle_of_Man".to_string(),
                "Istanbul".to_string(),
                "Jersey".to_string(),
                "Kaliningrad".to_string(),
                "Kiev".to_string(),
                "Kirov".to_string(),
                "Lisbon".to_string(),
                "Ljubljana".to_string(),
                "London".to_string(),
                "Luxembourg".to_string(),
                "Madrid".to_string(),
                "Malta".to_string(),
                "Mariehamn".to_string(),
                "Minsk".to_string(),
                "Monaco".to_string(),
                "Moscow".to_string(),
                "Oslo".to_string(),
                "Paris".to_string(),
                "Podgorica".to_string(),
                "Prague".to_string(),
                "Riga".to_string(),
                "Rome".to_string(),
                "Samara".to_string(),
                "San_Marino".to_string(),
                "Sarajevo".to_string(),
                "Saratov".to_string(),
                "Simferopol".to_string(),
                "Skopje".to_string(),
                "Sofia".to_string(),
                "Stockholm".to_string(),
                "Tallinn".to_string(),
                "Tirane".to_string(),
                "Ulyanovsk".to_string(),
                "Uzhgorod".to_string(),
                "Vaduz".to_string(),
                "Vatican".to_string(),
                "Vienna".to_string(),
                "Vilnius".to_string(),
                "Volgograd".to_string(),
                "Warsaw".to_string(),
                "Zagreb".to_string(),
                "Zaporozhye".to_string(),
                "Zurich".to_string(),
            ],
            "Indian" => vec![
                "Antananarivo".to_string(),
                "Chagos".to_string(),
                "Christmas".to_string(),
                "Cocos".to_string(),
                "Comoro".to_string(),
                "Kerguelen".to_string(),
                "Mahe".to_string(),
                "Maldives".to_string(),
                "Mauritius".to_string(),
                "Mayotte".to_string(),
                "Reunion".to_string(),
            ],
            "Pacific" => vec![
                "Apia".to_string(),
                "Auckland".to_string(),
                "Bougainville".to_string(),
                "Chatham".to_string(),
                "Chuuk".to_string(),
                "Easter".to_string(),
                "Efate".to_string(),
                "Fakaofo".to_string(),
                "Fiji".to_string(),
                "Funafuti".to_string(),
                "Galapagos".to_string(),
                "Gambier".to_string(),
                "Guadalcanal".to_string(),
                "Guam".to_string(),
                "Honolulu".to_string(),
                "Kiritimati".to_string(),
                "Kosrae".to_string(),
                "Kwajalein".to_string(),
                "Majuro".to_string(),
                "Marquesas".to_string(),
                "Midway".to_string(),
                "Nauru".to_string(),
                "Niue".to_string(),
                "Norfolk".to_string(),
                "Noumea".to_string(),
                "Pago_Pago".to_string(),
                "Palau".to_string(),
                "Pitcairn".to_string(),
                "Pohnpei".to_string(),
                "Port_Moresby".to_string(),
                "Rarotonga".to_string(),
                "Saipan".to_string(),
                "Tahiti".to_string(),
                "Tarawa".to_string(),
                "Tongatapu".to_string(),
                "Wake".to_string(),
                "Wallis".to_string(),
            ],
            "US" => vec![
                "Alaska".to_string(),
                "Arizona".to_string(),
                "Central".to_string(),
                "Eastern".to_string(),
                "Hawaii".to_string(),
                "Indiana-Starke".to_string(),
                "Michigan".to_string(),
                "Mountain".to_string(),
                "Pacific".to_string(),
            ],
            _ => vec!["Invalid region".to_string()],
        }
    }

    /// Start a warning dialog
    pub fn start_warning(&mut self, title: String, message: Vec<String>) {
        let input_type = InputType::Warning {
            title: title.clone(),
            message,
            acknowledged: false,
        };

        self.current_dialog = Some(InputDialog::new(
            input_type,
            title,
            "Press Enter to acknowledge, Esc to cancel".to_string(),
        ));
    }

    /// Start a password input dialog
    pub fn start_password_input(
        &mut self,
        field_name: String,
        current_value: String,
        placeholder: String,
    ) {
        let input_type = InputType::PasswordInput {
            field_name: field_name.clone(),
            current_value,
            placeholder,
        };

        self.current_dialog = Some(InputDialog::new(
            input_type,
            format!("Configure {}", field_name),
            "Type the password and press Enter to confirm, Esc to cancel".to_string(),
        ));
    }

    /// Start a text input dialog
    pub fn start_text_input(
        &mut self,
        field_name: String,
        current_value: String,
        placeholder: String,
    ) {
        let input_type = InputType::TextInput {
            field_name: field_name.clone(),
            current_value,
            placeholder,
        };

        self.current_dialog = Some(InputDialog::new(
            input_type,
            format!("Configure {}", field_name),
            "Type the value and press Enter to confirm, Esc to cancel".to_string(),
        ));
    }

    /// Start a selection dialog
    pub fn start_selection(
        &mut self,
        field_name: String,
        options: Vec<String>,
        current_value: String,
    ) {
        let selected_index = options
            .iter()
            .position(|opt| opt == &current_value)
            .unwrap_or(0);

        // Calculate visible items based on terminal size
        // Reserve space for dialog borders, title, instructions, and footer
        let terminal_height = crossterm::terminal::size()
            .map(|(_, h)| h as usize)
            .unwrap_or(24);

        // Reserve space for dialog elements: borders(2) + title(1) + instructions(1) + footer(1) = 5 lines
        let _available_height = terminal_height.saturating_sub(5);
        // Since you said 14 items show, let's set visible_items to match the actual display
        let visible_items = 14; // Fixed to match actual display

        let mut scroll_state = crate::scrolling::ScrollState::new(options.len(), visible_items);
        scroll_state.set_selected(selected_index);

        let input_type = InputType::Selection {
            field_name: field_name.clone(),
            options,
            scroll_state,
        };

        self.current_dialog = Some(InputDialog::new(
            input_type,
            format!("Select {}", field_name),
            "Use ↑↓ or PgUp/PgDn to navigate, Enter to select, Esc to cancel".to_string(),
        ));
    }

    /// Start a disk selection dialog
    pub fn start_disk_selection(&mut self, current_value: String) {
        let available_disks = Self::detect_available_disks();
        let selected_index = available_disks
            .iter()
            .position(|disk| disk == &current_value)
            .unwrap_or(0);

        let mut scroll_state = crate::scrolling::ScrollState::new(available_disks.len(), 10);
        scroll_state.set_selected(selected_index);

        let input_type = InputType::DiskSelection {
            current_value,
            available_disks,
            scroll_state,
        };

        self.current_dialog = Some(InputDialog::new(
            input_type,
            "Select Installation Disk".to_string(),
            "Use ↑↓ to navigate, Enter to select, Esc to cancel".to_string(),
        ));
    }

    /// Start a package selection dialog
    pub fn start_package_selection(&mut self, field_name: String, current_packages: String) {
        let is_pacman = field_name.contains("Pacman");
        let output_lines = vec![
            "Available commands:".to_string(),
            "".to_string(),
            "search <term> - Search for packages".to_string(),
            "add <package> - Add package to installation list".to_string(),
            "remove <package> - Remove package from installation list".to_string(),
            "list - Show current package list".to_string(),
            "done - Finish package selection".to_string(),
        ];

        let input_type = InputType::PackageSelection {
            field_name: field_name.clone(),
            current_input: String::new(),
            output_lines,
            scroll_offset: 0,
            package_list: current_packages,
            is_pacman,
            search_results: Vec::new(),
            list_state: ListState::default(),
            show_search_results: false,
        };

        self.current_dialog = Some(InputDialog::new(
            input_type,
            if is_pacman {
                "Interactive Pacman Package Selection".to_string()
            } else {
                "Interactive AUR Package Selection".to_string()
            },
            "Type commands, Enter to execute, Esc to exit".to_string(),
        ));
    }

    /// Handle keyboard input for the current dialog
    pub fn handle_input(&mut self, key_event: crossterm::event::KeyEvent) -> Option<String> {
        if let Some(ref mut dialog) = self.current_dialog {
            match dialog.handle_input(key_event) {
                InputResult::Confirm(value) => {
                    self.current_dialog = None;
                    Some(value)
                }
                InputResult::Cancel => {
                    self.current_dialog = None;
                    None
                }
                InputResult::Continue => None,
            }
        } else {
            None
        }
    }

    /// Check if a dialog is currently active
    pub fn is_dialog_active(&self) -> bool {
        self.current_dialog.is_some()
    }

    /// Get predefined options for common configuration fields
    ///
    /// Uses enum iteration where possible for type-safe option generation.
    /// Options that have too many values (locales, timezones, countries) remain as static lists.
    pub fn get_predefined_options(field_name: &str) -> Vec<String> {
        match field_name {
            // Type-safe enum-based options
            "Boot Mode" => BootMode::iter().map(|v| v.to_string()).collect(),
            "Secure Boot" => Toggle::iter().rev().map(|v| v.to_string()).collect(), // No first
            "Partitioning Strategy" => PartitionScheme::iter().map(|v| v.to_string()).collect(),
            "Encryption" => AutoToggle::iter().map(|v| v.to_string()).collect(),
            "Root Filesystem" => Filesystem::iter().map(|v| v.to_string()).collect(),
            "Home Filesystem" => Filesystem::iter().map(|v| v.to_string()).collect(),
            "Separate Home Partition" => Toggle::iter().map(|v| v.to_string()).collect(),
            "Swap" => Toggle::iter().map(|v| v.to_string()).collect(),
            "Btrfs Snapshots" => Toggle::iter().map(|v| v.to_string()).collect(),
            "Btrfs Frequency" => SnapshotFrequency::iter().map(|v| v.to_string()).collect(),
            "Btrfs Assistant" => Toggle::iter().map(|v| v.to_string()).collect(),
            "Time Sync (NTP)" => Toggle::iter().map(|v| v.to_string()).collect(),
            "Kernel" => Kernel::iter().map(|v| v.to_string()).collect(),
            "Multilib" => Toggle::iter().map(|v| v.to_string()).collect(),
            "GPU Drivers" => GpuDriver::iter().map(|v| v.to_string()).collect(),
            "AUR Helper" => AurHelper::iter().map(|v| v.to_string()).collect(),
            "Flatpak" => Toggle::iter().map(|v| v.to_string()).collect(),
            "Bootloader" => Bootloader::iter().map(|v| v.to_string()).collect(),
            "OS Prober" => Toggle::iter().map(|v| v.to_string()).collect(),
            "GRUB Theme" => Toggle::iter().map(|v| v.to_string()).collect(),
            "GRUB Theme Selection" => GrubTheme::iter().map(|v| v.to_string()).collect(),
            "Desktop Environment" => DesktopEnvironment::iter().map(|v| v.to_string()).collect(),
            "Display Manager" => DisplayManager::iter().map(|v| v.to_string()).collect(),
            "Plymouth" => Toggle::iter().map(|v| v.to_string()).collect(),
            "Plymouth Theme" => PlymouthTheme::iter().map(|v| v.to_string()).collect(),
            "Numlock on Boot" => Toggle::iter().map(|v| v.to_string()).collect(),
            "Git Repository" => Toggle::iter().map(|v| v.to_string()).collect(),

            // Static lists for options with too many values to enumerate
            "Locale" => vec![
                "en_US.UTF-8".to_string(),
                "en_GB.UTF-8".to_string(),
                "de_DE.UTF-8".to_string(),
                "fr_FR.UTF-8".to_string(),
                "es_ES.UTF-8".to_string(),
                "it_IT.UTF-8".to_string(),
                "pt_BR.UTF-8".to_string(),
                "ru_RU.UTF-8".to_string(),
                "ja_JP.UTF-8".to_string(),
                "zh_CN.UTF-8".to_string(),
            ],
            "Keymap" => vec![
                "us".to_string(),
                "uk".to_string(),
                "de".to_string(),
                "fr".to_string(),
                "es".to_string(),
                "it".to_string(),
                "pt".to_string(),
                "ru".to_string(),
                "jp".to_string(),
            ],
            "Swap Size" => vec![
                "1GB".to_string(),
                "2GB".to_string(),
                "4GB".to_string(),
                "8GB".to_string(),
                "16GB".to_string(),
                "32GB".to_string(),
                "Equal to RAM".to_string(),
                "Double RAM".to_string(),
            ],
            "Btrfs Keep Count" => vec![
                "3".to_string(),
                "5".to_string(),
                "10".to_string(),
                "20".to_string(),
            ],
            "Timezone Region" => vec![
                "Africa".to_string(),
                "America".to_string(),
                "Antarctica".to_string(),
                "Arctic".to_string(),
                "Asia".to_string(),
                "Atlantic".to_string(),
                "Australia".to_string(),
                "Europe".to_string(),
                "Indian".to_string(),
                "Pacific".to_string(),
                "US".to_string(),
            ],
            "Timezone" => {
                // Dynamically populated based on selected region
                vec!["Please select a timezone region first".to_string()]
            }
            "Mirror Country" => vec![
                "Australia".to_string(),
                "Austria".to_string(),
                "Belgium".to_string(),
                "Brazil".to_string(),
                "Bulgaria".to_string(),
                "Canada".to_string(),
                "Chile".to_string(),
                "China".to_string(),
                "Czech Republic".to_string(),
                "Denmark".to_string(),
                "Estonia".to_string(),
                "Finland".to_string(),
                "France".to_string(),
                "Germany".to_string(),
                "Greece".to_string(),
                "Hong Kong".to_string(),
                "Hungary".to_string(),
                "Iceland".to_string(),
                "India".to_string(),
                "Indonesia".to_string(),
                "Ireland".to_string(),
                "Israel".to_string(),
                "Italy".to_string(),
                "Japan".to_string(),
                "Kazakhstan".to_string(),
                "Korea".to_string(),
                "Latvia".to_string(),
                "Lithuania".to_string(),
                "Luxembourg".to_string(),
                "Netherlands".to_string(),
                "New Zealand".to_string(),
                "Norway".to_string(),
                "Poland".to_string(),
                "Portugal".to_string(),
                "Romania".to_string(),
                "Russia".to_string(),
                "Serbia".to_string(),
                "Singapore".to_string(),
                "Slovakia".to_string(),
                "Slovenia".to_string(),
                "South Africa".to_string(),
                "Spain".to_string(),
                "Sweden".to_string(),
                "Switzerland".to_string(),
                "Taiwan".to_string(),
                "Thailand".to_string(),
                "Turkey".to_string(),
                "Ukraine".to_string(),
                "United Kingdom".to_string(),
                "United States".to_string(),
            ],

            // Default fallback for unknown fields
            _ => Toggle::iter().map(|v| v.to_string()).collect(),
        }
    }

    /// Detect available disks on the system
    fn detect_available_disks() -> Vec<String> {
        use std::process::Command;

        let mut disks = Vec::new();

        // Try to use lsblk to get disk information with more details
        if let Ok(output) = Command::new("lsblk")
            .args(["-d", "-n", "-o", "NAME,SIZE,TYPE,RO,TRAN"])
            .in_new_process_group()
            .output()
        {
            let output_str = String::from_utf8_lossy(&output.stdout);
            for line in output_str.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 5 && parts[2] == "disk" {
                    let disk_name = format!("/dev/{}", parts[0]);
                    let disk_size = parts[1];
                    let is_readonly = parts[3] == "1";
                    let transport = parts[4];

                    // Safety check: Skip read-only disks (likely live ISO)
                    if is_readonly {
                        continue;
                    }

                    // Safety check: Skip removable media (USB, CD-ROM)
                    if transport == "usb"
                        || transport == "sata"
                            && disk_size.parse::<u64>().unwrap_or(0) < 1_073_741_824
                    {
                        // Skip disks smaller than 1GB (likely live ISO)
                        continue;
                    }

                    // Get additional disk info for RAID compatibility
                    let disk_info = Self::get_disk_info(&disk_name);
                    disks.push(format!("{} ({}) {}", disk_name, disk_size, disk_info));
                }
            }
        }

        // Fallback to common disk paths if lsblk fails
        if disks.is_empty() {
            let common_disks = vec![
                "/dev/sda",
                "/dev/sdb",
                "/dev/sdc",
                "/dev/sdd",
                "/dev/nvme0n1",
                "/dev/nvme1n1",
                "/dev/nvme2n1",
                "/dev/vda",
                "/dev/vdb",
                "/dev/vdc",
            ];

            for disk in common_disks {
                if std::path::Path::new(disk).exists() {
                    let disk_info = Self::get_disk_info(disk);
                    disks.push(format!("{} {}", disk, disk_info));
                }
            }
        }

        disks
    }

    /// Start multi-disk selection for RAID or manual partitioning
    pub fn start_multi_disk_selection(&mut self, partitioning_strategy: &str) {
        let available_disks = Self::detect_available_disks();

        // Determine disk requirements based on partitioning strategy
        let (min_disks, max_disks, title) = match partitioning_strategy {
            "auto_raid" | "auto_raid_luks" | "auto_raid_lvm" | "auto_raid_lvm_luks" => {
                // RAID requires minimum 2 disks
                (2, 8, "Select Disks for RAID Configuration")
            }
            "manual" => {
                // Manual partitioning can use 1+ disks
                (1, 8, "Select Disks for Manual Partitioning")
            }
            _ => {
                // Default to single disk
                (1, 1, "Select Disk")
            }
        };

        // Validate we have enough disks
        if available_disks.len() < min_disks {
            self.current_dialog = Some(InputDialog::new(
                InputType::Selection {
                    field_name: "error".to_string(),
                    options: vec![format!(
                        "ERROR: Need at least {} disk(s) for {} partitioning, but only {} available",
                        min_disks, partitioning_strategy, available_disks.len()
                    )],
                    scroll_state: crate::scrolling::ScrollState::new(1, 1),
                },
                "Insufficient Disks".to_string(),
                "Press Esc to return".to_string(),
            ));
            return;
        }

        let scroll_state = crate::scrolling::ScrollState::new(available_disks.len(), 10);

        let input_type = InputType::MultiDiskSelection {
            selected_disks: Vec::new(),
            available_disks,
            scroll_state,
            min_disks,
            max_disks,
        };

        self.current_dialog = Some(InputDialog::new(
            input_type,
            title.to_string(),
            "Use ↑↓ to navigate, Space to select/deselect, Enter to confirm, Esc to cancel"
                .to_string(),
        ));
    }

    /// Validate RAID disk compatibility
    fn validate_raid_disks(disks: &[String]) -> Result<(), String> {
        if disks.len() < 2 {
            return Err("RAID requires at least 2 disks".to_string());
        }

        // Extract disk paths from the formatted strings
        let disk_paths: Vec<&str> = disks
            .iter()
            .map(|d| d.split(' ').next().unwrap_or(""))
            .filter(|&d| !d.is_empty())
            .collect();

        // Check disk sizes for RAID compatibility
        let mut disk_sizes = Vec::new();
        for disk in &disk_paths {
            if let Ok(output) = std::process::Command::new("lsblk")
                .args(["-d", "-n", "-o", "SIZE", disk])
                .in_new_process_group()
                .output()
            {
                let size_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
                disk_sizes.push(size_str);
            }
        }

        // Check if all disks have similar sizes (within 10% tolerance)
        if disk_sizes.len() >= 2 {
            let first_size = Self::parse_disk_size(&disk_sizes[0]);
            for size_str in &disk_sizes[1..] {
                let size = Self::parse_disk_size(size_str);
                let tolerance = (first_size as f64 * 0.1) as u64; // 10% tolerance
                if (size as i64 - first_size as i64).abs() > tolerance as i64 {
                    return Err(format!(
                        "RAID disks should be similar sizes. Found: {} vs {}",
                        disk_sizes[0], size_str
                    ));
                }
            }
        }

        Ok(())
    }

    /// Parse disk size string to bytes for comparison
    fn parse_disk_size(size_str: &str) -> u64 {
        let size_str = size_str.trim();
        let (number, unit) = if let Some(stripped) = size_str.strip_suffix("G") {
            (stripped, "G")
        } else if let Some(stripped) = size_str.strip_suffix("M") {
            (stripped, "M")
        } else if let Some(stripped) = size_str.strip_suffix("T") {
            (stripped, "T")
        } else {
            (size_str, "")
        };

        if let Ok(num) = number.parse::<f64>() {
            match unit {
                "T" => (num * 1_000_000_000_000.0) as u64,
                "G" => (num * 1_000_000_000.0) as u64,
                "M" => (num * 1_000_000.0) as u64,
                _ => num as u64,
            }
        } else {
            0
        }
    }

    /// Launch partitioning tool for manual partitioning
    pub fn launch_partitioning_tool(&mut self, disks: &[String]) -> Result<(), String> {
        use std::process::{Command, Stdio};

        // Extract disk paths from the formatted strings
        let disk_paths: Vec<String> = disks
            .iter()
            .map(|d| d.split(' ').next().unwrap_or("").to_string())
            .filter(|d| !d.is_empty())
            .collect();

        if disk_paths.is_empty() {
            return Err("No valid disks selected".to_string());
        }

        // Validate disk paths to prevent command injection
        for disk in &disk_paths {
            if !disk.starts_with("/dev/") || disk.contains("..") || disk.contains(" ") {
                return Err(format!("Invalid disk path: {}", disk));
            }
        }

        // For single disk, use cfdisk (more user-friendly than fdisk)
        // For multiple disks, we'll launch cfdisk for each disk sequentially
        for disk in &disk_paths {
            log::info!("Launching cfdisk for {}", disk);
            log::info!("Please partition this disk according to your needs.");
            log::info!("Press Enter when you're done with {}", disk);

            // Launch cfdisk in interactive mode
            let status = Command::new("cfdisk")
                .arg(disk)
                .stdin(Stdio::inherit())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .in_new_process_group()
                .status();

            match status {
                Ok(exit_status) => {
                    if !exit_status.success() {
                        return Err(format!("cfdisk failed for {}", disk));
                    }
                }
                Err(e) => {
                    return Err(format!("Failed to launch cfdisk for {}: {}", disk, e));
                }
            }
        }

        Ok(())
    }

    /// Start manual partitioning confirmation dialog
    pub fn start_manual_partitioning_confirmation(&mut self, disks: &[String]) {
        let disk_list = disks.join("\n");
        let message = format!(
            "You have selected {} disk(s) for manual partitioning:\n\n{}\n\nThis will launch cfdisk for each disk.\n\nProceed with manual partitioning?",
            disks.len(),
            disk_list
        );

        let options = vec![
            "Yes, start partitioning".to_string(),
            "No, go back to disk selection".to_string(),
        ];

        let scroll_state = crate::scrolling::ScrollState::new(options.len(), 10);

        self.current_dialog = Some(InputDialog::new(
            InputType::Selection {
                field_name: "manual_partitioning_confirm".to_string(),
                options,
                scroll_state,
            },
            "Manual Partitioning Confirmation".to_string(),
            message,
        ));
    }

    /// Validate manual partitioning setup
    pub fn validate_manual_partitioning(
        &self,
        disks: &[String],
        boot_mode: &str,
    ) -> Result<PartitionLayout, String> {
        use std::process::Command;

        // Extract disk paths
        let disk_paths: Vec<String> = disks
            .iter()
            .map(|d| d.split(' ').next().unwrap_or("").to_string())
            .filter(|d| !d.is_empty())
            .collect();

        let mut partitions = Vec::new();
        let mut has_root = false;
        let mut has_esp = false;
        let mut has_boot = false;

        // Scan all partitions on selected disks
        for disk in &disk_paths {
            if let Ok(output) = Command::new("lsblk")
                .args(["-n", "-o", "NAME,TYPE,SIZE", disk])
                .in_new_process_group()
                .output()
            {
                let output_str = String::from_utf8_lossy(&output.stdout);
                for line in output_str.lines() {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2 && parts[1] == "part" {
                        let partition_name = format!("/dev/{}", parts[0]);
                        let partition_size = if parts.len() > 2 { parts[2] } else { "unknown" };
                        partitions.push(PartitionInfo {
                            name: partition_name,
                            size: partition_size.to_string(),
                        });
                    }
                }
            }
        }

        // Validate required partitions based on boot mode
        if boot_mode.to_lowercase() == "uefi" {
            // Check for GPT partition table and ESP partition
            for disk in &disk_paths {
                if let Ok(output) = Command::new("fdisk").args(["-l", disk]).in_new_process_group().output() {
                    let output_str = String::from_utf8_lossy(&output.stdout);
                    if output_str.contains("GPT") {
                        // Good, GPT partition table
                    } else if output_str.contains("MBR") || output_str.contains("DOS") {
                        return Err(format!("UEFI requires GPT partition table on {}", disk));
                    }
                }
            }

            // Check for ESP partition (EF00 type)
            for disk in &disk_paths {
                if let Ok(output) = Command::new("fdisk").args(["-l", disk]).in_new_process_group().output() {
                    let output_str = String::from_utf8_lossy(&output.stdout);
                    if output_str.contains("EF00") || output_str.contains("EFI System") {
                        has_esp = true;
                        break;
                    }
                }
            }

            if !has_esp {
                return Err(
                    "UEFI requires an EFI System Partition (ESP) with type EF00".to_string()
                );
            }
        } else {
            // BIOS mode - check for MBR or GPT with BIOS Boot Partition
            for disk in &disk_paths {
                if let Ok(output) = Command::new("fdisk").args(["-l", disk]).in_new_process_group().output() {
                    let output_str = String::from_utf8_lossy(&output.stdout);
                    if output_str.contains("MBR") || output_str.contains("DOS") {
                        has_boot = true; // MBR has boot sector
                        break;
                    } else if output_str.contains("GPT") && output_str.contains("EF02") {
                        has_boot = true; // GPT with BIOS Boot Partition
                        break;
                    }
                }
            }

            if !has_boot {
                return Err(
                    "BIOS requires MBR partition table or GPT with BIOS Boot Partition".to_string(),
                );
            }
        }

        // Check for root partition (Linux filesystem)
        for disk in &disk_paths {
            if let Ok(output) = Command::new("fdisk").args(["-l", disk]).in_new_process_group().output() {
                let output_str = String::from_utf8_lossy(&output.stdout);
                if output_str.contains("8300") || output_str.contains("Linux filesystem") {
                    has_root = true;
                    break;
                }
            }
        }

        if !has_root {
            return Err("No Linux filesystem partition found. You need at least one root partition (type 8300)".to_string());
        }

        Ok(PartitionLayout {
            partitions,
            table_type: if boot_mode.to_lowercase() == "uefi" {
                "GPT".to_string()
            } else {
                "MBR".to_string()
            },
            has_esp,
            has_boot,
            has_root,
        })
    }

    /// Get additional disk information for RAID compatibility
    fn get_disk_info(disk: &str) -> String {
        use std::process::Command;

        let mut info_parts = Vec::<String>::new();

        // Check if disk is mounted (safety check)
        if let Ok(output) = Command::new("mount").in_new_process_group().output() {
            let output_str = String::from_utf8_lossy(&output.stdout);
            if output_str.contains(disk) {
                info_parts.push("MOUNTED".to_string());
            }
        }

        // Get disk model for RAID compatibility
        if let Ok(output) = Command::new("lsblk")
            .args(["-d", "-n", "-o", "MODEL", disk])
            .in_new_process_group()
            .output()
        {
            let model = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !model.is_empty() && model != "disk" {
                info_parts.push(model);
            }
        }

        // Get sector size for RAID compatibility
        if let Ok(output) = Command::new("blockdev").args(["--getss", disk]).in_new_process_group().output() {
            if let Ok(sector_size) = String::from_utf8_lossy(&output.stdout)
                .trim()
                .parse::<u32>()
            {
                info_parts.push(format!("{}B", sector_size));
            }
        }

        if info_parts.is_empty() {
            "OK".to_string()
        } else {
            info_parts.join(", ")
        }
    }
}
