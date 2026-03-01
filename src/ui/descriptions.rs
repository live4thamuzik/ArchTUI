//! Tool description text generation
//!
//! This module contains functions that generate description text
//! for various tool categories and individual tools.

use crate::theme::{Colors, Styles};
use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};

/// Get description for tools category
pub fn get_tools_category_description(selection: usize) -> Vec<Line<'static>> {
    match selection {
        0 => vec![
            Line::from(""),
            Line::from(vec![Span::styled(
                "  Disk & Filesystem Tools",
                Styles::category(),
            )]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "  Manage disk partitions and filesystems.",
                Styles::text(),
            )]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "  Available tools:",
                Style::default()
                    .fg(Colors::SUCCESS)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![Span::styled(
                "  • Partition Disk    - Create/delete partitions",
                Styles::text_secondary(),
            )]),
            Line::from(vec![Span::styled(
                "  • Format Partition  - Create filesystems",
                Styles::text_secondary(),
            )]),
            Line::from(vec![Span::styled(
                "  • Wipe Disk         - Secure data erasure",
                Styles::text_secondary(),
            )]),
            Line::from(vec![Span::styled(
                "  • Check Health      - SMART diagnostics",
                Styles::text_secondary(),
            )]),
            Line::from(vec![Span::styled(
                "  • Mount/Unmount     - Manage mount points",
                Styles::text_secondary(),
            )]),
            Line::from(vec![Span::styled(
                "  • LUKS Encryption   - Disk encryption",
                Styles::text_secondary(),
            )]),
        ],
        1 => vec![
            Line::from(""),
            Line::from(vec![Span::styled(
                "  System Configuration Tools",
                Styles::category(),
            )]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "  Configure system components and boot settings.",
                Styles::text(),
            )]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "  Available tools:",
                Style::default()
                    .fg(Colors::SUCCESS)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![Span::styled(
                "  • Install Bootloader - GRUB/systemd-boot",
                Styles::text_secondary(),
            )]),
            Line::from(vec![Span::styled(
                "  • Generate fstab     - Auto-mount config",
                Styles::text_secondary(),
            )]),
            Line::from(vec![Span::styled(
                "  • Chroot            - Enter installed system",
                Styles::text_secondary(),
            )]),
            Line::from(vec![Span::styled(
                "  • Manage Services   - systemd services",
                Styles::text_secondary(),
            )]),
            Line::from(vec![Span::styled(
                "  • System Info       - Hardware details",
                Styles::text_secondary(),
            )]),
            Line::from(vec![Span::styled(
                "  • Enable Services   - Batch service enable",
                Styles::text_secondary(),
            )]),
            Line::from(vec![Span::styled(
                "  • Install AUR Helper - paru/yay setup",
                Styles::text_secondary(),
            )]),
            Line::from(vec![Span::styled(
                "  • Rebuild Initramfs  - mkinitcpio -P",
                Styles::text_secondary(),
            )]),
            Line::from(vec![Span::styled(
                "  • View Install Logs  - Browse log files",
                Styles::text_secondary(),
            )]),
        ],
        2 => vec![
            Line::from(""),
            Line::from(vec![Span::styled(
                "  User & Security Tools",
                Styles::category(),
            )]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "  Manage users, groups, and security settings.",
                Styles::text(),
            )]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "  Available tools:",
                Style::default()
                    .fg(Colors::SUCCESS)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![Span::styled(
                "  • Add User        - Create user accounts",
                Styles::text_secondary(),
            )]),
            Line::from(vec![Span::styled(
                "  • Reset Password  - Change passwords",
                Styles::text_secondary(),
            )]),
            Line::from(vec![Span::styled(
                "  • Manage Groups   - Group memberships",
                Styles::text_secondary(),
            )]),
            Line::from(vec![Span::styled(
                "  • Configure SSH   - SSH keys & config",
                Styles::text_secondary(),
            )]),
            Line::from(vec![Span::styled(
                "  • Security Audit  - Check vulnerabilities",
                Styles::text_secondary(),
            )]),
            Line::from(vec![Span::styled(
                "  • Install Dotfiles - Deploy user configs",
                Styles::text_secondary(),
            )]),
            Line::from(vec![Span::styled(
                "  • Run As User     - Execute as non-root",
                Styles::text_secondary(),
            )]),
        ],
        3 => vec![
            Line::from(""),
            Line::from(vec![Span::styled(
                "  Network Configuration Tools",
                Styles::category(),
            )]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "  Configure networking and connectivity.",
                Styles::text(),
            )]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "  Available tools:",
                Style::default()
                    .fg(Colors::SUCCESS)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![Span::styled(
                "  • Configure Network - Interface setup",
                Styles::text_secondary(),
            )]),
            Line::from(vec![Span::styled(
                "  • Test Connectivity - Ping & diagnostics",
                Styles::text_secondary(),
            )]),
            Line::from(vec![Span::styled(
                "  • Firewall Rules   - Security policies",
                Styles::text_secondary(),
            )]),
            Line::from(vec![Span::styled(
                "  • Network Info     - Current settings",
                Styles::text_secondary(),
            )]),
            Line::from(vec![Span::styled(
                "  • Update Mirrors   - Pacman mirrorlist",
                Styles::text_secondary(),
            )]),
        ],
        _ => vec![
            Line::from(""),
            Line::from(vec![Span::styled(
                "  Return to Main Menu",
                Styles::category(),
            )]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "  Go back to the main menu to choose",
                Styles::text_secondary(),
            )]),
            Line::from(vec![Span::styled(
                "  a different installation method.",
                Styles::text_secondary(),
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
        5 => luks_encryption_description(),
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
        5 => enable_services_description(),
        6 => install_aur_helper_description(),
        7 => rebuild_initramfs_description(),
        8 => view_install_logs_description(),
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
        5 => install_dotfiles_description(),
        6 => run_as_user_description(),
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
        4 => update_mirrors_description(),
        _ => back_to_menu_description("Tools Menu"),
    }
}

// Individual tool descriptions

fn partition_disk_description() -> Vec<Line<'static>> {
    vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Partition Disk (GPT/MBR/cfdisk)",
            Styles::category(),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Create partition tables and manage partitions.",
            Styles::text(),
        )]),
        Line::from(vec![Span::styled(
            "  Shows disk layout before any action.",
            Styles::text(),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Actions:",
            Style::default()
                .fg(Colors::SUCCESS)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![Span::styled(
            "  • Create Table  - Initialize GPT or MBR",
            Styles::text_secondary(),
        )]),
        Line::from(vec![Span::styled(
            "  • Add Partition  - Set size, type, label",
            Styles::text_secondary(),
        )]),
        Line::from(vec![Span::styled(
            "  • Delete Partition - Remove by number",
            Styles::text_secondary(),
        )]),
        Line::from(vec![Span::styled(
            "  • cfdisk         - Interactive editor",
            Styles::text_secondary(),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Partition types:",
            Style::default()
                .fg(Colors::SUCCESS)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![Span::styled(
            "  EFI, BIOS Boot, Linux, Swap, LVM, LUKS",
            Styles::text_secondary(),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  ⚠️  ", Styles::error()),
            Span::styled(
                "Warning: Partition changes are permanent",
                Styles::error(),
            ),
        ]),
    ]
}

fn format_partition_description() -> Vec<Line<'static>> {
    vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Format Partition",
            Styles::category(),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Format a partition with a filesystem.",
            Styles::text(),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Supported filesystems:",
            Style::default()
                .fg(Colors::SUCCESS)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![Span::styled(
            "  • ext4    - Standard Linux filesystem (recommended)",
            Styles::text_secondary(),
        )]),
        Line::from(vec![Span::styled(
            "  • btrfs   - Copy-on-write with snapshots",
            Styles::text_secondary(),
        )]),
        Line::from(vec![Span::styled(
            "  • xfs     - High-performance filesystem",
            Styles::text_secondary(),
        )]),
        Line::from(vec![Span::styled(
            "  • fat32   - For EFI system partitions",
            Styles::text_secondary(),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  ⚠️  ", Styles::error()),
            Span::styled(
                "Warning: All data on partition will be erased!",
                Styles::error(),
            ),
        ]),
    ]
}

fn wipe_disk_description() -> Vec<Line<'static>> {
    vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Wipe Disk (Secure Erase)",
            Styles::category(),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Securely erase all data on a disk.",
            Styles::text(),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Methods:",
            Style::default()
                .fg(Colors::SUCCESS)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![Span::styled(
            "  • Zero fill    - Fast, single pass of zeros",
            Styles::text_secondary(),
        )]),
        Line::from(vec![Span::styled(
            "  • Random fill  - More secure, random data",
            Styles::text_secondary(),
        )]),
        Line::from(vec![Span::styled(
            "  • ATA Secure   - Hardware-level secure erase",
            Styles::text_secondary(),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  🚨 ", Styles::error()),
            Span::styled(
                "DANGER: This operation is IRREVERSIBLE!",
                Style::default()
                    .fg(Colors::ERROR)
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
            Styles::category(),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Run SMART diagnostics on a disk drive.",
            Styles::text(),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Information provided:",
            Style::default()
                .fg(Colors::SUCCESS)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![Span::styled(
            "  • Overall health status",
            Styles::text_secondary(),
        )]),
        Line::from(vec![Span::styled(
            "  • Power-on hours",
            Styles::text_secondary(),
        )]),
        Line::from(vec![Span::styled(
            "  • Reallocated sector count",
            Styles::text_secondary(),
        )]),
        Line::from(vec![Span::styled(
            "  • Temperature readings",
            Styles::text_secondary(),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  ℹ️  ", Styles::info()),
            Span::styled(
                "Safe to run - does not modify disk",
                Styles::info(),
            ),
        ]),
    ]
}

fn mount_unmount_description() -> Vec<Line<'static>> {
    vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Mount/Unmount Partitions",
            Styles::category(),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Mount partitions to access their contents.",
            Styles::text(),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Common mount points:",
            Style::default()
                .fg(Colors::SUCCESS)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![Span::styled(
            "  • /mnt        - Temporary mount point",
            Styles::text_secondary(),
        )]),
        Line::from(vec![Span::styled(
            "  • /mnt/boot   - Boot partition",
            Styles::text_secondary(),
        )]),
        Line::from(vec![Span::styled(
            "  • /mnt/home   - Home partition",
            Styles::text_secondary(),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "  Tip:",
                Style::default()
                    .fg(Colors::PRIMARY)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " Mount root (/) first, then others",
                Styles::info(),
            ),
        ]),
    ]
}

