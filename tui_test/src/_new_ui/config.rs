//! Mock configuration for visual testing
//!
//! Provides a Configuration struct with sample data for rendering.
//! All 46 options match the real app's config.rs with correct enum values.
//! Options start with empty values so they show [Press Enter] until set.

#![allow(dead_code)]

use std::process::Command;

/// Individual configuration option
#[derive(Debug, Clone)]
pub struct ConfigOption {
    pub name: String,
    pub value: String,
    pub required: bool,
    pub description: String,
    pub default_value: String,
    pub options: Vec<String>,
}

impl ConfigOption {
    pub fn new(name: &str, required: bool, description: &str, default_value: &str) -> Self {
        Self {
            name: name.to_string(),
            value: String::new(), // Start empty — shows [Press Enter]
            required,
            description: description.to_string(),
            default_value: default_value.to_string(),
            options: Vec::new(),
        }
    }

    pub fn with_options(mut self, opts: &[&str]) -> Self {
        self.options = opts.iter().map(|s| s.to_string()).collect();
        self
    }

    /// Create with a pre-set value (for sentinel values like N/A)
    pub fn with_value(mut self, val: &str) -> Self {
        self.value = val.to_string();
        self
    }

    pub fn is_password(&self) -> bool {
        self.name.contains("Password")
    }

    pub fn get_value(&self) -> String {
        if self.value.is_empty() {
            self.default_value.clone()
        } else {
            self.value.clone()
        }
    }
}

/// Application configuration with sample installer options
#[derive(Debug, Clone)]
pub struct Configuration {
    pub options: Vec<ConfigOption>,
}

/// Get timezone cities for a given region (mock subset of real app data)
pub fn get_timezones_for_region(region: &str) -> Vec<String> {
    let cities: &[&str] = match region {
        "Africa" => &[
            "Abidjan", "Accra", "Addis_Ababa", "Algiers", "Cairo",
            "Casablanca", "Dakar", "Dar_es_Salaam", "Johannesburg",
            "Kampala", "Khartoum", "Lagos", "Nairobi", "Tripoli", "Tunis",
        ],
        "America" => &[
            "Adak", "Anchorage", "Argentina/Buenos_Aires", "Bogota", "Boise",
            "Chicago", "Denver", "Detroit", "Edmonton", "Halifax", "Havana",
            "Honolulu", "Indiana/Indianapolis", "Juneau", "Lima",
            "Los_Angeles", "Mexico_City", "New_York", "Phoenix",
            "Santiago", "Sao_Paulo", "St_Johns", "Toronto", "Vancouver",
        ],
        "Antarctica" => &[
            "Casey", "Davis", "McMurdo", "Palmer", "Rothera", "Vostok",
        ],
        "Arctic" => &["Longyearbyen"],
        "Asia" => &[
            "Almaty", "Bangkok", "Beirut", "Colombo", "Damascus", "Dhaka",
            "Dubai", "Ho_Chi_Minh", "Hong_Kong", "Istanbul", "Jakarta",
            "Jerusalem", "Kabul", "Karachi", "Kathmandu", "Kolkata",
            "Kuala_Lumpur", "Manila", "Riyadh", "Seoul", "Shanghai",
            "Singapore", "Taipei", "Tehran", "Tokyo", "Yekaterinburg",
        ],
        "Atlantic" => &[
            "Azores", "Bermuda", "Canary", "Cape_Verde", "Faroe",
            "Madeira", "Reykjavik", "South_Georgia", "St_Helena", "Stanley",
        ],
        "Australia" => &[
            "Adelaide", "Brisbane", "Darwin", "Eucla", "Hobart",
            "Lord_Howe", "Melbourne", "Perth", "Sydney",
        ],
        "Europe" => &[
            "Amsterdam", "Athens", "Belgrade", "Berlin", "Brussels",
            "Bucharest", "Budapest", "Copenhagen", "Dublin", "Helsinki",
            "Istanbul", "Kiev", "Lisbon", "Ljubljana", "London",
            "Madrid", "Minsk", "Moscow", "Oslo", "Paris", "Prague",
            "Riga", "Rome", "Stockholm", "Tallinn", "Vienna",
            "Vilnius", "Warsaw", "Zurich",
        ],
        "Indian" => &[
            "Antananarivo", "Chagos", "Christmas", "Cocos", "Comoro",
            "Kerguelen", "Mahe", "Maldives", "Mauritius", "Reunion",
        ],
        "Pacific" => &[
            "Apia", "Auckland", "Fiji", "Guam", "Honolulu",
            "Kiritimati", "Majuro", "Pago_Pago", "Port_Moresby",
            "Tahiti", "Tongatapu",
        ],
        "US" => &[
            "Alaska", "Arizona", "Central", "Eastern", "Hawaii",
            "Indiana-Starke", "Michigan", "Mountain", "Pacific",
        ],
        _ => &["New_York"],
    };
    cities.iter().map(|s| s.to_string()).collect()
}

