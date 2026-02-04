//! Desktop profile management (Sprint 12).
//!
//! This module provides type-safe profile selection for desktop environments
//! and window managers. Package lists are maintained in Rust enums for easy
//! updates and compile-time verification.

#![allow(dead_code)]
//!
//! # Supported Profiles
//!
//! | Profile   | Description | Display Manager |
//! |-----------|-------------|-----------------|
//! | Minimal   | No GUI, base system only | None |
//! | Gnome     | GNOME desktop environment | GDM |
//! | Kde       | KDE Plasma desktop | SDDM |
//! | Hyprland  | Hyprland Wayland compositor | SDDM |
//!
//! # Package List Philosophy
//!
//! Package lists are kept in Rust (not Bash) because:
//! 1. **Compile-time checks**: Typos in package names cause test failures
//! 2. **Easy updates**: Add/remove packages in one place
//! 3. **Testability**: Can verify package lists without running installer
//! 4. **ALPM integration**: Lists fed directly to PackageManager

use strum::{Display, EnumIter, EnumString};

/// Desktop/WM profile selection.
///
/// Each profile includes a curated set of packages for that environment.
/// The package lists are minimal but functional - users can add more via
/// the `extra_packages` configuration option.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, EnumIter, EnumString, Display)]
#[strum(serialize_all = "lowercase")]
pub enum Profile {
    /// Minimal installation - no GUI, just base system.
    /// Good for servers or advanced users who build their own setup.
    Minimal,

    /// GNOME desktop environment.
    /// Modern, polished DE with Wayland support.
    Gnome,

    /// KDE Plasma desktop.
    /// Feature-rich, highly customizable DE.
    Kde,

    /// Hyprland Wayland compositor.
    /// Tiling WM with animations and modern features.
    Hyprland,

    /// Sway Wayland compositor.
    /// i3-compatible tiling WM for Wayland.
    Sway,

    /// i3 window manager (X11).
    /// Lightweight tiling WM.
    I3,

    /// XFCE desktop environment.
    /// Lightweight, traditional desktop.
    Xfce,
}

impl Default for Profile {
    fn default() -> Self {
        Profile::Minimal
    }
}

