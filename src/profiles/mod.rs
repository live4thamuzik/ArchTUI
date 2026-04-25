//! Desktop profile management.
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
//! | Sway      | Sway Wayland compositor | SDDM |
//! | I3        | i3 window manager (X11) | LightDM |
//! | Xfce      | XFCE desktop | LightDM |
//! | Cinnamon  | Cinnamon desktop | LightDM |
//! | Mate      | MATE desktop | LightDM |
//! | Budgie    | Budgie desktop | LightDM |
//! | Cosmic    | COSMIC desktop | cosmic-greeter |
//! | Deepin    | Deepin desktop | LightDM |
//! | Lxde      | LXDE desktop | LXDM |
//! | Lxqt      | LXQt desktop | SDDM |
//! | Bspwm     | bspwm (X11 tiling) | LightDM |
//! | Awesome   | Awesome WM (X11 tiling) | LightDM |
//! | Qtile     | Qtile WM (X11 tiling) | LightDM |
//! | River     | River (Wayland tiling) | SDDM |
//! | Niri      | Niri (Wayland tiling) | SDDM |
//! | Labwc     | Labwc (Wayland stacking) | SDDM |
//! | Xmonad    | XMonad (X11 tiling) | LightDM |
//! | Dwm       | DWM (X11 tiling, suckless) | LightDM |
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, EnumIter, EnumString, Display)]
#[strum(serialize_all = "lowercase")]
pub enum Profile {
    /// Minimal installation - no GUI, just base system.
    /// Good for servers or advanced users who build their own setup.
    #[default]
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

    /// Cinnamon desktop environment.
    /// Traditional desktop based on GNOME technologies.
    Cinnamon,

    /// MATE desktop environment.
    /// Traditional desktop forked from GNOME 2.
    Mate,

    /// Budgie desktop environment.
    /// Modern, simple desktop by Solus.
    Budgie,

    /// COSMIC desktop environment.
    /// Modern Rust-based DE by System76 (official repos).
    Cosmic,

    /// Deepin desktop environment.
    /// Elegant Chinese-developed desktop.
    Deepin,

    /// LXDE desktop environment.
    /// Extremely lightweight GTK desktop.
    Lxde,

    /// LXQt desktop environment.
    /// Lightweight Qt-based desktop.
    Lxqt,

    /// bspwm tiling window manager (X11).
    /// Binary space partitioning WM.
    Bspwm,

    /// Awesome tiling window manager (X11).
    /// Highly configurable Lua-based WM.
    Awesome,

    /// Qtile tiling window manager (X11).
    /// Python-based tiling WM.
    Qtile,

    /// River Wayland compositor.
    /// Dynamic tiling Wayland compositor.
    River,

    /// Niri Wayland compositor.
    /// Scrollable-tiling Wayland compositor.
    Niri,

    /// Labwc Wayland compositor.
    /// Openbox-like stacking Wayland compositor.
    Labwc,

    /// XMonad tiling window manager (X11).
    /// Haskell-based tiling WM.
    Xmonad,

    /// DWM tiling window manager (X11).
    /// Suckless dynamic window manager.
    Dwm,
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
            // Minimal profile has no extras of its own. The base system (BASE_PACKAGES) plus the
            // user's chosen network manager and editor already cover the TTY install needs.
            Profile::Minimal => &[],

            // GNOME baseline (Minimal): just enough to boot a usable desktop.
            // Full extras (`gnome` + `gnome-extra`) live in get_full_extras().
            // Wiki: https://wiki.archlinux.org/title/GNOME
            Profile::Gnome => &[
                "gnome-shell",
                "gnome-control-center",
                "gnome-terminal",
                "nautilus",
                "gnome-keyring",
                "gdm",
                "pipewire",
                "pipewire-pulse",
                "wireplumber",
                "firefox",
            ],

            // KDE Plasma baseline (Minimal): plasma-desktop + core apps.
            // Full extras (plasma-meta + kde-applications-meta) live in get_full_extras().
            // Wiki: https://wiki.archlinux.org/title/KDE
            Profile::Kde => &[
                "plasma-desktop",
                "konsole",
                "dolphin",
                "kate",
                "kwallet",
                "plasma-nm",
                "plasma-pa",
                "sddm",
                "pipewire",
                "pipewire-pulse",
                "wireplumber",
                "firefox",
            ],

