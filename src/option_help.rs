//! Per-option help text shown in selection dialogs.
//!
//! `describe(field, value)` returns a one-line, pacman-style summary for the
//! currently-highlighted choice. Keys here MUST match the strum-serialized
//! values from `src/types.rs` exactly — mismatches mean a silent "no help."

/// Lookup a description for `(field, value)`. Returns `None` for unknown
/// combinations — callers should treat that as "no help available."
pub fn describe(field: &str, value: &str) -> Option<&'static str> {
    match field {
        "Kernel" => kernel(value),
        "Bootloader" => bootloader(value),
        "Desktop Environment" => desktop_environment(value),
        "Display Manager" => display_manager(value),
        "GPU Drivers" => gpu_driver(value),
        "Partitioning Strategy" => partitioning(value),
        "Root Filesystem" | "Home Filesystem" => filesystem(value),
        "Network Manager" => network_manager(value),
        "Editor" => editor(value),
        "AUR Helper" => aur_helper(value),
        "Snapshot Tool" => snapshot_tool(value),
        "DE Variant" => de_variant(value),
        "Encryption" => encryption(value),
        "Boot Mode" => boot_mode(value),
        "Network Tools" | "System Utilities" | "Dev Tools" => opt_in_package(value),
        _ => None,
    }
}

fn kernel(v: &str) -> Option<&'static str> {
    Some(match v {
        "linux" => "Vanilla mainline kernel — recommended default",
        "linux-lts" => "Long-term support — older but more conservative updates",
        "linux-zen" => "Patched for desktop responsiveness and lower latency",
        "linux-hardened" => "Security-focused with KSPP patches and stricter defaults",
        _ => return None,
    })
}

fn bootloader(v: &str) -> Option<&'static str> {
    Some(match v {
        "grub" => "Most flexible — BIOS+UEFI, dual-boot, OS prober, themes",
        "systemd-boot" => "UEFI-only, minimal, fast — best for single-OS UEFI installs",
        "refind" => "UEFI graphical boot menu — auto-detects kernels, themable",
        "limine" => "Modern lightweight loader — UEFI/BIOS, fast, simple config",
        "efistub" => "Boot kernel directly via EFI — no bootloader, manual entries",
        _ => return None,
    })
}

fn desktop_environment(v: &str) -> Option<&'static str> {
    Some(match v {
        "none" => "No desktop — TTY/headless install",
        "gnome" => "Modern, polished, touch-friendly — GNOME Shell workflow",
        "kde" => "KDE Plasma — highly customizable, traditional layout, Qt apps",
        "xfce" => "Lightweight traditional desktop — fast, low resource use",
        "mate" => "Classic GNOME 2 fork — traditional layout, low resource use",
        "lxqt" => "Lightweight Qt desktop — among the fastest major DEs",
        "lxde" => "Predecessor to LXQt (GTK) — minimal, very low resource use",
        "cinnamon" => "Mint's GNOME 3 fork — Windows-like traditional layout",
        "budgie" => "Modern opinionated layout on the GNOME stack",
        "deepin" => "Polished desktop from Deepin — distinctive UI",
        "cosmic" => "System76's Rust-based DE — early/alpha quality",
        "hyprland" => "Wayland tiling compositor — eye-candy, manual config",
        "sway" => "Wayland tiling — i3-compatible, keyboard-driven, manual config",
        "i3" => "X11 tiling — classic i3, manual config required",
        "river" => "Wayland tiling — dynamic layouts via external commands",
        "niri" => "Wayland scrollable tiling — columns scroll horizontally",
        "labwc" => "Wayland stacking compositor — Openbox-like, lightweight",
        "bspwm" => "Tiling WM controlled via shell scripts — scriptable, minimal",
        "awesome" => "Tiling WM configured in Lua — dynamic layouts, extensible",
        "qtile" => "Tiling WM configured in Python — extensible",
        "xmonad" => "Tiling WM configured in Haskell — small, fast, extensible",
        "dwm" => "Suckless tiling WM — config in C, recompile to change",
        _ => return None,
    })
}

