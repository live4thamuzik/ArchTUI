# Dockerfile for archinstall-tui development and testing
FROM archlinux:latest

# Install development dependencies
RUN pacman -Syu --noconfirm \
    base-devel \
    rust \
    cargo \
    git \
    dosfstools \
    exfatprogs \
    e2fsprogs \
    xfsprogs \
    btrfs-progs \
    parted \
    gptfdisk \
    lvm2 \
    mdadm \
    cryptsetup \
    grub \
    efibootmgr \
    networkmanager \
    sudo \
    vim \
    && pacman -Scc --noconfirm

# Create a non-root user for development
RUN useradd -m -G wheel -s /bin/bash developer \
    && echo "developer ALL=(ALL) NOPASSWD: ALL" >> /etc/sudoers

# Set up working directory
WORKDIR /workspace

# Switch to developer user
USER developer

# Set up Rust environment
RUN rustup default stable

# Default command
CMD ["/bin/bash"]