            Profile::Hyprland => &[
                // Hyprland compositor + portals
                "hyprland",
                "xdg-desktop-portal-hyprland",
                "xdg-desktop-portal-gtk",
                // Authentication
                "polkit",
                "hyprpolkitagent",
                // Lock/idle
                "hyprlock",
                "hypridle",
                // Wallpaper
                "hyprpaper",
                // Status bar
                "waybar",
                // Terminal
                "kitty",
                // Launcher
                "rofi",
                // Notification daemon
                "mako",
                // Screenshot
                "grim",
                "slurp",
                // Clipboard
                "wl-clipboard",
                "cliphist",
                // File manager
                "thunar",
                // Brightness
                "brightnessctl",
                // Bluetooth
                "blueman",
                // Display manager
                "sddm",
                // Network
                // Audio
                "pipewire",
                "pipewire-pulse",
                "wireplumber",
                "pavucontrol",
                // Fonts
                "ttf-jetbrains-mono-nerd",
                "noto-fonts",
                "noto-fonts-emoji",
                // Utilities
                "firefox",
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
                "rofi",
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
                // Audio
                "pipewire",
                "pipewire-pulse",
                "wireplumber",
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
                // Notification daemon
                "dunst",
                // Screenshot
                "maim",
                "xdotool",
                // File manager
                "thunar",
                // Display manager
                "lightdm",
                "lightdm-gtk-greeter",
                // Network
                // Audio
                "pipewire",
                "pipewire-pulse",
                "wireplumber",
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

            // XFCE baseline (Minimal): xfce4 group + DM only. Full extras add xfce4-goodies.
            // Wiki: https://wiki.archlinux.org/title/Xfce
            Profile::Xfce => &[
                "xfce4",
                "xorg-server",
                "xorg-xinit",
                "lightdm",
                "lightdm-gtk-greeter",
                "pipewire",
                "pipewire-pulse",
                "wireplumber",
                "pavucontrol",
                "firefox",
            ],

            Profile::Cinnamon => &[
                // Cinnamon desktop
                "cinnamon",
                "nemo-fileroller",
                // Terminal (cinnamon doesn't include one)
                "gnome-terminal",
                // Screenshot
                "gnome-screenshot",
                // X11 essentials
                "xorg-server",
                "xorg-xinit",
                // Display manager
                "lightdm",
                "lightdm-gtk-greeter",
                // Network
                // Audio
                "pipewire",
                "pipewire-pulse",
                "wireplumber",
                "pavucontrol",
                // Utilities
                "firefox",
            ],

            // MATE baseline (Minimal): mate group + DM only. Full extras add mate-extra.
            // Wiki: https://wiki.archlinux.org/title/MATE
            Profile::Mate => &[
                "mate",
                "xorg-server",
                "xorg-xinit",
                "lightdm",
                "lightdm-gtk-greeter",
                "pipewire",
                "pipewire-pulse",
                "wireplumber",
                "pavucontrol",
                "firefox",
            ],

            Profile::Budgie => &[
                // Budgie desktop
                "budgie-desktop",
                "budgie-extras",
                // Terminal (budgie-desktop doesn't include one)
                "gnome-terminal",
                // File manager
                "nautilus",
                // Screenshot
                "gnome-screenshot",
                // X11 essentials
                "xorg-server",
                "xorg-xinit",
                // Display manager
                "lightdm",
                "lightdm-gtk-greeter",
                // Network
                // Audio
                "pipewire",
                "pipewire-pulse",
                "wireplumber",
                "pavucontrol",
                // Utilities
                "firefox",
            ],

            Profile::Cosmic => &[
                // COSMIC desktop (official Arch repos since 2025)
                "cosmic-session",
                // Core COSMIC apps (not pulled by cosmic-session)
                "cosmic-terminal",
                "cosmic-files",
                "cosmic-text-editor",
                "cosmic-store",
                "cosmic-settings",
                "cosmic-screenshot",
                "cosmic-player",
                "cosmic-icon-theme",
                "cosmic-wallpapers",
                "cosmic-app-library",
                "cosmic-initial-setup",
                "xdg-desktop-portal-cosmic",
                // Display manager
                "cosmic-greeter",
                // Network
                // Audio
                "pipewire",
                "pipewire-pulse",
                "wireplumber",
                "pavucontrol",
                // Utilities
                "firefox",
            ],

            Profile::Deepin => &[
                // Deepin desktop
                "deepin",
                "deepin-extra",
                // X11 essentials
                "xorg-server",
                "xorg-xinit",
                // Display manager
                "lightdm",
                "lightdm-gtk-greeter",
                // Network
                // Audio
                "pipewire",
                "pipewire-pulse",
                "wireplumber",
                "pavucontrol",
                // Utilities
                "firefox",
            ],

            Profile::Lxde => &[
                // LXDE desktop
                "lxde",
                // X11 essentials
                "xorg-server",
                "xorg-xinit",
                // Display manager
                "lxdm",
                // Network
                // Audio
                "pipewire",
                "pipewire-pulse",
                "wireplumber",
                "pavucontrol",
                // Utilities
                "firefox",
            ],

            // LXQt baseline (Minimal): lxqt group + DM only. Full extras add featherpad/etc.
            // Wiki: https://wiki.archlinux.org/title/LXQt
            Profile::Lxqt => &[
                "lxqt",
                "breeze-icons",
                "sddm",
                "pipewire",
                "pipewire-pulse",
                "wireplumber",
                "pavucontrol",
                "firefox",
            ],

            Profile::Bspwm => &[
                // bspwm window manager
                "bspwm",
                "sxhkd",
                // X11 essentials
                "xorg-server",
                "xorg-xinit",
                // Terminal
                "alacritty",
                // Launcher
                "dmenu",
                // Compositor
                "picom",
                // Notification daemon
                "dunst",
                // Screenshot
                "maim",
                "xdotool",
                // Wallpaper
                "feh",
                // File manager
                "thunar",
                // Display manager
                "lightdm",
                "lightdm-gtk-greeter",
                // Network
                // Audio
                "pipewire",
                "pipewire-pulse",
                "wireplumber",
                "pavucontrol",
                // Fonts
                "ttf-dejavu",
                "noto-fonts",
                // Utilities
                "firefox",
            ],

            Profile::Awesome => &[
                // Awesome window manager
                "awesome",
                // X11 essentials
                "xorg-server",
                "xorg-xinit",
                // Terminal
                "alacritty",
                // Launcher
                "dmenu",
                // Compositor
                "picom",
                // Screenshot
                "maim",
                "xdotool",
                // File manager
                "thunar",
                // Wallpaper
                "feh",
                // Display manager
                "lightdm",
                "lightdm-gtk-greeter",
                // Network
                // Audio
                "pipewire",
                "pipewire-pulse",
                "wireplumber",
                "pavucontrol",
                // Fonts
                "ttf-dejavu",
                "noto-fonts",
                // Utilities
                "firefox",
            ],

            Profile::Qtile => &[
                // Qtile window manager
                "qtile",
                "python-psutil",
                // X11 essentials
                "xorg-server",
                "xorg-xinit",
                // Terminal
                "alacritty",
                // Launcher
                "dmenu",
                // Compositor
                "picom",
                // Notification daemon
                "dunst",
                // Screenshot
                "maim",
                "xdotool",
                // File manager
                "thunar",
                // Wallpaper
                "feh",
                // Display manager
                "lightdm",
                "lightdm-gtk-greeter",
                // Network
                // Audio
                "pipewire",
                "pipewire-pulse",
                "wireplumber",
                "pavucontrol",
                // Fonts
                "ttf-dejavu",
                "noto-fonts",
                // Utilities
                "firefox",
            ],

            Profile::River => &[
                // River Wayland compositor
                "river",
                "xdg-desktop-portal-wlr",
                // Lock/idle
                "swaylock",
                "swayidle",
                // Status bar
                "waybar",
                // Terminal
                "foot",
                // Launcher
                "rofi",
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
                // Audio
                "pipewire",
                "pipewire-pulse",
                "wireplumber",
                "pavucontrol",
                // Fonts
                "ttf-dejavu",
                "noto-fonts",
                // Utilities
                "firefox",
            ],

            Profile::Niri => &[
                // Niri Wayland compositor
                "niri",
                "xdg-desktop-portal-gnome",
                // Lock/idle
                "swaylock",
                "swayidle",
                // Status bar
                "waybar",
                // Terminal
                "foot",
                // Launcher
                "fuzzel",
                // Notification
                "mako",
                // Screenshot
                "grim",
                "slurp",
                // Clipboard
                "wl-clipboard",
                // File manager
                "nautilus",
                // Display manager
                "sddm",
                // Network
                // Audio
                "pipewire",
                "pipewire-pulse",
                "wireplumber",
                "pavucontrol",
                // Fonts
                "ttf-dejavu",
                "noto-fonts",
                // Utilities
                "firefox",
            ],

            Profile::Labwc => &[
                // Labwc Wayland compositor
                "labwc",
                "xdg-desktop-portal-wlr",
                // Lock/idle
                "swaylock",
                "swayidle",
                // Status bar
                "waybar",
                // Terminal
                "foot",
                // Launcher
                "rofi",
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
                // Audio
                "pipewire",
                "pipewire-pulse",
                "wireplumber",
                "pavucontrol",
                // Fonts
                "ttf-dejavu",
                "noto-fonts",
                // Utilities
                "firefox",
            ],

            Profile::Xmonad => &[
                // XMonad window manager
                "xmonad",
                "xmonad-contrib",
                "xmobar",
                // X11 essentials
                "xorg-server",
                "xorg-xinit",
                // Launcher
                "dmenu",
                // Terminal
                "alacritty",
                // Compositor
                "picom",
                // Notification daemon
                "dunst",
                // Screenshot
                "maim",
                "xdotool",
                // File manager
                "thunar",
                // Wallpaper
                "feh",
                // Display manager
                "lightdm",
                "lightdm-gtk-greeter",
                // Network
                // Audio
                "pipewire",
                "pipewire-pulse",
                "wireplumber",
                "pavucontrol",
                // Fonts
                "ttf-dejavu",
                "noto-fonts",
                // Utilities
                "firefox",
            ],

            Profile::Dwm => &[
                // DWM window manager
                "dwm",
                // X11 essentials
                "xorg-server",
                "xorg-xinit",
                // Terminal (st is suckless default, but alacritty is more usable out of the box)
                "alacritty",
                // Launcher
                "dmenu",
                // Compositor
                "picom",
                // Notification daemon
                "dunst",
                // Screenshot
                "maim",
                "xdotool",
                // File manager
                "thunar",
                // Wallpaper
                "feh",
                // Display manager
                "lightdm",
                "lightdm-gtk-greeter",
                // Network
                // Audio
                "pipewire",
                "pipewire-pulse",
                "wireplumber",
                "pavucontrol",
                // Fonts
                "ttf-dejavu",
                "noto-fonts",
                // Utilities
                "firefox",
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
            Profile::Cosmic => Some("cosmic-greeter"),
            Profile::Gnome => Some("gdm"),
            Profile::Kde
            | Profile::Hyprland
            | Profile::Sway
            | Profile::Lxqt
            | Profile::River
            | Profile::Niri
            | Profile::Labwc => Some("sddm"),
            Profile::I3
            | Profile::Xfce
            | Profile::Cinnamon
            | Profile::Mate
            | Profile::Budgie
            | Profile::Deepin
            | Profile::Bspwm
            | Profile::Awesome
            | Profile::Qtile
            | Profile::Xmonad
            | Profile::Dwm => Some("lightdm"),
            Profile::Lxde => Some("lxdm"),
        }
    }

