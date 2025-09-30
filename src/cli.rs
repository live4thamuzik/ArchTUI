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
}

impl Cli {
    pub fn parse_args() -> Self {
        <Self as clap::Parser>::parse()
    }
}
