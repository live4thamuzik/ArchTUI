//! Tool description text generation
//!
//! This module contains functions that generate description text
//! for various tool categories and individual tools.

use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

/// Get description for tools category
pub fn get_tools_category_description(selection: usize) -> Vec<Line<'static>> {
    match selection {
        0 => vec![
            Line::from(""),
            Line::from(vec![Span::styled(
                "  Disk & Filesystem Tools",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "  Manage disk partitions and filesystems.",
                Style::default().fg(Color::White),
            )]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "  Available tools:",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![Span::styled(
                "  • Partition Disk    - Create/delete partitions",
                Style::default().fg(Color::Gray),
            )]),
            Line::from(vec![Span::styled(
                "  • Format Partition  - Create filesystems",
                Style::default().fg(Color::Gray),
            )]),
            Line::from(vec![Span::styled(
                "  • Wipe Disk         - Secure data erasure",
                Style::default().fg(Color::Gray),
            )]),
            Line::from(vec![Span::styled(
                "  • Check Health      - SMART diagnostics",
                Style::default().fg(Color::Gray),
            )]),
            Line::from(vec![Span::styled(
                "  • Mount/Unmount     - Manage mount points",
                Style::default().fg(Color::Gray),
            )]),
        ],
        1 => vec![
            Line::from(""),
            Line::from(vec![Span::styled(
                "  System Configuration Tools",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "  Configure system components and boot settings.",
                Style::default().fg(Color::White),
            )]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "  Available tools:",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![Span::styled(
                "  • Install Bootloader - GRUB/systemd-boot",
                Style::default().fg(Color::Gray),
            )]),
            Line::from(vec![Span::styled(
                "  • Generate fstab     - Auto-mount config",
                Style::default().fg(Color::Gray),
            )]),
            Line::from(vec![Span::styled(
                "  • Chroot            - Enter installed system",
                Style::default().fg(Color::Gray),
            )]),
            Line::from(vec![Span::styled(
                "  • Manage Services   - systemd services",
                Style::default().fg(Color::Gray),
            )]),
            Line::from(vec![Span::styled(
                "  • System Info       - Hardware details",
                Style::default().fg(Color::Gray),
            )]),
        ],
        2 => vec![
            Line::from(""),
            Line::from(vec![Span::styled(
                "  User & Security Tools",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "  Manage users, groups, and security settings.",
                Style::default().fg(Color::White),
            )]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "  Available tools:",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![Span::styled(
                "  • Add User        - Create user accounts",
                Style::default().fg(Color::Gray),
            )]),
            Line::from(vec![Span::styled(
                "  • Reset Password  - Change passwords",
                Style::default().fg(Color::Gray),
            )]),
            Line::from(vec![Span::styled(
                "  • Manage Groups   - Group memberships",
                Style::default().fg(Color::Gray),
            )]),
            Line::from(vec![Span::styled(
                "  • Configure SSH   - SSH keys & config",
                Style::default().fg(Color::Gray),
            )]),
            Line::from(vec![Span::styled(
                "  • Security Audit  - Check vulnerabilities",
                Style::default().fg(Color::Gray),
            )]),
        ],
        3 => vec![
            Line::from(""),
            Line::from(vec![Span::styled(
                "  Network Configuration Tools",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "  Configure networking and connectivity.",
                Style::default().fg(Color::White),
            )]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "  Available tools:",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![Span::styled(
                "  • Configure Network - Interface setup",
                Style::default().fg(Color::Gray),
            )]),
            Line::from(vec![Span::styled(
                "  • Test Connectivity - Ping & diagnostics",
                Style::default().fg(Color::Gray),
            )]),
            Line::from(vec![Span::styled(
                "  • Firewall Rules   - Security policies",
                Style::default().fg(Color::Gray),
            )]),
            Line::from(vec![Span::styled(
                "  • Network Info     - Current settings",
                Style::default().fg(Color::Gray),
            )]),
        ],
        _ => vec![
            Line::from(""),
            Line::from(vec![Span::styled(
                "  Return to Main Menu",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "  Go back to the main menu to choose",
                Style::default().fg(Color::Gray),
            )]),
            Line::from(vec![Span::styled(
                "  a different installation method.",
                Style::default().fg(Color::Gray),
            )]),
        ],
    }
}