/// Detect available disks via lsblk, matching the real app's detect_available_disks()
pub fn detect_disks() -> Vec<String> {
    let output = match Command::new("lsblk")
        .args(["-d", "-n", "-o", "NAME,SIZE,TYPE,RO,TRAN"])
        .output()
    {
        Ok(o) => o,
        Err(_) => return vec!["(no disks detected)".to_string()],
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut disks = Vec::new();

    for line in stdout.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 4 {
            continue;
        }

        let name = parts[0];
        let size = parts[1];
        let dtype = parts[2];
        let ro = parts[3];

        // Only whole disks, not read-only
        if dtype != "disk" || ro == "1" {
            continue;
        }

        // Skip USB transport (likely live ISO media)
        if parts.len() >= 5 && parts[4] == "usb" {
            continue;
        }

        let disk_path = format!("/dev/{}", name);

        // Get model info
        let model = Command::new("lsblk")
            .args(["-d", "-n", "-o", "MODEL", &disk_path])
            .output()
            .ok()
            .and_then(|o| {
                let m = String::from_utf8_lossy(&o.stdout).trim().to_string();
                if m.is_empty() || m == "disk" {
                    None
                } else {
                    Some(m)
                }
            })
            .unwrap_or_default();

        let entry = if model.is_empty() {
            format!("{} ({})", disk_path, size)
        } else {
            format!("{} ({}) {}", disk_path, size, model)
        };
        disks.push(entry);
    }

    if disks.is_empty() {
        // Fallback: check common device paths
        for path in &[
            "/dev/sda", "/dev/sdb", "/dev/sdc", "/dev/sdd",
            "/dev/nvme0n1", "/dev/nvme1n1", "/dev/nvme2n1",
            "/dev/vda", "/dev/vdb",
        ] {
            if std::path::Path::new(path).exists() {
                disks.push(format!("{} (unknown size)", path));
            }
        }
    }

    if disks.is_empty() {
        disks.push("(no disks detected)".to_string());
    }

    disks
}

/// Get disk layout for a device path (partition table, partitions, sizes, filesystems)
/// Extracts /dev/xxx from strings like "/dev/sda (953.9G) Samsung..."
pub fn get_disk_layout(device_str: &str) -> Vec<String> {
    // Extract the device path from the selection string
    let dev_path = device_str
        .split_whitespace()
        .next()
        .unwrap_or(device_str);

    if dev_path.is_empty() || dev_path.starts_with('(') {
        return vec!["No device selected".to_string()];
    }

    let mut lines = Vec::new();

    // Get partition table type via blkid or fdisk
    if let Ok(output) = Command::new("fdisk")
        .args(["-l", dev_path])
        .output()
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if line.starts_with("Disk model:")
                || line.starts_with("Disklabel type:")
                || line.starts_with("Disk identifier:")
            {
                lines.push(format!("  {}", line.trim()));
            }
        }
    }

    if lines.is_empty() {
        lines.push(format!("  Device: {}", dev_path));
    }

    lines.push(String::new());

    // Get partition layout via lsblk
    if let Ok(output) = Command::new("lsblk")
        .args(["-o", "NAME,SIZE,TYPE,FSTYPE,LABEL,MOUNTPOINTS", dev_path])
        .output()
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for (i, line) in stdout.lines().enumerate() {
            if i == 0 {
                // Header line — style it
                lines.push(format!("  {}", line));
                lines.push(format!("  {}", "─".repeat(line.len().min(50))));
            } else {
                lines.push(format!("  {}", line));
            }
        }
    }

    if lines.len() <= 2 {
        lines.push("  (no partitions found)".to_string());
    }

    lines
}