impl Profile {
    /// Get the packages required for this profile.
    ///
    /// Returns a list of package names to install via ALPM/pacman.
    /// These are the base packages for the profile - users can add
    /// more via configuration.
    ///
    /// # Note
    ///
    /// Package lists include the display manager where applicable.
    /// Services must be enabled separately via `systemctl enable`.
    pub fn get_packages(&self) -> &'static [&'static str] {
        match self {
            Profile::Minimal => &[
                // No GUI packages - just ensure basic tools
                "networkmanager",
                "vim",
                "sudo",
            ],

            Profile::Gnome => &[
                // Core GNOME
                "gnome",
                "gnome-tweaks",
                "gnome-terminal",
                // Display manager
                "gdm",
                // Network
                "networkmanager",
                // Utilities
                "firefox",
                "file-roller",
            ],

            Profile::Kde => &[
                // Core KDE Plasma
                "plasma-meta",
                "plasma-wayland-session",
                "kde-applications-meta",
                "konsole",
                "dolphin",
                // Display manager
                "sddm",
                // Network
                "networkmanager",
                // Utilities
                "firefox",
                "ark",
            ],

            Profile::Hyprland => &[
                // Hyprland compositor
                "hyprland",
                "xdg-desktop-portal-hyprland",
                // Status bar
                "waybar",
                // Terminal
                "kitty",
                // Launcher
                "wofi",
                // Notification daemon
                "mako",
                // Screenshot
                "grim",
                "slurp",
                // Clipboard
                "wl-clipboard",
                // File manager
                "thunar",
                // Display manager
                "sddm",
                // Network
                "networkmanager",
                "network-manager-applet",
                // Audio
                "pipewire",
                "pipewire-pulse",
                "pavucontrol",
                // Fonts
                "ttf-jetbrains-mono-nerd",
                "noto-fonts",
                "noto-fonts-emoji",
                // Utilities
                "firefox",
                "polkit-kde-agent",
            ],

            Profile::Sway => &[
                // Sway compositor
                "sway",
                "swaylock",
                "swayidle",
                "xdg-desktop-portal-wlr",
                // Status bar
                "waybar",
                // Terminal
                "foot",
                // Launcher
                "wofi",
                // Notification
                "mako",
                // Screenshot
                "grim",
                "slurp",
                // Clipboard
                "wl-clipboard",
                // File manager
                "thunar",
                // Display manager
                "sddm",
                // Network
                "networkmanager",
                "network-manager-applet",
                // Audio
                "pipewire",
                "pipewire-pulse",
                "pavucontrol",
                // Fonts
                "ttf-dejavu",
                "noto-fonts",
                // Utilities
                "firefox",
            ],

            Profile::I3 => &[
                // i3 window manager
                "i3-wm",
                "i3status",
                "i3lock",
                // Terminal
                "alacritty",
                // Launcher
                "dmenu",
                "rofi",
                // Compositor (for transparency)
                "picom",
                // File manager
                "thunar",
                // Display manager
                "lightdm",
                "lightdm-gtk-greeter",
                // Network
                "networkmanager",
                "network-manager-applet",
                // Audio
                "pipewire",
                "pipewire-pulse",
                "pavucontrol",
                // X11 essentials
                "xorg-server",
                "xorg-xinit",
                // Fonts
                "ttf-dejavu",
                "noto-fonts",
                // Utilities
                "firefox",
                "feh", // Wallpaper
            ],

            Profile::Xfce => &[
                // XFCE desktop
                "xfce4",
                "xfce4-goodies",
                // Display manager
                "lightdm",
                "lightdm-gtk-greeter",
                // Network
                "networkmanager",
                "network-manager-applet",
                // Audio
                "pipewire",
                "pipewire-pulse",
                "pavucontrol",
                // Utilities
                "firefox",
                "thunar-archive-plugin",
            ],
        }
    }

    /// Get the display manager for this profile.
    ///
    /// Returns the service name to enable with systemctl.
    /// Returns `None` for profiles without a GUI.
    pub fn get_display_manager(&self) -> Option<&'static str> {
        match self {
            Profile::Minimal => None,
            Profile::Gnome => Some("gdm"),
            Profile::Kde => Some("sddm"),
            Profile::Hyprland => Some("sddm"),
            Profile::Sway => Some("sddm"),
            Profile::I3 => Some("lightdm"),
            Profile::Xfce => Some("lightdm"),
        }
    }

    /// Get additional services to enable for this profile.
    ///
    /// Returns service names for systemctl enable.
    pub fn get_services(&self) -> &'static [&'static str] {
        match self {
            Profile::Minimal => &["NetworkManager"],
            Profile::Gnome | Profile::Kde => &["NetworkManager"],
            Profile::Hyprland | Profile::Sway | Profile::I3 | Profile::Xfce => {
                &["NetworkManager"]
            }
        }
    }

    /// Check if this profile uses Wayland.
    pub fn is_wayland(&self) -> bool {
        matches!(self, Profile::Gnome | Profile::Kde | Profile::Hyprland | Profile::Sway)
    }

    /// Check if this profile is a tiling WM.
    pub fn is_tiling(&self) -> bool {
        matches!(self, Profile::Hyprland | Profile::Sway | Profile::I3)
    }

    /// Get a human-readable description of the profile.
    pub fn description(&self) -> &'static str {
        match self {
            Profile::Minimal => "Minimal system without GUI",
            Profile::Gnome => "GNOME desktop environment",
            Profile::Kde => "KDE Plasma desktop",
            Profile::Hyprland => "Hyprland Wayland compositor (tiling)",
            Profile::Sway => "Sway Wayland compositor (i3-compatible)",
            Profile::I3 => "i3 window manager (X11 tiling)",
            Profile::Xfce => "XFCE desktop environment (lightweight)",
        }
    }
}

// ============================================================================
// Package Constants (used by logic::resolver)
// ============================================================================

/// GPU driver packages indexed by driver type.
pub mod gpu_packages {
    /// Nvidia proprietary driver packages.
    pub const NVIDIA: &[&str] = &["nvidia", "nvidia-utils", "nvidia-settings", "lib32-nvidia-utils"];

    /// AMD open-source driver packages (mesa-based).
    pub const AMD: &[&str] = &["mesa", "xf86-video-amdgpu", "vulkan-radeon", "lib32-mesa"];

    /// Intel integrated graphics packages.
    pub const INTEL: &[&str] = &["mesa", "intel-ucode", "vulkan-intel", "lib32-mesa"];

    /// Fallback packages when GPU is auto-detected at runtime.
    pub const AUTO: &[&str] = &["mesa"];
}

/// Kernel packages indexed by kernel variant.
pub mod kernel_packages {
    /// Standard Linux kernel.
    pub const LINUX: &[&str] = &["linux", "linux-headers"];

    /// Long-term support kernel.
    pub const LINUX_LTS: &[&str] = &["linux-lts", "linux-lts-headers"];

    /// Performance-tuned kernel.
    pub const LINUX_ZEN: &[&str] = &["linux-zen", "linux-zen-headers"];

    /// Security-hardened kernel.
    pub const LINUX_HARDENED: &[&str] = &["linux-hardened", "linux-hardened-headers"];
}