/// Get description for disk tool
pub fn get_disk_tool_description(selection: usize) -> Vec<Line<'static>> {
    match selection {
        0 => partition_disk_description(),
        1 => format_partition_description(),
        2 => wipe_disk_description(),
        3 => check_disk_health_description(),
        4 => mount_unmount_description(),
        _ => back_to_menu_description("Tools Menu"),
    }
}

/// Get description for system tool
pub fn get_system_tool_description(selection: usize) -> Vec<Line<'static>> {
    match selection {
        0 => install_bootloader_description(),
        1 => generate_fstab_description(),
        2 => chroot_description(),
        3 => manage_services_description(),
        4 => system_info_description(),
        _ => back_to_menu_description("Tools Menu"),
    }
}

/// Get description for user tool
pub fn get_user_tool_description(selection: usize) -> Vec<Line<'static>> {
    match selection {
        0 => add_user_description(),
        1 => reset_password_description(),
        2 => manage_groups_description(),
        3 => configure_ssh_description(),
        4 => security_audit_description(),
        _ => back_to_menu_description("Tools Menu"),
    }
}

/// Get description for network tool
pub fn get_network_tool_description(selection: usize) -> Vec<Line<'static>> {
    match selection {
        0 => configure_network_description(),
        1 => test_connectivity_description(),
        2 => firewall_rules_description(),
        3 => network_info_description(),
        _ => back_to_menu_description("Tools Menu"),
    }
}

// Individual tool descriptions

fn partition_disk_description() -> Vec<Line<'static>> {
    vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Partition Disk (cfdisk)",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Interactive partition editor for creating, deleting,",
            Style::default().fg(Color::White),
        )]),
        Line::from(vec![Span::styled(
            "  and resizing disk partitions.",
            Style::default().fg(Color::White),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Usage:",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![Span::styled(
            "  • Use arrow keys to navigate partitions",
            Style::default().fg(Color::Gray),
        )]),
        Line::from(vec![Span::styled(
            "  • [New] to create a new partition",
            Style::default().fg(Color::Gray),
        )]),
        Line::from(vec![Span::styled(
            "  • [Delete] to remove a partition",
            Style::default().fg(Color::Gray),
        )]),
        Line::from(vec![Span::styled(
            "  • [Write] to save changes to disk",
            Style::default().fg(Color::Gray),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  ⚠️  ", Style::default().fg(Color::Red)),
            Span::styled(
                "Warning: Changes are permanent after [Write]",
                Style::default().fg(Color::Red),
            ),
        ]),
    ]
}

fn format_partition_description() -> Vec<Line<'static>> {
    vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Format Partition",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Format a partition with a filesystem.",
            Style::default().fg(Color::White),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Supported filesystems:",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![Span::styled(
            "  • ext4    - Standard Linux filesystem (recommended)",
            Style::default().fg(Color::Gray),
        )]),
        Line::from(vec![Span::styled(
            "  • btrfs   - Copy-on-write with snapshots",
            Style::default().fg(Color::Gray),
        )]),
        Line::from(vec![Span::styled(
            "  • xfs     - High-performance filesystem",
            Style::default().fg(Color::Gray),
        )]),
        Line::from(vec![Span::styled(
            "  • fat32   - For EFI system partitions",
            Style::default().fg(Color::Gray),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  ⚠️  ", Style::default().fg(Color::Red)),
            Span::styled(
                "Warning: All data on partition will be erased!",
                Style::default().fg(Color::Red),
            ),
        ]),
    ]
}

