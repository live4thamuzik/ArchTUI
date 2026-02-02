use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// ArchInstall TUI - A friendly Arch Linux installer
#[derive(Parser)]
#[command(name = "archinstall-tui")]
#[command(about = "A user-friendly Arch Linux installer with TUI interface")]
#[command(version)]
pub struct Cli {
    /// Dry-run mode: show what would be executed without making changes.
    ///
    /// In this mode, destructive operations (wipe, format, install) are
    /// skipped and logged. Non-destructive operations (lsblk, system_info)
    /// still execute so the preview is realistic.
    #[arg(long, global = true)]
    pub dry_run: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Run the interactive TUI installer
    Install {
        /// Path to configuration file to use (skips TUI, uses config file)
        #[arg(short, long)]
        config: Option<PathBuf>,

        /// Save current configuration to file and exit (after TUI configuration)
        #[arg(long)]
        save_config: Option<PathBuf>,
    },
    /// Validate a configuration file
    Validate {
        /// Path to configuration file to validate
        config: PathBuf,
    },
    /// Arch Linux Tools - System administration and repair
    Tools {
        #[command(subcommand)]
        tool: ToolCommands,
    },
}

#[derive(Subcommand)]
pub enum ToolCommands {
    /// Disk and filesystem tools
    Disk {
        #[command(subcommand)]
        disk_tool: DiskToolCommands,
    },
    /// System and boot tools
    System {
        #[command(subcommand)]
        system_tool: SystemToolCommands,
    },
    /// User and security tools
    User {
        #[command(subcommand)]
        user_tool: UserToolCommands,
    },
    /// Network tools
    Network {
        #[command(subcommand)]
        network_tool: NetworkToolCommands,
    },
}

#[derive(Subcommand)]
pub enum DiskToolCommands {
    /// Format a partition with specified filesystem
    Format {
        /// Partition device (e.g., /dev/sda1)
        #[arg(short, long)]
        device: String,
        /// Filesystem type
        #[arg(short, long)]
        filesystem: String,
        /// Partition label (optional)
        #[arg(short, long)]
        label: Option<String>,
    },
    /// Securely wipe a disk
    Wipe {
        /// Disk device to wipe (e.g., /dev/sda)
        #[arg(short, long)]
        device: String,
        /// Wipe method (zero, random, secure)
        #[arg(short, long, default_value = "zero")]
        method: String,
        /// Confirm destructive operation
        #[arg(short, long)]
        confirm: bool,
    },
    /// Check disk health using SMART
    Health {
        /// Disk device to check (e.g., /dev/sda)
        #[arg(short, long)]
        device: String,
    },
    /// Mount or unmount partitions
    Mount {
        /// Action to perform
        #[arg(short, long)]
        action: String,
        /// Device to mount/unmount (e.g., /dev/sda1)
        #[arg(short, long)]
        device: String,
        /// Mount point (required for mount action)
        #[arg(short, long)]
        mountpoint: Option<String>,
        /// Filesystem type (optional)
        #[arg(short, long)]
        filesystem: Option<String>,
    },
    /// Manual disk partitioning using cfdisk
    Manual {
        /// Disk device to partition (e.g., /dev/sda)
        #[arg(short, long)]
        device: String,
    },
}

