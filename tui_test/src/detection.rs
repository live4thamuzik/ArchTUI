//! System detection module
//!
//! Functions for detecting hardware, users, services, and disk layouts
//! by querying lsblk, ip, systemctl, and /etc/ files.

#![allow(dead_code)]

use std::process::Command;

/// Get timezone cities for a given region from /usr/share/zoneinfo
pub fn get_timezones_for_region(region: &str) -> Vec<String> {
    // Try reading from the actual filesystem first
    let path = format!("/usr/share/zoneinfo/{}", region);
    if let Ok(entries) = std::fs::read_dir(&path) {
        let mut cities: Vec<String> = entries
            .filter_map(|e| e.ok())
            .filter(|e| {
                // Include files and directories (for sub-regions like America/Indiana)
                let name = e.file_name().to_string_lossy().to_string();
                !name.starts_with('.') && name != "posixrules"
            })
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();
        cities.sort();
        if !cities.is_empty() {
            return cities;
        }
    }

    // Fallback: hardcoded common cities
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
        "Antarctica" => &["Casey", "Davis", "McMurdo", "Palmer", "Rothera", "Vostok"],
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
        _ => &["New_York"],
    };
    cities.iter().map(|s| s.to_string()).collect()
}

/// Detect available disks via lsblk
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

        if dtype != "disk" || ro == "1" {
            continue;
        }

        // Skip USB transport (likely live ISO media)
        if parts.len() >= 5 && parts[4] == "usb" {
            continue;
        }

        let disk_path = format!("/dev/{}", name);

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
pub fn get_disk_layout(device_str: &str) -> Vec<String> {
    let dev_path = device_str
        .split_whitespace()
        .next()
        .unwrap_or(device_str);

    if dev_path.is_empty() || dev_path.starts_with('(') {
        return vec!["No device selected".to_string()];
    }

    let mut lines = Vec::new();

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

    if let Ok(output) = Command::new("lsblk")
        .args(["-o", "NAME,SIZE,TYPE,FSTYPE,LABEL,MOUNTPOINTS", dev_path])
        .output()
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for (i, line) in stdout.lines().enumerate() {
            if i == 0 {
                lines.push(format!("  {}", line));
                lines.push(format!("  {}", "\u{2500}".repeat(line.len().min(50))));
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

/// Detect available partitions via lsblk
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

        let raw_name = parts[0].trim_start_matches(|c: char| !c.is_alphanumeric());
        if raw_name.is_empty() {
            continue;
        }

        let size = parts[1];
        let ptype = parts[2];

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

/// Detect network interfaces via ip link / /sys/class/net
pub fn detect_interfaces() -> Vec<String> {
    let output = match Command::new("ip")
        .args(["-o", "link", "show"])
        .output()
    {
        Ok(o) => o,
        Err(_) => {
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
        let parts: Vec<&str> = line.splitn(3, ':').collect();
        if parts.len() < 2 {
            continue;
        }
        let name = parts[1].trim();
        if name == "lo" {
            continue;
        }

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

/// Detect common system groups from /etc/group
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
