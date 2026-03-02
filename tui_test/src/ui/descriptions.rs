//! Tool description text for visual testing
//!
//! Each tool category has a distinct accent color.
//! Descriptions use consistent formatting with the new theme.

use crate::theme::{Colors, Styles};
use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};

pub fn get_tools_category_description(selection: usize) -> Vec<Line<'static>> {
    match selection {
        0 => desc_block(
            "Disk & Filesystem Tools",
            "Manage disk partitions and filesystems.",
            Colors::CAT_DISK,
            &[
                "Partition Disk    - Create/delete partitions",
                "Format Partition  - Create filesystems",
                "Wipe Disk         - Secure data erasure",
                "Check Health      - SMART diagnostics",
                "Mount/Unmount     - Manage mount points",
                "LUKS Encryption   - Encrypt partitions",
            ],
        ),
        1 => desc_block(
            "System Configuration Tools",
            "Configure system components and services.",
            Colors::CAT_SYSTEM,
            &[
                "Install Bootloader - GRUB/systemd-boot",
                "Generate fstab     - Filesystem table",
                "Chroot into System - Enter installed system",
                "Manage Services    - systemctl interface",
                "System Info        - Hardware detection",
            ],
        ),
        2 => desc_block(
            "User & Security Tools",
            "Manage users, groups, and security settings.",
            Colors::CAT_USER,
            &[
                "Add User       - Create user accounts",
                "Reset Password - Change passwords",
                "Manage Groups  - Group membership",
                "Configure SSH  - SSH server setup",
                "Security Audit - System hardening",
            ],
        ),
        3 => desc_block(
            "Network Configuration Tools",
            "Configure network interfaces and services.",
            Colors::CAT_NETWORK,
            &[
                "Configure Network - Interface setup",
                "Test Connectivity - Ping/DNS/HTTP tests",
                "Firewall Rules    - iptables/ufw config",
                "Network Info      - Interface diagnostics",
                "Update Mirrors    - Pacman mirrorlist",
            ],
        ),
        _ => vec![
            Line::from(""),
            Line::from(Span::styled(
                "  Select a category",
                Styles::text_muted(),
            )),
        ],
    }
}

pub fn get_disk_tool_description(selection: usize) -> Vec<Line<'static>> {
    match selection {
        0 => tool_desc(
            "Partition Disk",
            "Create, delete, and manage disk partitions using GPT or MBR partition tables. Supports cfdisk for interactive partitioning.",
            Colors::CAT_DISK,
        ),
        1 => tool_desc(
            "Format Partition",
            "Create filesystems on partitions. Supports ext4, xfs, btrfs, FAT32, and NTFS.",
            Colors::CAT_DISK,
        ),
        2 => tool_desc(
            "Wipe Disk",
            "Securely erase all data from a disk. Supports quick wipe (zero-fill) and secure wipe (random data).",
            Colors::CAT_DISK,
        ),
        3 => tool_desc(
            "Check Disk Health",
            "Run SMART diagnostics on storage devices. Shows health status, temperature, and wear indicators.",
            Colors::CAT_DISK,
        ),
        4 => tool_desc(
            "Mount/Unmount",
            "Mount and unmount partitions to the filesystem tree. Handles /mnt for installation targets.",
            Colors::CAT_DISK,
        ),
        5 => tool_desc(
            "LUKS Encryption",
            "Set up LUKS2 disk encryption. Create encrypted partitions, open/close encrypted volumes.",
            Colors::CAT_DISK,
        ),
        _ => tool_desc("Back", "Return to the Tools Menu.", Colors::FG_SECONDARY),
    }
}