    /// Get additional services to enable for this profile.
    ///
    /// Returns profile-specific service names for systemctl enable.
    /// Network manager service is determined by the user's `NetworkManager` choice
    /// and resolved separately in `resolve_services`.
    pub fn get_services(&self) -> &'static [&'static str] {
        &[]
    }

    /// Extra packages added on top of baseline when DE Variant = Full.
    ///
    /// Only the 5 meta-group DEs (GNOME / KDE / XFCE / MATE / LXQt) have meaningful
    /// Full vs Minimal distinctions. Other profiles return an empty slice — their
    /// package lists are already curated to a wiki-prescribed working desktop.
    pub fn get_full_extras(&self) -> &'static [&'static str] {
        match self {
            // Wiki: https://wiki.archlinux.org/title/GNOME — `gnome` group + `gnome-extra` for full suite
            Profile::Gnome => &["gnome", "gnome-extra"],
            // Wiki: https://wiki.archlinux.org/title/KDE — plasma-meta + kde-applications-meta for full suite
            Profile::Kde => &["plasma-meta", "kde-applications-meta"],
            // Wiki: https://wiki.archlinux.org/title/Xfce — xfce4-goodies for full suite
            Profile::Xfce => &["xfce4-goodies", "thunar-archive-plugin"],
            // Wiki: https://wiki.archlinux.org/title/MATE — mate-extra for full suite
            Profile::Mate => &["mate-extra"],
            // Wiki: https://wiki.archlinux.org/title/LXQt — additional curated apps for full suite
            Profile::Lxqt => &["featherpad", "pavucontrol-qt"],
            _ => &[],
        }
    }

    /// Whether this profile has a meaningful Full/Minimal distinction.
    /// True only for the 5 meta-group DEs above.
    pub fn has_full_variant(&self) -> bool {
        !self.get_full_extras().is_empty()
    }

    /// Check if this profile uses Wayland.
    pub fn is_wayland(&self) -> bool {
        matches!(
            self,
            Profile::Gnome
                | Profile::Kde
                | Profile::Hyprland
                | Profile::Sway
                | Profile::Cosmic
                | Profile::River
                | Profile::Niri
                | Profile::Labwc
        )
    }

    /// Check if this profile is a tiling WM.
    pub fn is_tiling(&self) -> bool {
        matches!(
            self,
            Profile::Hyprland
                | Profile::Sway
                | Profile::I3
                | Profile::Bspwm
                | Profile::Awesome
                | Profile::Qtile
                | Profile::River
                | Profile::Niri
                | Profile::Xmonad
                | Profile::Dwm
        )
    }

    /// Check if this profile is a traditional desktop environment.
    pub fn is_traditional_de(&self) -> bool {
        matches!(
            self,
            Profile::Gnome
                | Profile::Kde
                | Profile::Xfce
                | Profile::Cinnamon
                | Profile::Mate
                | Profile::Budgie
                | Profile::Deepin
                | Profile::Lxde
                | Profile::Lxqt
        )
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
            Profile::Cinnamon => "Cinnamon desktop environment (traditional)",
            Profile::Mate => "MATE desktop environment (GNOME 2 fork)",
            Profile::Budgie => "Budgie desktop environment (modern, simple)",
            Profile::Cosmic => "COSMIC desktop environment (Rust-based)",
            Profile::Deepin => "Deepin desktop environment (elegant)",
            Profile::Lxde => "LXDE desktop environment (ultra-lightweight)",
            Profile::Lxqt => "LXQt desktop environment (lightweight Qt)",
            Profile::Bspwm => "bspwm tiling window manager (X11)",
            Profile::Awesome => "Awesome window manager (Lua, X11)",
            Profile::Qtile => "Qtile window manager (Python, X11)",
            Profile::River => "River Wayland compositor (dynamic tiling)",
            Profile::Niri => "Niri Wayland compositor (scrollable tiling)",
            Profile::Labwc => "Labwc Wayland compositor (Openbox-like)",
            Profile::Xmonad => "XMonad window manager (Haskell, X11)",
            Profile::Dwm => "DWM window manager (suckless, X11)",
        }
    }
}