fn install_bootloader_description() -> Vec<Line<'static>> {
    vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Install Bootloader",
            Styles::category(),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Install a bootloader to make your system bootable.",
            Styles::text(),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Available bootloaders:",
            Style::default()
                .fg(Colors::SUCCESS)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![Span::styled(
            "  • GRUB         - Traditional, feature-rich",
            Styles::text_secondary(),
        )]),
        Line::from(vec![Span::styled(
            "  • systemd-boot - Simple, fast UEFI boot manager",
            Styles::text_secondary(),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Requirements:",
            Styles::title(),
        )]),
        Line::from(vec![Span::styled(
            "  • Root partition mounted at /mnt",
            Styles::text_secondary(),
        )]),
    ]
}

fn generate_fstab_description() -> Vec<Line<'static>> {
    vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Generate fstab",
            Styles::category(),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Generate /etc/fstab for automatic mounting.",
            Styles::text(),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Identification methods:",
            Style::default()
                .fg(Colors::SUCCESS)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![Span::styled(
            "  • UUID    - Universally unique identifier",
            Styles::text_secondary(),
        )]),
        Line::from(vec![Span::styled(
            "  • LABEL   - Filesystem label",
            Styles::text_secondary(),
        )]),
        Line::from(vec![Span::styled(
            "  • PARTUUID - Partition UUID (GPT only)",
            Styles::text_secondary(),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "  Tip:",
                Style::default()
                    .fg(Colors::PRIMARY)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " Mount all partitions before generating",
                Styles::info(),
            ),
        ]),
    ]
}