pub fn get_system_tool_description(selection: usize) -> Vec<Line<'static>> {
    match selection {
        0 => tool_desc(
            "Install Bootloader",
            "Install GRUB or systemd-boot. Detects UEFI/BIOS mode automatically.",
            Colors::CAT_SYSTEM,
        ),
        1 => tool_desc(
            "Generate fstab",
            "Generate /etc/fstab from current mount points using genfstab -U.",
            Colors::CAT_SYSTEM,
        ),
        2 => tool_desc(
            "Chroot into System",
            "Enter the installed system via arch-chroot for manual configuration.",
            Colors::CAT_SYSTEM,
        ),
        3 => tool_desc(
            "Manage Services",
            "Enable, disable, start, or stop systemd services.",
            Colors::CAT_SYSTEM,
        ),
        4 => tool_desc(
            "System Info",
            "Display hardware information including CPU, memory, GPU, and storage.",
            Colors::CAT_SYSTEM,
        ),
        5 => tool_desc(
            "Enable Services",
            "Batch-enable services for the installed system (NetworkManager, etc).",
            Colors::CAT_SYSTEM,
        ),
        6 => tool_desc(
            "Install AUR Helper",
            "Install paru, yay, or pikaur for AUR package management.",
            Colors::CAT_SYSTEM,
        ),
        7 => tool_desc(
            "Rebuild Initramfs",
            "Regenerate initramfs images with mkinitcpio -P.",
            Colors::CAT_SYSTEM,
        ),
        8 => tool_desc(
            "View Install Logs",
            "Browse installation log files from /var/log/archtui/.",
            Colors::CAT_SYSTEM,
        ),
        _ => tool_desc("Back", "Return to the Tools Menu.", Colors::FG_SECONDARY),
    }
}

pub fn get_user_tool_description(selection: usize) -> Vec<Line<'static>> {
    match selection {
        0 => tool_desc(
            "Add User",
            "Create a new user account with home directory and group membership.",
            Colors::CAT_USER,
        ),
        1 => tool_desc(
            "Reset Password",
            "Change the password for an existing user account.",
            Colors::CAT_USER,
        ),
        2 => tool_desc(
            "Manage Groups",
            "Add or remove users from system groups (wheel, audio, video, etc).",
            Colors::CAT_USER,
        ),
        3 => tool_desc(
            "Configure SSH",
            "Set up OpenSSH server with key-based authentication and hardened config.",
            Colors::CAT_USER,
        ),
        4 => tool_desc(
            "Security Audit",
            "Run security checks on the system configuration.",
            Colors::CAT_USER,
        ),
        5 => tool_desc(
            "Install Dotfiles",
            "Clone and install dotfiles from a git repository.",
            Colors::CAT_USER,
        ),
        6 => tool_desc(
            "Run As User",
            "Execute a command as a specified user.",
            Colors::CAT_USER,
        ),
        _ => tool_desc("Back", "Return to the Tools Menu.", Colors::FG_SECONDARY),
    }
}

pub fn get_network_tool_description(selection: usize) -> Vec<Line<'static>> {
    match selection {
        0 => tool_desc(
            "Configure Network",
            "Set up network interfaces, DNS, and routing.",
            Colors::CAT_NETWORK,
        ),
        1 => tool_desc(
            "Test Connectivity",
            "Test network with ping, DNS resolution, and HTTP checks.",
            Colors::CAT_NETWORK,
        ),
        2 => tool_desc(
            "Firewall Rules",
            "Configure iptables/nftables firewall rules. Supports UFW frontend.",
            Colors::CAT_NETWORK,
        ),
        3 => tool_desc(
            "Network Info",
            "Display network interface information, routing tables, and DNS config.",
            Colors::CAT_NETWORK,
        ),
        4 => tool_desc(
            "Update Mirrors",
            "Update pacman mirrorlist using reflector for fastest mirrors.",
            Colors::CAT_NETWORK,
        ),
        _ => tool_desc("Back", "Return to the Tools Menu.", Colors::FG_SECONDARY),
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────

fn desc_block(
    title: &'static str,
    subtitle: &'static str,
    accent: ratatui::style::Color,
    items: &[&'static str],
) -> Vec<Line<'static>> {
    let mut lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("  {}", title),
            Style::default().fg(accent).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            format!("  {}", subtitle),
            Styles::text(),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  Available tools:",
            Style::default()
                .fg(Colors::FG_SECONDARY)
                .add_modifier(Modifier::BOLD),
        )),
    ];
    for item in items {
        lines.push(Line::from(vec![
            Span::styled("  \u{2022} ", Style::default().fg(accent)),
            Span::styled(*item, Styles::text_secondary()),
        ]));
    }
    lines
}

fn tool_desc(
    name: &'static str,
    desc: &'static str,
    accent: ratatui::style::Color,
) -> Vec<Line<'static>> {
    vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("  {}", name),
            Style::default().fg(accent).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(format!("  {}", desc), Styles::text())),
    ]
}