// ============================================================================
// Package Constants (used by logic::resolver)
// ============================================================================

/// GPU driver packages indexed by driver type.
pub mod gpu_packages {
    /// Nvidia proprietary driver packages (DKMS for all kernel variants).
    pub const NVIDIA: &[&str] = &[
        "nvidia-dkms",
        "libglvnd",
        "nvidia-utils",
        "opencl-nvidia",
        "nvidia-settings",
        "lib32-libglvnd",
        "lib32-nvidia-utils",
        "lib32-opencl-nvidia",
    ];

    /// Nvidia open-source kernel module packages (DKMS-based).
    pub const NVIDIA_OPEN: &[&str] = &[
        "nvidia-open-dkms",
        "libglvnd",
        "nvidia-utils",
        "opencl-nvidia",
        "nvidia-settings",
        "lib32-libglvnd",
        "lib32-nvidia-utils",
        "lib32-opencl-nvidia",
    ];

    /// AMD open-source driver packages (mesa-based).
    pub const AMD: &[&str] = &["mesa", "xf86-video-amdgpu", "vulkan-radeon", "lib32-mesa"];

    /// Intel integrated graphics packages.
    pub const INTEL: &[&str] = &["mesa", "xf86-video-intel", "vulkan-intel", "lib32-mesa"];