fn wipe_disk_description() -> Vec<Line<'static>> {
    vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Wipe Disk (Secure Erase)",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Securely erase all data on a disk.",
            Style::default().fg(Color::White),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Methods:",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![Span::styled(
            "  • Zero fill    - Fast, single pass of zeros",
            Style::default().fg(Color::Gray),
        )]),
        Line::from(vec![Span::styled(
            "  • Random fill  - More secure, random data",
            Style::default().fg(Color::Gray),
        )]),
        Line::from(vec![Span::styled(
            "  • ATA Secure   - Hardware-level secure erase",
            Style::default().fg(Color::Gray),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  🚨 ", Style::default().fg(Color::Red)),
            Span::styled(
                "DANGER: This operation is IRREVERSIBLE!",
                Style::default()
                    .fg(Color::Red)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
    ]
}

fn check_disk_health_description() -> Vec<Line<'static>> {
    vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Check Disk Health (SMART)",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Run SMART diagnostics on a disk drive.",
            Style::default().fg(Color::White),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Information provided:",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![Span::styled(
            "  • Overall health status",
            Style::default().fg(Color::Gray),
        )]),
        Line::from(vec![Span::styled(
            "  • Power-on hours",
            Style::default().fg(Color::Gray),
        )]),
        Line::from(vec![Span::styled(
            "  • Reallocated sector count",
            Style::default().fg(Color::Gray),
        )]),
        Line::from(vec![Span::styled(
            "  • Temperature readings",
            Style::default().fg(Color::Gray),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  ℹ️  ", Style::default().fg(Color::Cyan)),
            Span::styled(
                "Safe to run - does not modify disk",
                Style::default().fg(Color::Cyan),
            ),
        ]),
    ]
}

fn mount_unmount_description() -> Vec<Line<'static>> {
    vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Mount/Unmount Partitions",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Mount partitions to access their contents.",
            Style::default().fg(Color::White),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Common mount points:",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![Span::styled(
            "  • /mnt        - Temporary mount point",
            Style::default().fg(Color::Gray),
        )]),
        Line::from(vec![Span::styled(
            "  • /mnt/boot   - Boot partition",
            Style::default().fg(Color::Gray),
        )]),
        Line::from(vec![Span::styled(
            "  • /mnt/home   - Home partition",
            Style::default().fg(Color::Gray),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "  Tip:",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " Mount root (/) first, then others",
                Style::default().fg(Color::Cyan),
            ),
        ]),
    ]
}

fn install_bootloader_description() -> Vec<Line<'static>> {
    vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Install Bootloader",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Install a bootloader to make your system bootable.",
            Style::default().fg(Color::White),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Available bootloaders:",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![Span::styled(
            "  • GRUB         - Traditional, feature-rich",
            Style::default().fg(Color::Gray),
        )]),
        Line::from(vec![Span::styled(
            "  • systemd-boot - Simple, fast UEFI boot manager",
            Style::default().fg(Color::Gray),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Requirements:",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![Span::styled(
            "  • Root partition mounted at /mnt",
            Style::default().fg(Color::Gray),
        )]),
    ]
}

fn generate_fstab_description() -> Vec<Line<'static>> {
    vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Generate fstab",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Generate /etc/fstab for automatic mounting.",
            Style::default().fg(Color::White),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Identification methods:",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![Span::styled(
            "  • UUID    - Universally unique identifier",
            Style::default().fg(Color::Gray),
        )]),
        Line::from(vec![Span::styled(
            "  • LABEL   - Filesystem label",
            Style::default().fg(Color::Gray),
        )]),
        Line::from(vec![Span::styled(
            "  • PARTUUID - Partition UUID (GPT only)",
            Style::default().fg(Color::Gray),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "  Tip:",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " Mount all partitions before generating",
                Style::default().fg(Color::Cyan),
            ),
        ]),
    ]
}

fn chroot_description() -> Vec<Line<'static>> {
    vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Chroot into System",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Enter an installed system for maintenance.",
            Style::default().fg(Color::White),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Common uses:",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![Span::styled(
            "  • Fix broken bootloader",
            Style::default().fg(Color::Gray),
        )]),
        Line::from(vec![Span::styled(
            "  • Reset forgotten password",
            Style::default().fg(Color::Gray),
        )]),
        Line::from(vec![Span::styled(
            "  • Install/remove packages",
            Style::default().fg(Color::Gray),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "  Exit:",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " Type 'exit' or press Ctrl+D",
                Style::default().fg(Color::Cyan),
            ),
        ]),
    ]
}

