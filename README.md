# 🐧 ArchInstall TUI

> **The professional Arch Linux installer that senior developers actually want to use**

A complete ground-up rewrite of the Arch Linux installer with a focus on **clean code**, **modular architecture**, and **production-quality standards**. This isn't just another installer - it's a **professional-grade tool** built with Rust and Bash that follows Arch Wiki best practices.

## 🚀 **Why This Installer?**

### **Built for Professionals**
- ✅ **Clean, maintainable codebase** that any senior dev would sign off on
- ✅ **Modular architecture** with clear separation of concerns
- ✅ **Comprehensive error handling** and validation
- ✅ **Arch Wiki compliant** partitioning and configuration
- ✅ **Production-ready** with proper testing and CI/CD

### **Zero-Friction Experience**
- 🔥 **Pre-compiled binary** - no build tools needed on live ISO
- 🔥 **Instant execution** - just `git clone` and run
- 🔥 **Real-time progress** with live installation feedback
- 🔥 **Smart validation** prevents common configuration mistakes

## 📋 **Quick Start**

```bash
# Clone and run - it's that simple
git clone https://github.com/your-username/archinstall.git
cd archinstall
sudo ./archinstall-tui
```

**That's it!** No dependencies, no compilation, no hassle.

## 🎯 **Key Features**

### **🏗️ Modern Architecture**
- **Rust TUI Frontend**: Clean, responsive interface using `ratatui`
- **Bash Backend**: Robust installation engine following Arch Wiki
- **Modular Design**: Each component has a single responsibility
- **Type Safety**: Rust ensures compile-time error prevention

### **🔧 Comprehensive Configuration**
- **40+ Installation Options** covering every aspect of system setup
- **Smart Validation**: Prevents incompatible configurations
- **Contextual Warnings**: Proactive guidance for complex setups
- **Dynamic Dependencies**: Options automatically adjust based on selections

### **💾 Advanced Partitioning**
- **ESP + XBOOTLDR**: Arch Wiki recommended dual-boot friendly setup
- **Multiple Filesystems**: ext4, xfs, btrfs support
- **LVM Support**: Logical volume management for complex setups
- **Encryption**: Full LUKS encryption support
- **UUID Management**: Robust device identification

### **📦 Package Management**
- **Interactive Package Selection**: Terminal-like interface for Pacman packages
- **AUR Integration**: Search and install AUR packages via API
- **Dependency Resolution**: Automatic package dependency checking
- **Clean Installation**: Proper package management following Arch standards

### **🖥️ Desktop Environment Support**
- **Multiple DEs**: GNOME, KDE, Hyprland, i3, XFCE
- **Auto-configured**: Display managers automatically set based on DE
- **Theme Support**: Plymouth themes and GRUB customization
- **Clean Integration**: Proper systemd service management

## 🛠️ **Technical Excellence**

### **Code Quality**
```rust
// Example: Clean, documented Rust code
impl Configuration {
    /// Create a new configuration with validation
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Export to environment variables for Bash backend
    pub fn to_env_vars(&self) -> HashMap<String, String> {
        // Robust mapping with error handling
    }
}
```

### **Bash Best Practices**
```bash
# Example: Proper error handling and logging
set -euo pipefail

format_filesystem() {
    local device="$1"
    local fs_type="$2"
    
    case "$fs_type" in
        "ext4")
            check_package_available "e2fsprogs" "mkfs.ext4" || return 1
            mkfs.ext4 -F "$device"
            ;;
        # ... other filesystems with proper dependency checking
    esac
}
```

### **Testing & Quality Assurance**
- **Unit Tests**: Comprehensive Rust test suite
- **Integration Tests**: End-to-end functionality validation
- **CI/CD Pipeline**: Automated testing and building
- **Error Handling**: Graceful failure recovery

## 📁 **Project Structure**

```
archinstall/
├── archinstall-tui          # Pre-compiled binary (ready to run!)
├── src/                     # Rust TUI source code
│   ├── main.rs             # Application entry point
│   ├── app.rs              # Main application logic
│   ├── config.rs           # Configuration management
│   ├── input.rs            # User input handling
│   ├── ui.rs               # TUI rendering
│   └── package_utils.rs    # Package search utilities
├── scripts/                 # Bash installation backend
│   ├── install.sh          # Main installation script
│   ├── utils.sh            # Utility functions
│   └── disk_strategies.sh  # Partitioning strategies
├── Source/                  # Plymouth themes
└── tests/                   # Comprehensive test suite
```

## 🎮 **Usage Examples**

### **Basic Installation**
```bash
sudo ./archinstall-tui
# Navigate with arrow keys
# Press Enter to configure options
# Press Space to start installation
```

### **Advanced Configuration**
- **UEFI + Secure Boot**: Automatic validation and warnings
- **Encrypted Installation**: Full LUKS setup with proper key management
- **Custom Partitioning**: Manual partition layout with validation
- **Package Selection**: Interactive terminal-like package management

## 🔒 **Security & Validation**

### **Proactive Warnings**
- **Secure Boot**: Warns about UEFI requirements
- **Encryption**: Validates LUKS configuration
- **Boot Mode**: Checks system compatibility
- **Dependencies**: Ensures required packages are available

### **Arch Wiki Compliance**
- **ESP Mounting**: Uses `/efi` + XBOOTLDR as recommended
- **UUID Usage**: Robust device identification
- **Package Management**: Proper pacman integration
- **Systemd Integration**: Clean service management

## 🚦 **Development**

### **For Contributors**
```bash
# Development branch has all the tools
git checkout dev
make dev-setup    # Install development dependencies
make test         # Run test suite
make build        # Build release binary
```

### **For Testing**
```bash
# Test branch has comprehensive test suite
git checkout test
cargo test        # Run Rust tests
make docker-test  # Test in containerized environment
```

## 📊 **What Makes This Different**

| Feature | This Installer | Others |
|---------|---------------|---------|
| **Code Quality** | Production-ready Rust + Bash | Often Python scripts |
| **Architecture** | Modular, maintainable | Monolithic |
| **Testing** | Comprehensive test suite | Minimal testing |
| **Dependencies** | Zero (pre-compiled) | Requires build tools |
| **Validation** | Proactive warnings | Basic validation |
| **Partitioning** | Arch Wiki compliant | Often outdated methods |
| **Error Handling** | Robust recovery | Basic error messages |

## 🤝 **Contributing**

This project follows professional development practices:

1. **Main Branch**: Clean, production-ready code
2. **Dev Branch**: Development infrastructure and tools
3. **Test Branch**: Comprehensive testing framework
4. **Pull Requests**: Required for all changes
5. **Code Review**: All code must meet quality standards

## 📄 **License**

MIT License - Feel free to use, modify, and distribute.

## 🙏 **Acknowledgments**

- Built following [Arch Linux Installation Guide](https://wiki.archlinux.org/title/Installation_guide)
- Inspired by the need for a professional, maintainable installer
- Thanks to the Arch Linux community for excellent documentation

---

**Ready to install Arch Linux the professional way?** 

```bash
git clone https://github.com/your-username/archinstall.git
cd archinstall
sudo ./archinstall-tui
```

*No build tools. No dependencies. No excuses.* 🚀