use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// ArchInstall TUI - A friendly Arch Linux installer
#[derive(Parser)]
#[command(name = "archinstall-tui")]
#[command(about = "A user-friendly Arch Linux installer with TUI interface")]
#[command(version)]
pub struct Cli {
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
}

impl Cli {
    pub fn parse_args() -> Self {
        <Self as clap::Parser>::parse()
    }
}