fn manage_services_description() -> Vec<Line<'static>> {
    vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Manage Services",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Enable or disable systemd services.",
            Style::default().fg(Color::White),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Common services:",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![Span::styled(
            "  • NetworkManager   - Network management",
            Style::default().fg(Color::Gray),
        )]),
        Line::from(vec![Span::styled(
            "  • sshd             - SSH server",
            Style::default().fg(Color::Gray),
        )]),
        Line::from(vec![Span::styled(
            "  • bluetooth        - Bluetooth support",
            Style::default().fg(Color::Gray),
        )]),
        Line::from(vec![Span::styled(
            "  • gdm/sddm         - Display managers",
            Style::default().fg(Color::Gray),
        )]),
    ]
}

fn system_info_description() -> Vec<Line<'static>> {
    vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            "  System Information",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Display detailed system information.",
            Style::default().fg(Color::White),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Information shown:",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![Span::styled(
            "  • CPU model and cores",
            Style::default().fg(Color::Gray),
        )]),
        Line::from(vec![Span::styled(
            "  • Memory (RAM) size",
            Style::default().fg(Color::Gray),
        )]),
        Line::from(vec![Span::styled(
            "  • Disk information",
            Style::default().fg(Color::Gray),
        )]),
        Line::from(vec![Span::styled(
            "  • Boot mode (UEFI/BIOS)",
            Style::default().fg(Color::Gray),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  ℹ️  ", Style::default().fg(Color::Cyan)),
            Span::styled(
                "Read-only - no changes made",
                Style::default().fg(Color::Cyan),
            ),
        ]),
    ]
}

fn add_user_description() -> Vec<Line<'static>> {
    vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Add User",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Create a new user account.",
            Style::default().fg(Color::White),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Options:",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![Span::styled(
            "  • Username        - Login name",
            Style::default().fg(Color::Gray),
        )]),
        Line::from(vec![Span::styled(
            "  • Home directory  - User's home folder",
            Style::default().fg(Color::Gray),
        )]),
        Line::from(vec![Span::styled(
            "  • Shell           - Default login shell",
            Style::default().fg(Color::Gray),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "  Tip:",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " Add to 'wheel' for sudo access",
                Style::default().fg(Color::Cyan),
            ),
        ]),
    ]
}

fn reset_password_description() -> Vec<Line<'static>> {
    vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Reset Password",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Reset or change a user's password.",
            Style::default().fg(Color::White),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Use cases:",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![Span::styled(
            "  • Forgotten password recovery",
            Style::default().fg(Color::Gray),
        )]),
        Line::from(vec![Span::styled(
            "  • Set initial password",
            Style::default().fg(Color::Gray),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  ⚠️  ", Style::default().fg(Color::Yellow)),
            Span::styled(
                "Requires root/sudo privileges",
                Style::default().fg(Color::Yellow),
            ),
        ]),
    ]
}

fn manage_groups_description() -> Vec<Line<'static>> {
    vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Manage Groups",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Add or remove users from system groups.",
            Style::default().fg(Color::White),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Common groups:",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![Span::styled(
            "  • wheel    - Sudo/admin privileges",
            Style::default().fg(Color::Gray),
        )]),
        Line::from(vec![Span::styled(
            "  • audio    - Audio device access",
            Style::default().fg(Color::Gray),
        )]),
        Line::from(vec![Span::styled(
            "  • video    - Video device access",
            Style::default().fg(Color::Gray),
        )]),
        Line::from(vec![Span::styled(
            "  • docker   - Docker management",
            Style::default().fg(Color::Gray),
        )]),
    ]
}

fn configure_ssh_description() -> Vec<Line<'static>> {
    vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Configure SSH",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Set up SSH keys and server configuration.",
            Style::default().fg(Color::White),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Features:",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![Span::styled(
            "  • Generate SSH key pairs",
            Style::default().fg(Color::Gray),
        )]),
        Line::from(vec![Span::styled(
            "  • Import authorized keys",
            Style::default().fg(Color::Gray),
        )]),
        Line::from(vec![Span::styled(
            "  • Configure sshd settings",
            Style::default().fg(Color::Gray),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  🔐 ", Style::default().fg(Color::Green)),
            Span::styled(
                "Key-based auth is more secure",
                Style::default().fg(Color::Green),
            ),
        ]),
    ]
}