/// Detect available partitions via lsblk (for format/mount tools)
pub fn detect_partitions() -> Vec<String> {
    let output = match Command::new("lsblk")
        .args(["-n", "-o", "NAME,SIZE,TYPE,FSTYPE,MOUNTPOINTS"])
        .output()
    {
        Ok(o) => o,
        Err(_) => return vec!["(no partitions detected)".to_string()],
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut parts_list = Vec::new();

    for line in stdout.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 3 {
            continue;
        }

        // Strip tree-drawing characters (├─, └─, etc.)
        let raw_name = parts[0].trim_start_matches(|c: char| !c.is_alphanumeric());
        if raw_name.is_empty() {
            continue;
        }

        let size = parts[1];
        let ptype = parts[2];

        // Only partitions (not whole disks, not lvm, not crypt)
        if ptype != "part" {
            continue;
        }

        let dev_path = format!("/dev/{}", raw_name);
        let fstype = parts.get(3).unwrap_or(&"");
        let mount = parts.get(4).unwrap_or(&"");

        let entry = if !fstype.is_empty() && !mount.is_empty() {
            format!("{} ({}) [{}] mounted: {}", dev_path, size, fstype, mount)
        } else if !fstype.is_empty() {
            format!("{} ({}) [{}]", dev_path, size, fstype)
        } else {
            format!("{} ({})", dev_path, size)
        };
        parts_list.push(entry);
    }

    if parts_list.is_empty() {
        parts_list.push("(no partitions detected)".to_string());
    }

    parts_list
}