/// Base system packages always installed.
pub const BASE_PACKAGES: &[&str] = &[
    "base",
    "base-devel",
    "linux-firmware",
    "networkmanager",
    "vim",
    "sudo",
    "git",
];

/// Bootloader packages.
pub mod bootloader_packages {
    /// GRUB bootloader packages.
    pub const GRUB: &[&str] = &["grub", "efibootmgr", "os-prober"];

    /// systemd-boot (included in systemd, no extra packages needed).
    pub const SYSTEMD_BOOT: &[&str] = &[];
}

/// AUR helper packages (installed from AUR, not official repos).
pub mod aur_packages {
    /// Paru AUR helper.
    pub const PARU: &str = "paru";

    /// Yay AUR helper.
    pub const YAY: &str = "yay";
}

// ============================================================================
// Dotfiles Configuration
// ============================================================================

/// Dotfiles installation configuration.
///
/// Supports cloning dotfiles from a Git repository.
#[derive(Debug, Clone)]
pub struct DotfilesConfig {
    /// Git repository URL (https:// or git://).
    pub repo_url: String,
    /// Target user for dotfiles installation.
    pub target_user: String,
    /// Target directory (default: ~username).
    pub target_dir: Option<String>,
    /// Branch to clone (default: main).
    pub branch: Option<String>,
}

impl DotfilesConfig {
    /// Create a new dotfiles configuration.
    pub fn new(repo_url: &str, target_user: &str) -> Self {
        Self {
            repo_url: repo_url.to_string(),
            target_user: target_user.to_string(),
            target_dir: None,
            branch: None,
        }
    }

    /// Set the target directory.
    pub fn with_target_dir(mut self, dir: &str) -> Self {
        self.target_dir = Some(dir.to_string());
        self
    }

    /// Set the branch to clone.
    pub fn with_branch(mut self, branch: &str) -> Self {
        self.branch = Some(branch.to_string());
        self
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profile_packages_not_empty() {
        use strum::IntoEnumIterator;
        for profile in Profile::iter() {
            let packages = profile.get_packages();
            assert!(
                !packages.is_empty() || profile == Profile::Minimal,
                "{:?} should have packages (or be Minimal)",
                profile
            );
        }
    }

    #[test]
    fn test_profile_display_managers() {
        assert!(Profile::Minimal.get_display_manager().is_none());
        assert_eq!(Profile::Gnome.get_display_manager(), Some("gdm"));
        assert_eq!(Profile::Kde.get_display_manager(), Some("sddm"));
        assert_eq!(Profile::Hyprland.get_display_manager(), Some("sddm"));
    }

    #[test]
    fn test_hyprland_packages() {
        let packages = Profile::Hyprland.get_packages();
        assert!(packages.contains(&"hyprland"));
        assert!(packages.contains(&"waybar"));
        assert!(packages.contains(&"kitty"));
        assert!(packages.contains(&"wofi"));
        assert!(packages.contains(&"sddm")); // Display manager included
    }

    #[test]
    fn test_gnome_packages() {
        let packages = Profile::Gnome.get_packages();
        assert!(packages.contains(&"gnome"));
        assert!(packages.contains(&"gdm")); // Display manager included
    }

    #[test]
    fn test_profile_from_string() {
        assert_eq!("hyprland".parse::<Profile>().unwrap(), Profile::Hyprland);
        assert_eq!("gnome".parse::<Profile>().unwrap(), Profile::Gnome);
        assert_eq!("minimal".parse::<Profile>().unwrap(), Profile::Minimal);
    }

    #[test]
    fn test_profile_wayland() {
        assert!(Profile::Hyprland.is_wayland());
        assert!(Profile::Sway.is_wayland());
        assert!(Profile::Gnome.is_wayland());
        assert!(!Profile::I3.is_wayland());
        assert!(!Profile::Minimal.is_wayland());
    }

    #[test]
    fn test_profile_tiling() {
        assert!(Profile::Hyprland.is_tiling());
        assert!(Profile::Sway.is_tiling());
        assert!(Profile::I3.is_tiling());
        assert!(!Profile::Gnome.is_tiling());
        assert!(!Profile::Kde.is_tiling());
    }

    #[test]
    fn test_dotfiles_config() {
        let config = DotfilesConfig::new("https://github.com/user/dotfiles", "archuser")
            .with_branch("main")
            .with_target_dir("/home/archuser");

        assert_eq!(config.repo_url, "https://github.com/user/dotfiles");
        assert_eq!(config.target_user, "archuser");
        assert_eq!(config.branch, Some("main".to_string()));
        assert_eq!(config.target_dir, Some("/home/archuser".to_string()));
    }
}