    /// Nouveau open-source Nvidia driver packages.
    pub const NOUVEAU: &[&str] = &["mesa", "xf86-video-nouveau", "lib32-mesa"];

    /// No GPU driver packages.
    pub const NONE: &[&str] = &[];

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
///
/// Wiki-aligned minimum: https://wiki.archlinux.org/title/Installation_guide#Install_essential_packages
/// User choices for editor (`Editor`) and network manager (`NetworkManager`) are added by the
/// installer at pacstrap time and are not present in this constant. base-devel is no longer
/// always installed; it moves into the `Dev Tools` opt-in group.
pub const BASE_PACKAGES: &[&str] = &[
    "base",
    "linux-firmware",
    "sudo",
    "git",        // Culturally unavoidable on Arch (AUR clones, dotfiles, every wiki tutorial)
    "man-db",     // Wiki philosophy: offline man pages > googling
    "man-pages",  // ditto
    "texinfo",    // GNU info pages, wiki-recommended for many tools
    "pciutils",   // lspci — required for GPU auto-detection in chroot
];

/// Bootloader packages.
pub mod bootloader_packages {
    /// GRUB bootloader packages (os-prober added conditionally by resolver).
    pub const GRUB: &[&str] = &["grub", "efibootmgr"];

    /// systemd-boot (included in systemd, no extra packages needed).
    pub const SYSTEMD_BOOT: &[&str] = &[];