/// Detect network interfaces via /sys/class/net
pub fn detect_interfaces() -> Vec<String> {
    let output = match Command::new("ip")
        .args(["-o", "link", "show"])
        .output()
    {
        Ok(o) => o,
        Err(_) => {
            // Fallback: read /sys/class/net
            return std::fs::read_dir("/sys/class/net")
                .map(|entries| {
                    entries
                        .filter_map(|e| e.ok())
                        .map(|e| e.file_name().to_string_lossy().to_string())
                        .filter(|name| name != "lo")
                        .collect()
                })
                .unwrap_or_else(|_| vec!["(no interfaces detected)".to_string()]);
        }
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut ifaces = Vec::new();

    for line in stdout.lines() {
        // Format: "2: enp0s3: <BROADCAST,MULTICAST,UP> ..."
        let parts: Vec<&str> = line.splitn(3, ':').collect();
        if parts.len() < 2 {
            continue;
        }
        let name = parts[1].trim();
        if name == "lo" {
            continue;
        }

        // Get IP address if any
        let ip = Command::new("ip")
            .args(["-4", "-o", "addr", "show", name])
            .output()
            .ok()
            .and_then(|o| {
                let s = String::from_utf8_lossy(&o.stdout).to_string();
                s.split_whitespace()
                    .nth(3)
                    .map(|ip| ip.to_string())
            });

        // Check state (UP/DOWN)
        let state = if line.contains("state UP") {
            "UP"
        } else if line.contains("state DOWN") {
            "DOWN"
        } else {
            "UNKNOWN"
        };

        let entry = if let Some(ref addr) = ip {
            format!("{} [{}] {}", name, state, addr)
        } else {
            format!("{} [{}]", name, state)
        };
        ifaces.push(entry);
    }

    if ifaces.is_empty() {
        ifaces.push("(no interfaces detected)".to_string());
    }

    ifaces
}

/// Detect non-system users (UID >= 1000, with login shell)
pub fn detect_users() -> Vec<String> {
    let content = match std::fs::read_to_string("/etc/passwd") {
        Ok(c) => c,
        Err(_) => return vec!["(cannot read users)".to_string()],
    };

    let mut users = Vec::new();
    for line in content.lines() {
        let fields: Vec<&str> = line.split(':').collect();
        if fields.len() < 7 {
            continue;
        }
        let name = fields[0];
        let uid: u32 = fields[2].parse().unwrap_or(0);
        let shell = fields[6];

        // Regular users: UID >= 1000, not nobody, has a real shell
        if uid >= 1000
            && name != "nobody"
            && !shell.ends_with("nologin")
            && !shell.ends_with("false")
        {
            let home = fields[5];
            users.push(format!("{} ({})", name, home));
        }
    }

    if users.is_empty() {
        users.push("(no users found)".to_string());
    }

    users
}

/// Detect available login shells from /etc/shells
pub fn detect_shells() -> Vec<String> {
    let content = match std::fs::read_to_string("/etc/shells") {
        Ok(c) => c,
        Err(_) => return vec!["/bin/bash".to_string(), "/bin/zsh".to_string(), "/bin/fish".to_string()],
    };

    let mut shells: Vec<String> = content
        .lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .map(|l| l.trim().to_string())
        .collect();

    if shells.is_empty() {
        shells.push("/bin/bash".to_string());
    }

    shells
}

/// Detect common system groups
pub fn detect_groups() -> Vec<String> {
    let content = match std::fs::read_to_string("/etc/group") {
        Ok(c) => c,
        Err(_) => {
            return vec![
                "wheel".to_string(), "audio".to_string(), "video".to_string(),
                "docker".to_string(), "storage".to_string(), "network".to_string(),
            ];
        }
    };

    let mut groups = Vec::new();
    for line in content.lines() {
        let fields: Vec<&str> = line.split(':').collect();
        if fields.len() < 3 {
            continue;
        }
        let name = fields[0];
        let gid: u32 = fields[2].parse().unwrap_or(0);

        // Include well-known system groups and all user groups (GID >= 1000)
        let well_known = [
            "wheel", "audio", "video", "storage", "network", "optical",
            "scanner", "power", "lp", "docker", "libvirt", "kvm",
            "input", "plugdev", "uucp", "games", "users", "http",
        ];

        if well_known.contains(&name) || gid >= 1000 {
            groups.push(name.to_string());
        }
    }

    if groups.is_empty() {
        groups.push("wheel".to_string());
    }

    groups
}

/// Detect systemd services (enabled + running)
pub fn detect_services() -> Vec<String> {
    let output = match Command::new("systemctl")
        .args(["list-unit-files", "--type=service", "--no-pager", "--no-legend"])
        .output()
    {
        Ok(o) => o,
        Err(_) => return vec!["(cannot list services)".to_string()],
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut services = Vec::new();

    for line in stdout.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 2 {
            continue;
        }
        let name = parts[0].strip_suffix(".service").unwrap_or(parts[0]);
        let state = parts[1];

        // Skip template services and static ones
        if name.contains('@') || state == "static" || state == "indirect" {
            continue;
        }

        let entry = format!("{} [{}]", name, state);
        services.push(entry);
    }

    if services.is_empty() {
        services.push("(no services found)".to_string());
    }

    services
}

impl Default for Configuration {
    fn default() -> Self {
        Self {
            options: vec![
                // =============================================
                // Boot Setup (0-1)
                // =============================================
                ConfigOption::new("Boot Mode", true, "Boot firmware type (Auto/UEFI/BIOS)", "Auto")
                    .with_options(&["Auto", "UEFI", "BIOS"]),
                ConfigOption::new("Secure Boot", false,
                    "Enable Secure Boot (requires UEFI firmware configuration)", "No")
                    .with_options(&["No", "Yes"]),

                // =============================================
                // System Locale and Input (2-3)
                // =============================================
                ConfigOption::new("Locale", true, "System locale", "en_US.UTF-8")
                    .with_options(&[
                        "en_US.UTF-8", "en_GB.UTF-8", "en_AU.UTF-8", "en_CA.UTF-8",
                        "de_DE.UTF-8", "fr_FR.UTF-8", "es_ES.UTF-8", "it_IT.UTF-8",
                        "pt_BR.UTF-8", "nl_NL.UTF-8", "sv_SE.UTF-8", "nb_NO.UTF-8",
                        "da_DK.UTF-8", "fi_FI.UTF-8", "pl_PL.UTF-8", "cs_CZ.UTF-8",
                        "hu_HU.UTF-8", "ro_RO.UTF-8", "tr_TR.UTF-8", "ru_RU.UTF-8",
                        "ja_JP.UTF-8", "zh_CN.UTF-8", "ko_KR.UTF-8",
                    ]),
                ConfigOption::new("Keymap", true, "Keyboard layout", "us")
                    .with_options(&[
                        "us", "uk", "de", "fr", "es", "it", "pt", "br", "ca",
                        "se", "no", "fi", "dk", "pl", "cz", "hu", "ro", "tr",
                        "ru", "kr", "jp", "dvorak",
                    ]),

                // =============================================
                // Disk and Storage (4-19)
                // =============================================
                {
                    let disks = detect_disks();
                    let disk_refs: Vec<&str> = disks.iter().map(|s| s.as_str()).collect();
                    ConfigOption::new("Disk", true, "Target disk for installation", "")
                        .with_options(&disk_refs)
                },
                ConfigOption::new("Partitioning Strategy", true, "How to partition the disk", "")
                    .with_options(&[
                        "auto_simple", "auto_simple_luks", "auto_lvm", "auto_luks_lvm",
                        "auto_raid", "auto_raid_luks", "auto_raid_lvm", "auto_raid_lvm_luks",
                        "manual",
                    ]),
                ConfigOption::new("RAID Level", false, "RAID array level", "")
                    .with_options(&["raid0", "raid1", "raid5", "raid6", "raid10"]),
                ConfigOption::new("Encryption", false, "Enable disk encryption", "No")
                    .with_options(&["Auto", "Yes", "No"]),
                ConfigOption::new("Encryption Password", false, "LUKS encryption passphrase", ""),
                ConfigOption::new("Root Filesystem", true, "Root partition filesystem", "ext4")
                    .with_options(&["ext4", "xfs", "btrfs"]),
                ConfigOption::new("Separate Home Partition", false, "Create separate /home partition", "No")
                    .with_options(&["Yes", "No"]),
                ConfigOption::new("Home Filesystem", false, "Home partition filesystem", "")
                    .with_options(&["ext4", "xfs", "btrfs"]),
                ConfigOption::new("Swap", false, "Enable swap partition", "No")
                    .with_options(&["Yes", "No"]),
                ConfigOption::new("Swap Size", false, "Swap partition size", "")
                    .with_options(&["1GB", "2GB", "4GB", "8GB", "16GB", "32GB", "Equal to RAM", "Double RAM"]),
                ConfigOption::new("Root Size", false, "Root partition size", "")
                    .with_options(&["30GB", "50GB", "80GB", "100GB", "150GB", "200GB", "Remaining"]),
                ConfigOption::new("Home Size", false, "Home partition size", "")
                    .with_options(&["50GB", "100GB", "200GB", "500GB", "1TB", "Remaining"]),
                ConfigOption::new("Btrfs Snapshots", false, "Enable Btrfs snapshots", "No")
                    .with_options(&["Yes", "No"]),
                ConfigOption::new("Btrfs Frequency", false, "Btrfs snapshot frequency", "")
                    .with_options(&["hourly", "daily", "weekly", "monthly"]),
                ConfigOption::new("Btrfs Keep Count", false, "Number of snapshots to keep", "")
                    .with_options(&["3", "5", "10", "20"]),
                ConfigOption::new("Btrfs Assistant", false, "Use Btrfs assistant", "No")
                    .with_options(&["Yes", "No"]),

                // =============================================
                // Time and Location (20-22)
                // =============================================
                ConfigOption::new("Timezone Region", true, "Timezone region", "")
                    .with_options(&[
                        "Africa", "America", "Antarctica", "Arctic", "Asia",
                        "Atlantic", "Australia", "Europe", "Indian", "Pacific", "US",
                    ]),
                ConfigOption::new("Timezone", true, "Timezone city (select region first)", ""),
                ConfigOption::new("Time Sync (NTP)", false, "Enable NTP time synchronization", "No")
                    .with_options(&["Yes", "No"]),

                // =============================================
                // System Packages (23-27)
                // =============================================
                ConfigOption::new("Mirror Country", true, "Package mirror country", "United States")
                    .with_options(&[
                        "Australia", "Austria", "Brazil", "Canada", "China",
                        "Denmark", "Finland", "France", "Germany", "Greece",
                        "Hong Kong", "Hungary", "India", "Ireland", "Italy",
                        "Japan", "Korea", "Netherlands", "New Zealand", "Norway",
                        "Poland", "Portugal", "Russia", "Singapore", "Spain",
                        "Sweden", "Switzerland", "Taiwan", "United Kingdom", "United States",
                    ]),
                ConfigOption::new("Kernel", true, "Linux kernel to install", "linux")
                    .with_options(&["linux", "linux-lts", "linux-zen", "linux-hardened"]),
                ConfigOption::new("Multilib", false, "Enable multilib repository", "No")
                    .with_options(&["Yes", "No"]),
                ConfigOption::new("Additional Pacman Packages", false,
                    "Interactive package search: search/add/remove/list/done", ""),
                ConfigOption::new("GPU Drivers", false, "Graphics drivers", "Auto")
                    .with_options(&["Auto", "NVIDIA", "nvidia-open", "AMD", "Intel", "nouveau", "None"]),

                // =============================================
                // Hostname (28)
                // =============================================
                ConfigOption::new("Hostname", true, "System hostname", ""),

                // =============================================
                // User Setup (29-31)
                // =============================================
                ConfigOption::new("Username", true, "Primary user account", ""),
                ConfigOption::new("User Password", true, "User account password", ""),
                ConfigOption::new("Root Password", true, "Root account password", ""),

                // =============================================
                // Package Management (32-34)
                // =============================================
                ConfigOption::new("AUR Helper", false, "AUR package helper", "none")
                    .with_options(&["paru", "yay", "pikaur", "none"]),
                ConfigOption::new("Additional AUR Packages", false,
                    "Interactive AUR package search: search/add/remove/list/done", ""),
                ConfigOption::new("Flatpak", false, "Enable Flatpak support", "No")
                    .with_options(&["Yes", "No"]),

                // =============================================
                // Boot Configuration (35-38)
                // =============================================
                ConfigOption::new("Bootloader", true, "Boot loader", "grub")
                    .with_options(&["grub", "systemd-boot"]),
                ConfigOption::new("OS Prober", false, "Enable OS detection (dual-boot)", "No")
                    .with_options(&["Yes", "No"]),
                ConfigOption::new("GRUB Theme", false, "Enable GRUB themes", "No")
                    .with_options(&["Yes", "No"]),
                ConfigOption::new("GRUB Theme Selection", false, "GRUB theme to use", "PolyDark")
                    .with_options(&["PolyDark", "CyberEXS", "CyberPunk", "HyperFluent", "none"]),

                // =============================================
                // Desktop Environment (39-40)
                // =============================================
                ConfigOption::new("Desktop Environment", false, "Desktop environment or window manager", "none")
                    .with_options(&[
                        "none", "gnome", "kde", "hyprland", "sway", "i3",
                        "xfce", "cinnamon", "mate", "budgie",
                    ]),
                ConfigOption::new("Display Manager", false, "Display manager (login screen)", "none")
                    .with_options(&["none", "gdm", "sddm", "lightdm", "lxdm", "ly"]),

                // =============================================
                // Boot Splash and Final Setup (41-45)
                // =============================================
                ConfigOption::new("Plymouth", false, "Boot splash screen", "No")
                    .with_options(&["Yes", "No"]),
                ConfigOption::new("Plymouth Theme", false, "Plymouth boot theme", "arch-glow")
                    .with_options(&["arch-glow", "arch-mac-style", "none"]),
                ConfigOption::new("Numlock on Boot", false, "Enable numlock at boot", "No")
                    .with_options(&["Yes", "No"]),
                ConfigOption::new("Git Repository", false, "Clone dotfiles/config repository", "No")
                    .with_options(&["Yes", "No"]),
                ConfigOption::new("Git Repository URL", false, "Git repository URL to clone", ""),
            ],
        }
    }
}