fn chroot_description() -> Vec<Line<'static>> {
    vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Chroot into System",
            Styles::category(),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Enter an installed system for maintenance.",
            Styles::text(),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Common uses:",
            Style::default()
                .fg(Colors::SUCCESS)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![Span::styled(
            "  • Fix broken bootloader",
            Styles::text_secondary(),
        )]),
        Line::from(vec![Span::styled(
            "  • Reset forgotten password",
            Styles::text_secondary(),
        )]),
        Line::from(vec![Span::styled(
            "  • Install/remove packages",
            Styles::text_secondary(),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "  Exit:",
                Style::default()
                    .fg(Colors::PRIMARY)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " Type 'exit' or press Ctrl+D",
                Styles::info(),
            ),
        ]),
    ]
}

fn manage_services_description() -> Vec<Line<'static>> {
    vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Manage Services",
            Styles::category(),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Enable or disable systemd services.",
            Styles::text(),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Common services:",
            Style::default()
                .fg(Colors::SUCCESS)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![Span::styled(
            "  • NetworkManager   - Network management",
            Styles::text_secondary(),
        )]),
        Line::from(vec![Span::styled(
            "  • sshd             - SSH server",
            Styles::text_secondary(),
        )]),
        Line::from(vec![Span::styled(
            "  • bluetooth        - Bluetooth support",
            Styles::text_secondary(),
        )]),
        Line::from(vec![Span::styled(
            "  • gdm/sddm         - Display managers",
            Styles::text_secondary(),
        )]),
    ]
}

