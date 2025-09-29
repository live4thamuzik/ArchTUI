//! Input handling module
//!
//! Handles different types of user input including popups, text input, and selection dialogs.

use crate::config::Package;
use ratatui::widgets::ListState;

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
    pub fn get_predefined_options(field_name: &str) -> Vec<String> {
        match field_name {
            "Boot Mode" => vec!["Auto".to_string(), "UEFI".to_string(), "BIOS".to_string()],
            "Secure Boot" => vec!["No".to_string(), "Yes".to_string()],
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
            "Partitioning Strategy" => vec![
                "auto_simple".to_string(),
                "auto_simple_luks".to_string(),
                "auto_lvm".to_string(),
                "auto_luks_lvm".to_string(),
                "auto_raid".to_string(),
                "auto_raid_luks".to_string(),
                "auto_raid_lvm".to_string(),
                "auto_raid_lvm_luks".to_string(),
                "manual".to_string(),
            ],
            "Encryption" => vec!["Auto".to_string(), "Yes".to_string(), "No".to_string()],
            "Root Filesystem" => vec!["ext4".to_string(), "xfs".to_string(), "btrfs".to_string()],
            "Separate Home Partition" => vec!["Yes".to_string(), "No".to_string()],
            "Home Filesystem" => vec!["ext4".to_string(), "xfs".to_string(), "btrfs".to_string()],
            "Swap" => vec!["Yes".to_string(), "No".to_string()],
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
            "Btrfs Snapshots" => vec!["Yes".to_string(), "No".to_string()],
            "Btrfs Frequency" => vec![
                "hourly".to_string(),
                "daily".to_string(),
                "weekly".to_string(),
                "monthly".to_string(),
            ],
            "Btrfs Keep Count" => vec![
                "3".to_string(),
                "5".to_string(),
                "10".to_string(),
                "20".to_string(),
            ],
            "Btrfs Assistant" => vec!["Yes".to_string(), "No".to_string()],
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
                // This will be dynamically populated based on selected region
                vec!["Please select a timezone region first".to_string()]
            }
            "Time Sync (NTP)" => vec!["Yes".to_string(), "No".to_string()],
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
            "Kernel" => vec![
                "linux".to_string(),
                "linux-lts".to_string(),
                "linux-zen".to_string(),
                "linux-hardened".to_string(),
            ],
            "Multilib" => vec!["Yes".to_string(), "No".to_string()],
            "GPU Drivers" => vec![
                "Auto".to_string(),
                "NVIDIA".to_string(),
                "AMD".to_string(),
                "Intel".to_string(),
            ],
            "AUR Helper" => vec!["paru".to_string(), "yay".to_string(), "none".to_string()],
            "Flatpak" => vec!["Yes".to_string(), "No".to_string()],
            "Bootloader" => vec!["grub".to_string(), "systemd-boot".to_string()],
            "OS Prober" => vec!["Yes".to_string(), "No".to_string()],
            "GRUB Theme" => vec!["Yes".to_string(), "No".to_string()],
            "GRUB Theme Selection" => vec![
                "PolyDark".to_string(),
                "CyberEXS".to_string(),
                "CyberPunk".to_string(),
                "HyperFluent".to_string(),
                "none".to_string(),
            ],
            "Desktop Environment" => vec![
                "none".to_string(),
                "gnome".to_string(),
                "kde".to_string(),
                "hyprland".to_string(),
            ],
            "Display Manager" => vec!["none".to_string(), "gdm".to_string(), "sddm".to_string()],
            "Plymouth" => vec!["Yes".to_string(), "No".to_string()],
            "Plymouth Theme" => vec![
                "arch-glow".to_string(),
                "arch-mac-style".to_string(),
                "none".to_string(),
            ],
            "Numlock on Boot" => vec!["Yes".to_string(), "No".to_string()],
            "Git Repository" => vec!["Yes".to_string(), "No".to_string()],
            _ => vec!["Yes".to_string(), "No".to_string()],
        }
    }

    /// Detect available disks on the system
    fn detect_available_disks() -> Vec<String> {
        use std::process::Command;

        let mut disks = Vec::new();

        // Try to use lsblk to get disk information
        if let Ok(output) = Command::new("lsblk")
            .args(["-d", "-n", "-o", "NAME,SIZE,TYPE"])
            .output()
        {
            let output_str = String::from_utf8_lossy(&output.stdout);
            for line in output_str.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 3 && parts[2] == "disk" {
                    let disk_name = format!("/dev/{}", parts[0]);
                    let disk_size = parts[1];
                    disks.push(format!("{} ({})", disk_name, disk_size));
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
                    disks.push(disk.to_string());
                }
            }
        }

        disks
    }
}