fn display_manager(v: &str) -> Option<&'static str> {
    Some(match v {
        "none" => "No graphical login — start the session manually (startx/sway)",
        "gdm" => "GNOME's display manager — Wayland-first, ships with GNOME",
        "sddm" => "Qt-based — ships with KDE Plasma, themable",
        "lightdm" => "Lightweight, X11/Wayland, supports multiple greeter themes",
        "lxdm" => "Minimal display manager — meant for LXDE/LXQt",
        "ly" => "Console TUI display manager — no X11/Wayland required",
        "greetd" => "Minimal greeter daemon — pair with tuigreet/gtkgreet",
        "cosmic-greeter" => "Greeter from the COSMIC desktop — pairs with COSMIC",
        _ => return None,
    })
}

fn gpu_driver(v: &str) -> Option<&'static str> {
    Some(match v {
        "Auto" => "Detect GPU and install the matching driver (recommended)",
        "NVIDIA" => "NVIDIA proprietary — best gaming on Turing/Ampere/Ada",
        "nvidia-open" => "NVIDIA open kernel module — Turing+ GPUs only",
        "nouveau" => "Open-source NVIDIA — limited 3D performance, no power mgmt",
        "AMD" => "Mesa AMDGPU — well-supported open driver",
        "Intel" => "Mesa Intel driver — works out of the box for Intel iGPUs",
        "None" => "No GPU driver — text-mode only, install manually later",
        _ => return None,
    })
}

fn partitioning(v: &str) -> Option<&'static str> {
    Some(match v {
        "auto_simple" => "Single root partition + optional /home and swap (recommended)",
        "auto_simple_luks" => "Simple layout with full-disk LUKS encryption",
        "auto_lvm" => "LVM volumes on disk — flexible resize/snapshot",
        "auto_luks_lvm" => "LVM inside a LUKS container — encrypted + flexible",
        "auto_raid" => "Software RAID via mdadm — requires 2+ disks",
        "auto_raid_luks" => "RAID with LUKS encryption on top",
        "auto_raid_lvm" => "RAID with LVM on top — redundancy + flexibility",
        "auto_raid_lvm_luks" => "RAID + LVM + LUKS — most complex, most flexible",
        "manual" => "Pre-partition with cfdisk/fdisk first; assign mount points here",
        "pre_mounted" => "Skip partitioning — root and /boot already mounted at /mnt",
        _ => return None,
    })
}

fn filesystem(v: &str) -> Option<&'static str> {
    Some(match v {
        "ext4" => "Mature, fast, journaled — Linux default (recommended)",
        "xfs" => "High-performance for large files; cannot shrink",
        "btrfs" => "Copy-on-write — snapshots, subvolumes, compression",
        "f2fs" => "Flash-friendly — tuned for SSD/eMMC",
        _ => return None,
    })
}

fn network_manager(v: &str) -> Option<&'static str> {
    Some(match v {
        "NetworkManager" => "Wi-Fi/Ethernet/VPN, GUI-friendly — recommended for desktops",
        "iwd" => "Modern Wi-Fi daemon by Intel — minimal, no wpa_supplicant",
        "dhcpcd" => "DHCP client only — wired Ethernet, no Wi-Fi management",
        "none" => "No network manager — configure networking manually after install",
        _ => return None,
    })
}

fn editor(v: &str) -> Option<&'static str> {
    Some(match v {
        "nano" => "Beginner-friendly editor — recommended for most users",
        "vim" => "Modal editor — steep learning curve, very capable",
        "neovim" => "Modernized vim fork — Lua config, better defaults",
        "none" => "No editor in pacstrap — install one yourself later",
        _ => return None,
    })
}

fn aur_helper(v: &str) -> Option<&'static str> {
    Some(match v {
        "paru" => "Modern AUR helper in Rust — flexible, popular default",
        "yay" => "Long-standing AUR helper in Go — well-tested",
        "pikaur" => "Minimal Python AUR helper — small dependency footprint",
        "none" => "No AUR helper — use makepkg directly or install one later",
        _ => return None,
    })
}

fn snapshot_tool(v: &str) -> Option<&'static str> {
    Some(match v {
        "snapper" => "Btrfs-only snapshot manager — openSUSE-style",
        "timeshift" => "Btrfs/rsync snapshots — simple UI, broad filesystem support",
        "none" => "No snapshot tool",
        _ => return None,
    })
}

fn de_variant(v: &str) -> Option<&'static str> {
    Some(match v {
        "Full" => "Install the full meta-package + extras (everything)",
        "Minimal" => "Install just the shell, terminal, and file manager",
        _ => return None,
    })
}