fn system_info_description() -> Vec<Line<'static>> {
    vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            "  System Information",
            Styles::category(),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Display detailed system information.",
            Styles::text(),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Information shown:",
            Style::default()
                .fg(Colors::SUCCESS)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![Span::styled(
            "  • CPU model and cores",
            Styles::text_secondary(),
        )]),
        Line::from(vec![Span::styled(
            "  • Memory (RAM) size",
            Styles::text_secondary(),
        )]),
        Line::from(vec![Span::styled(
            "  • Disk information",
            Styles::text_secondary(),
        )]),
        Line::from(vec![Span::styled(
            "  • Boot mode (UEFI/BIOS)",
            Styles::text_secondary(),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  ℹ️  ", Styles::info()),
            Span::styled(
                "Read-only - no changes made",
                Styles::info(),
            ),
        ]),
    ]
}

fn add_user_description() -> Vec<Line<'static>> {
    vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Add User",
            Styles::category(),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Create a new user account.",
            Styles::text(),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Options:",
            Style::default()
                .fg(Colors::SUCCESS)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![Span::styled(
            "  • Username        - Login name",
            Styles::text_secondary(),
        )]),
        Line::from(vec![Span::styled(
            "  • Home directory  - User's home folder",
            Styles::text_secondary(),
        )]),
        Line::from(vec![Span::styled(
            "  • Shell           - Default login shell",
            Styles::text_secondary(),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "  Tip:",
                Style::default()
                    .fg(Colors::PRIMARY)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " Add to 'wheel' for sudo access",
                Styles::info(),
            ),
        ]),
    ]
}

fn reset_password_description() -> Vec<Line<'static>> {
    vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Reset Password",
            Styles::category(),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Reset or change a user's password.",
            Styles::text(),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Use cases:",
            Style::default()
                .fg(Colors::SUCCESS)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![Span::styled(
            "  • Forgotten password recovery",
            Styles::text_secondary(),
        )]),
        Line::from(vec![Span::styled(
            "  • Set initial password",
            Styles::text_secondary(),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  ⚠️  ", Styles::warning()),
            Span::styled(
                "Requires root/sudo privileges",
                Styles::warning(),
            ),
        ]),
    ]
}