    /// rEFInd bootloader packages.
    pub const REFIND: &[&str] = &["refind"];

    /// Limine bootloader packages.
    pub const LIMINE: &[&str] = &["limine"];

    /// EFISTUB (uses efibootmgr to create boot entries).
    pub const EFISTUB: &[&str] = &["efibootmgr"];
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
        assert!(packages.contains(&"xdg-desktop-portal-hyprland"));
        assert!(packages.contains(&"xdg-desktop-portal-gtk"));
        assert!(packages.contains(&"polkit"));
        assert!(packages.contains(&"hyprpolkitagent"));
        assert!(packages.contains(&"hyprpaper"));
        assert!(packages.contains(&"waybar"));
        assert!(packages.contains(&"kitty"));
        assert!(packages.contains(&"rofi"));
        assert!(packages.contains(&"cliphist"));
        assert!(packages.contains(&"brightnessctl"));
        assert!(packages.contains(&"sddm"));
    }

    #[test]
    fn test_gnome_packages_baseline() {
        // Baseline (Minimal) GNOME: shell + control center + DM, no `gnome` group/extras.
        let packages = Profile::Gnome.get_packages();
        assert!(packages.contains(&"gnome-shell"));
        assert!(packages.contains(&"gdm"));
        assert!(!packages.contains(&"gnome"));
        assert!(!packages.contains(&"gnome-extra"));
    }

    #[test]
    fn test_gnome_full_extras() {
        let extras = Profile::Gnome.get_full_extras();
        assert!(extras.contains(&"gnome"));
        assert!(extras.contains(&"gnome-extra"));
    }

    #[test]
    fn test_meta_de_has_full_variant() {
        for p in [Profile::Gnome, Profile::Kde, Profile::Xfce, Profile::Mate, Profile::Lxqt] {
            assert!(p.has_full_variant(), "{:?} should have full variant", p);
        }
        // Hyprland and Minimal should not
        assert!(!Profile::Hyprland.has_full_variant());
        assert!(!Profile::Minimal.has_full_variant());
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