#[derive(Subcommand)]
pub enum SystemToolCommands {
    /// Install or repair bootloader
    Bootloader {
        /// Bootloader type (grub or systemd-boot)
        #[arg(short, long)]
        r#type: String,
        /// Target disk device (e.g., /dev/sda)
        #[arg(short, long)]
        disk: String,
        /// EFI partition path (optional)
        #[arg(short, long)]
        efi_path: Option<String>,
        /// Boot mode (uefi or bios)
        #[arg(short, long, default_value = "uefi")]
        mode: String,
    },
    /// Generate fstab file
    Fstab {
        /// Root partition path (e.g., /mnt)
        #[arg(short, long)]
        root: String,
    },
    /// Chroot into a mounted system
    Chroot {
        /// Root directory to chroot into (default: /mnt)
        #[arg(short, long, default_value = "/mnt")]
        root: String,
        /// Skip mounting /proc, /sys, /dev
        #[arg(long)]
        no_mount: bool,
    },
    /// Display system information
    Info {
        /// Show detailed information
        #[arg(short, long)]
        detailed: bool,
    },
    /// Manage systemd services
    Services {
        /// Action to perform (enable, disable, start, stop, status, list)
        #[arg(short, long)]
        action: String,
        /// Service name
        #[arg(short, long)]
        service: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum UserToolCommands {
    /// Add a new user to the system
    Add {
        /// Username to create
        #[arg(short, long)]
        username: String,
        /// Full name (optional)
        #[arg(short, long)]
        full_name: Option<String>,
        /// Additional groups (comma-separated)
        #[arg(short, long)]
        groups: Option<String>,
        /// Default shell
        #[arg(short, long, default_value = "/bin/bash")]
        shell: String,
    },
    /// Reset user password
    ResetPassword {
        /// Username to reset password for
        #[arg(short, long)]
        username: String,
    },
    /// Manage user groups
    Groups {
        /// Action to perform (add, remove, list, create, delete)
        #[arg(short, long)]
        action: String,
        /// Username
        #[arg(short, long)]
        user: Option<String>,
        /// Group name
        #[arg(short, long)]
        group: Option<String>,
    },
    /// Configure SSH server
    Ssh {
        /// Action to perform (install, configure, enable, disable, status)
        #[arg(short, long)]
        action: String,
        /// SSH port
        #[arg(short, long)]
        port: Option<u16>,
        /// Enable/disable root login
        #[arg(long)]
        root_login: Option<bool>,
        /// Enable/disable password authentication
        #[arg(long)]
        password_auth: Option<bool>,
    },
    /// Perform security audit
    Security {
        /// Action to perform (basic, full)
        #[arg(short, long)]
        action: String,
    },
}

#[derive(Subcommand)]
pub enum NetworkToolCommands {
    /// Configure network settings
    Configure {
        /// Network interface name
        #[arg(short, long)]
        interface: String,
        /// IP address (optional)
        #[arg(short, long)]
        ip: Option<String>,
        /// Gateway (optional)
        #[arg(short, long)]
        gateway: Option<String>,
    },
    /// Test network connectivity
    Test {
        /// Action to perform (ping, dns, http, full)
        #[arg(short, long)]
        action: String,
        /// Host to test (optional)
        #[arg(short = 'H', long)]
        host: Option<String>,
        /// Timeout in seconds
        #[arg(short, long, default_value = "5")]
        timeout: u16,
    },
    /// Configure firewall
    Firewall {
        /// Action to perform (enable, disable, status, rules, install)
        #[arg(short, long)]
        action: String,
        /// Firewall type (iptables, ufw)
        #[arg(short, long, default_value = "iptables")]
        r#type: String,
        /// Port to manage
        #[arg(short, long)]
        port: Option<u16>,
        /// Protocol (tcp, udp)
        #[arg(short, long, default_value = "tcp")]
        protocol: String,
        /// Allow the specified port
        #[arg(long)]
        allow: bool,
        /// Deny the specified port
        #[arg(long)]
        deny: bool,
    },
    /// Network diagnostics
    Diagnostics {
        /// Action to perform (basic, detailed, troubleshoot)
        #[arg(short, long)]
        action: String,
    },
}

impl Cli {
    pub fn parse_args() -> Self {
        <Self as clap::Parser>::parse()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn test_cli_no_args() {
        // Running with no args should succeed (defaults to TUI mode)
        let result = Cli::try_parse_from(["archinstall-tui"]);
        assert!(result.is_ok());
        let cli = result.unwrap();
        assert!(cli.command.is_none());
    }

    #[test]
    fn test_cli_install_with_config() {
        let result = Cli::try_parse_from([
            "archinstall-tui",
            "install",
            "--config",
            "/path/to/config.json",
        ]);
        assert!(result.is_ok());
        let cli = result.unwrap();
        match cli.command {
            Some(Commands::Install { config, .. }) => {
                assert_eq!(config.unwrap().to_str().unwrap(), "/path/to/config.json");
            }
            _ => panic!("Expected Install command"),
        }
    }

    #[test]
    fn test_cli_validate_command() {
        let result = Cli::try_parse_from([
            "archinstall-tui",
            "validate",
            "/path/to/config.json",
        ]);
        assert!(result.is_ok());
        let cli = result.unwrap();
        match cli.command {
            Some(Commands::Validate { config }) => {
                assert_eq!(config.to_str().unwrap(), "/path/to/config.json");
            }
            _ => panic!("Expected Validate command"),
        }
    }

    #[test]
    fn test_cli_disk_format_tool() {
        let result = Cli::try_parse_from([
            "archinstall-tui",
            "tools",
            "disk",
            "format",
            "--device",
            "/dev/sda1",
            "--filesystem",
            "ext4",
        ]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cli_system_bootloader_tool() {
        let result = Cli::try_parse_from([
            "archinstall-tui",
            "tools",
            "system",
            "bootloader",
            "--type",
            "grub",
            "--disk",
            "/dev/sda",
        ]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cli_user_add_tool() {
        let result = Cli::try_parse_from([
            "archinstall-tui",
            "tools",
            "user",
            "add",
            "--username",
            "testuser",
        ]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cli_network_test_tool() {
        let result = Cli::try_parse_from([
            "archinstall-tui",
            "tools",
            "network",
            "test",
            "--action",
            "ping",
        ]);
        assert!(result.is_ok());
    }
}