fn manage_groups_description() -> Vec<Line<'static>> {
    vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Manage Groups",
            Styles::category(),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Add or remove users from system groups.",
            Styles::text(),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Common groups:",
            Style::default()
                .fg(Colors::SUCCESS)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![Span::styled(
            "  • wheel    - Sudo/admin privileges",
            Styles::text_secondary(),
        )]),
        Line::from(vec![Span::styled(
            "  • audio    - Audio device access",
            Styles::text_secondary(),
        )]),
        Line::from(vec![Span::styled(
            "  • video    - Video device access",
            Styles::text_secondary(),
        )]),
        Line::from(vec![Span::styled(
            "  • docker   - Docker management",
            Styles::text_secondary(),
        )]),
    ]
}

fn configure_ssh_description() -> Vec<Line<'static>> {
    vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Configure SSH",
            Styles::category(),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Set up SSH keys and server configuration.",
            Styles::text(),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Features:",
            Style::default()
                .fg(Colors::SUCCESS)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![Span::styled(
            "  • Generate SSH key pairs",
            Styles::text_secondary(),
        )]),
        Line::from(vec![Span::styled(
            "  • Import authorized keys",
            Styles::text_secondary(),
        )]),
        Line::from(vec![Span::styled(
            "  • Configure sshd settings",
            Styles::text_secondary(),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  🔐 ", Styles::success()),
            Span::styled(
                "Key-based auth is more secure",
                Styles::success(),
            ),
        ]),
    ]
}

fn security_audit_description() -> Vec<Line<'static>> {
    vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Security Audit",
            Styles::category(),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Check system security settings.",
            Styles::text(),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Checks performed:",
            Style::default()
                .fg(Colors::SUCCESS)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![Span::styled(
            "  • Password policy",
            Styles::text_secondary(),
        )]),
        Line::from(vec![Span::styled(
            "  • File permissions",
            Styles::text_secondary(),
        )]),
        Line::from(vec![Span::styled(
            "  • Running services",
            Styles::text_secondary(),
        )]),
        Line::from(vec![Span::styled(
            "  • Open ports",
            Styles::text_secondary(),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  ℹ️  ", Styles::info()),
            Span::styled(
                "Read-only - suggests improvements",
                Styles::info(),
            ),
        ]),
    ]
}

fn configure_network_description() -> Vec<Line<'static>> {
    vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Configure Network",
            Styles::category(),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Configure network interfaces.",
            Styles::text(),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Configuration options:",
            Style::default()
                .fg(Colors::SUCCESS)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![Span::styled(
            "  • DHCP      - Automatic IP",
            Styles::text_secondary(),
        )]),
        Line::from(vec![Span::styled(
            "  • Static IP - Manual setup",
            Styles::text_secondary(),
        )]),
        Line::from(vec![Span::styled(
            "  • WiFi      - Wireless connection",
            Styles::text_secondary(),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Supported: NetworkManager, systemd-networkd",
            Styles::info(),
        )]),
    ]
}

fn test_connectivity_description() -> Vec<Line<'static>> {
    vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Test Connectivity",
            Styles::category(),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Test network connectivity.",
            Styles::text(),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Tests performed:",
            Style::default()
                .fg(Colors::SUCCESS)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![Span::styled(
            "  • Ping gateway",
            Styles::text_secondary(),
        )]),
        Line::from(vec![Span::styled(
            "  • Ping DNS server",
            Styles::text_secondary(),
        )]),
        Line::from(vec![Span::styled(
            "  • DNS resolution",
            Styles::text_secondary(),
        )]),
        Line::from(vec![Span::styled(
            "  • Internet access",
            Styles::text_secondary(),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  ℹ️  ", Styles::info()),
            Span::styled(
                "Helps identify network problems",
                Styles::info(),
            ),
        ]),
    ]
}