fn security_audit_description() -> Vec<Line<'static>> {
    vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Security Audit",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Check system security settings.",
            Style::default().fg(Color::White),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Checks performed:",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![Span::styled(
            "  • Password policy",
            Style::default().fg(Color::Gray),
        )]),
        Line::from(vec![Span::styled(
            "  • File permissions",
            Style::default().fg(Color::Gray),
        )]),
        Line::from(vec![Span::styled(
            "  • Running services",
            Style::default().fg(Color::Gray),
        )]),
        Line::from(vec![Span::styled(
            "  • Open ports",
            Style::default().fg(Color::Gray),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  ℹ️  ", Style::default().fg(Color::Cyan)),
            Span::styled(
                "Read-only - suggests improvements",
                Style::default().fg(Color::Cyan),
            ),
        ]),
    ]
}

fn configure_network_description() -> Vec<Line<'static>> {
    vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Configure Network",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Configure network interfaces.",
            Style::default().fg(Color::White),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Configuration options:",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![Span::styled(
            "  • DHCP      - Automatic IP",
            Style::default().fg(Color::Gray),
        )]),
        Line::from(vec![Span::styled(
            "  • Static IP - Manual setup",
            Style::default().fg(Color::Gray),
        )]),
        Line::from(vec![Span::styled(
            "  • WiFi      - Wireless connection",
            Style::default().fg(Color::Gray),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Supported: NetworkManager, systemd-networkd",
            Style::default().fg(Color::Cyan),
        )]),
    ]
}

fn test_connectivity_description() -> Vec<Line<'static>> {
    vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Test Connectivity",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Test network connectivity.",
            Style::default().fg(Color::White),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Tests performed:",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![Span::styled(
            "  • Ping gateway",
            Style::default().fg(Color::Gray),
        )]),
        Line::from(vec![Span::styled(
            "  • Ping DNS server",
            Style::default().fg(Color::Gray),
        )]),
        Line::from(vec![Span::styled(
            "  • DNS resolution",
            Style::default().fg(Color::Gray),
        )]),
        Line::from(vec![Span::styled(
            "  • Internet access",
            Style::default().fg(Color::Gray),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  ℹ️  ", Style::default().fg(Color::Cyan)),
            Span::styled(
                "Helps identify network problems",
                Style::default().fg(Color::Cyan),
            ),
        ]),
    ]
}

fn firewall_rules_description() -> Vec<Line<'static>> {
    vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Firewall Rules",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Configure firewall rules.",
            Style::default().fg(Color::White),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Supported firewalls:",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![Span::styled(
            "  • ufw       - Uncomplicated Firewall",
            Style::default().fg(Color::Gray),
        )]),
        Line::from(vec![Span::styled(
            "  • firewalld - Zone-based firewall",
            Style::default().fg(Color::Gray),
        )]),
        Line::from(vec![Span::styled(
            "  • nftables  - Modern replacement",
            Style::default().fg(Color::Gray),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  ⚠️  ", Style::default().fg(Color::Yellow)),
            Span::styled(
                "Incorrect rules may lock you out!",
                Style::default().fg(Color::Yellow),
            ),
        ]),
    ]
}

fn network_info_description() -> Vec<Line<'static>> {
    vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Network Information",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Display current network configuration.",
            Style::default().fg(Color::White),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Information shown:",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![Span::styled(
            "  • Interface names and states",
            Style::default().fg(Color::Gray),
        )]),
        Line::from(vec![Span::styled(
            "  • IP addresses (IPv4/IPv6)",
            Style::default().fg(Color::Gray),
        )]),
        Line::from(vec![Span::styled(
            "  • MAC addresses",
            Style::default().fg(Color::Gray),
        )]),
        Line::from(vec![Span::styled(
            "  • Default gateway and DNS",
            Style::default().fg(Color::Gray),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  ℹ️  ", Style::default().fg(Color::Cyan)),
            Span::styled(
                "Read-only - no changes made",
                Style::default().fg(Color::Cyan),
            ),
        ]),
    ]
}

fn back_to_menu_description(menu_name: &str) -> Vec<Line<'static>> {
    vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            format!("  Return to {}", menu_name),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Go back to the previous menu.",
            Style::default().fg(Color::Gray),
        )]),
    ]
}