fn encryption(v: &str) -> Option<&'static str> {
    Some(match v {
        "Auto" => "Decide based on the chosen partitioning strategy",
        "Yes" => "Force LUKS encryption on the root partition",
        "No" => "No encryption — fastest, simplest",
        _ => return None,
    })
}

fn boot_mode(v: &str) -> Option<&'static str> {
    Some(match v {
        "Auto" => "Detect firmware automatically (recommended)",
        "UEFI" => "Force UEFI boot — requires UEFI firmware and an ESP",
        "BIOS" => "Force legacy BIOS boot — uses MBR/GPT with BIOS boot partition",
        _ => return None,
    })
}

fn opt_in_package(v: &str) -> Option<&'static str> {
    Some(match v {
        // Network Tools
        "openssh" => "SSH daemon + client — remote shell, scp, sftp",
        "wget" => "Non-interactive downloader — HTTP/HTTPS/FTP",
        "curl" => "Versatile CLI HTTP client — used by many scripts",
        // System Utilities
        "htop" => "Interactive process viewer — colorful replacement for top",
        "btop" => "Modern resource monitor — CPU, memory, disks, network",
        "fastfetch" => "Fast system info display — neofetch successor",
        // Dev Tools
        "base-devel" => "Build tools group (gcc, make, autoconf...) — needed for AUR",
        "gcc" => "GNU Compiler Collection — C/C++ compiler",
        "make" => "Build automation — executes Makefiles",
        "gdb" => "GNU Debugger — debug compiled programs",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{
        AurHelper, AutoToggle, Bootloader, DesktopEnvironment, DisplayManager, Filesystem,
        GpuDriver, Kernel, PartitionScheme, SnapshotTool,
    };
    use strum::IntoEnumIterator;

    /// Every variant of every covered enum must have a description.
    /// This is the regression test for the "auto_luks_lvm missing" /
    /// "some greeters missing" class of bug — strum names get out of sync
    /// with the manual match keys here, and you only notice in the running TUI.
    #[test]
    fn every_enum_variant_has_a_description() {
        for k in Kernel::iter() {
            let v = k.to_string();
            assert!(describe("Kernel", &v).is_some(), "Kernel: {}", v);
        }
        for b in Bootloader::iter() {
            let v = b.to_string();
            assert!(describe("Bootloader", &v).is_some(), "Bootloader: {}", v);
        }
        for d in DesktopEnvironment::iter() {
            let v = d.to_string();
            assert!(
                describe("Desktop Environment", &v).is_some(),
                "DE: {}",
                v
            );
        }
        for d in DisplayManager::iter() {
            let v = d.to_string();
            assert!(
                describe("Display Manager", &v).is_some(),
                "DM: {}",
                v
            );
        }
        for g in GpuDriver::iter() {
            let v = g.to_string();
            assert!(describe("GPU Drivers", &v).is_some(), "GPU: {}", v);
        }
        for p in PartitionScheme::iter() {
            let v = p.to_string();
            assert!(
                describe("Partitioning Strategy", &v).is_some(),
                "PartScheme: {}",
                v
            );
        }
        for f in Filesystem::iter().filter(|v| {
            matches!(
                v,
                Filesystem::Ext4 | Filesystem::Xfs | Filesystem::Btrfs | Filesystem::F2fs
            )
        }) {
            let v = f.to_string();
            assert!(describe("Root Filesystem", &v).is_some(), "FS: {}", v);
        }
        for a in AurHelper::iter() {
            let v = a.to_string();
            assert!(describe("AUR Helper", &v).is_some(), "AUR: {}", v);
        }
        for s in SnapshotTool::iter() {
            let v = s.to_string();
            assert!(
                describe("Snapshot Tool", &v).is_some(),
                "SnapTool: {}",
                v
            );
        }
        for t in AutoToggle::iter() {
            let v = t.to_string();
            assert!(describe("Encryption", &v).is_some(), "Encryption: {}", v);
        }
    }

    #[test]
    fn unknown_value_returns_none() {
        assert!(describe("Kernel", "linux-fictional").is_none());
        assert!(describe("Bootloader", "lilo").is_none());
    }

    #[test]
    fn unknown_field_returns_none() {
        assert!(describe("Hostname", "anything").is_none());
    }

    #[test]
    fn opt_in_packages_resolve_across_groups() {
        assert!(describe("Network Tools", "openssh").is_some());
        assert!(describe("System Utilities", "htop").is_some());
        assert!(describe("Dev Tools", "gcc").is_some());
    }
}