fn firewall_rules_description() -> Vec<Line<'static>> {
    vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Firewall Rules",
            Styles::category(),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Configure firewall rules.",
            Styles::text(),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Supported firewalls:",
            Style::default()
                .fg(Colors::SUCCESS)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![Span::styled(
            "  • ufw       - Uncomplicated Firewall",
            Styles::text_secondary(),
        )]),
        Line::from(vec![Span::styled(
            "  • firewalld - Zone-based firewall",
            Styles::text_secondary(),
        )]),
        Line::from(vec![Span::styled(
            "  • nftables  - Modern replacement",
            Styles::text_secondary(),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  ⚠️  ", Styles::warning()),
            Span::styled(
                "Incorrect rules may lock you out!",
                Styles::warning(),
            ),
        ]),
    ]
}

fn network_info_description() -> Vec<Line<'static>> {
    vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Network Information",
            Styles::category(),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Display current network configuration.",
            Styles::text(),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Information shown:",
            Style::default()
                .fg(Colors::SUCCESS)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![Span::styled(
            "  • Interface names and states",
            Styles::text_secondary(),
        )]),
        Line::from(vec![Span::styled(
            "  • IP addresses (IPv4/IPv6)",
            Styles::text_secondary(),
        )]),
        Line::from(vec![Span::styled(
            "  • MAC addresses",
            Styles::text_secondary(),
        )]),
        Line::from(vec![Span::styled(
            "  • Default gateway and DNS",
            Styles::text_secondary(),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  ℹ️  ", Styles::info()),
            Span::styled(
                "Read-only - no changes made",
                Styles::info(),
            ),
        ]),
    ]
}

fn luks_encryption_description() -> Vec<Line<'static>> {
    vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            "  LUKS Encryption",
            Styles::category(),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Encrypt partitions with LUKS2 for full-disk encryption.",
            Styles::text(),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Actions:",
            Style::default()
                .fg(Colors::SUCCESS)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![Span::styled(
            "  • Format   - Create new encrypted volume",
            Styles::text_secondary(),
        )]),
        Line::from(vec![Span::styled(
            "  • Open     - Unlock an encrypted volume",
            Styles::text_secondary(),
        )]),
        Line::from(vec![Span::styled(
            "  • Close    - Lock an encrypted volume",
            Styles::text_secondary(),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  🚨 ", Styles::error()),
            Span::styled(
                "Format DESTROYS all data on partition!",
                Style::default()
                    .fg(Colors::ERROR)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
    ]
}

fn enable_services_description() -> Vec<Line<'static>> {
    vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Enable Services",
            Styles::category(),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Enable systemd services in a chroot environment.",
            Styles::text(),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Common services:",
            Style::default()
                .fg(Colors::SUCCESS)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![Span::styled(
            "  • sddm, gdm        - Display managers",
            Styles::text_secondary(),
        )]),
        Line::from(vec![Span::styled(
            "  • NetworkManager    - Network management",
            Styles::text_secondary(),
        )]),
        Line::from(vec![Span::styled(
            "  • bluetooth         - Bluetooth support",
            Styles::text_secondary(),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Enter comma-separated service names.",
            Styles::info(),
        )]),
    ]
}

fn install_aur_helper_description() -> Vec<Line<'static>> {
    vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Install AUR Helper",
            Styles::category(),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Install paru or yay for AUR package access.",
            Styles::text(),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Available helpers:",
            Style::default()
                .fg(Colors::SUCCESS)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![Span::styled(
            "  • paru  - Feature-rich, Rust-based (recommended)",
            Styles::text_secondary(),
        )]),
        Line::from(vec![Span::styled(
            "  • yay   - Go-based, widely used",
            Styles::text_secondary(),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  ⚠️  ", Styles::warning()),
            Span::styled(
                "Requires a non-root user to build packages",
                Styles::warning(),
            ),
        ]),
    ]
}

fn rebuild_initramfs_description() -> Vec<Line<'static>> {
    vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Rebuild Initramfs",
            Styles::category(),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Rebuild the initramfs via mkinitcpio -P inside a chroot.",
            Styles::text(),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Use this to recover from:",
            Style::default()
                .fg(Colors::SUCCESS)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![Span::styled(
            "  • Failed mkinitcpio during installation",
            Styles::text_secondary(),
        )]),
        Line::from(vec![Span::styled(
            "  • Modified /etc/mkinitcpio.conf",
            Styles::text_secondary(),
        )]),
        Line::from(vec![Span::styled(
            "  • Changed HOOKS or MODULES",
            Styles::text_secondary(),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  ⚠️  ", Styles::warning()),
            Span::styled(
                "Requires system mounted at /mnt with bind mounts",
                Styles::warning(),
            ),
        ]),
    ]
}

fn view_install_logs_description() -> Vec<Line<'static>> {
    vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            "  View Install Logs",
            Styles::category(),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  View ArchTUI installation logs for diagnostics.",
            Styles::text(),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Available logs:",
            Style::default()
                .fg(Colors::SUCCESS)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![Span::styled(
            "  • Master log    - /var/log/archtui/install-*-master.log",
            Styles::text_secondary(),
        )]),
        Line::from(vec![Span::styled(
            "  • TUI debug log - /tmp/archtui.log",
            Styles::text_secondary(),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Shows the most recent log by default.",
            Styles::text_secondary(),
        )]),
    ]
}

fn install_dotfiles_description() -> Vec<Line<'static>> {
    vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Install Dotfiles",
            Styles::category(),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Clone a dotfiles repository for a user.",
            Styles::text(),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Features:",
            Style::default()
                .fg(Colors::SUCCESS)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![Span::styled(
            "  • Clone from any Git repository URL",
            Styles::text_secondary(),
        )]),
        Line::from(vec![Span::styled(
            "  • Set correct file ownership",
            Styles::text_secondary(),
        )]),
        Line::from(vec![Span::styled(
            "  • Optional backup of existing files",
            Styles::text_secondary(),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  ⚠️  ", Styles::warning()),
            Span::styled(
                "May overwrite existing configuration files",
                Styles::warning(),
            ),
        ]),
    ]
}

fn run_as_user_description() -> Vec<Line<'static>> {
    vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Run As User",
            Styles::category(),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Execute a command as a non-root user in chroot.",
            Styles::text(),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Use cases:",
            Style::default()
                .fg(Colors::SUCCESS)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![Span::styled(
            "  • Run makepkg for package building",
            Styles::text_secondary(),
        )]),
        Line::from(vec![Span::styled(
            "  • Test user environment setup",
            Styles::text_secondary(),
        )]),
        Line::from(vec![Span::styled(
            "  • Execute user-specific configuration",
            Styles::text_secondary(),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Uses arch-chroot + sudo -u for privilege dropping.",
            Styles::info(),
        )]),
    ]
}

fn update_mirrors_description() -> Vec<Line<'static>> {
    vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Update Mirrors",
            Styles::category(),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Update pacman mirrorlist using reflector.",
            Styles::text(),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Options:",
            Style::default()
                .fg(Colors::SUCCESS)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![Span::styled(
            "  • Filter by country",
            Styles::text_secondary(),
        )]),
        Line::from(vec![Span::styled(
            "  • Sort by speed, age, or score",
            Styles::text_secondary(),
        )]),
        Line::from(vec![Span::styled(
            "  • Limit number of mirrors",
            Styles::text_secondary(),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  ⚠️  ", Styles::warning()),
            Span::styled(
                "Overwrites /etc/pacman.d/mirrorlist",
                Styles::warning(),
            ),
        ]),
    ]
}

fn back_to_menu_description(menu_name: &str) -> Vec<Line<'static>> {
    vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            format!("  Return to {}", menu_name),
            Styles::category(),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Go back to the previous menu.",
            Styles::text_secondary(),
        )]),
    ]
}
