#!/usr/bin/zsh

# Disk and bootloader
ENCRYPT_DISK=true
BOOTLOADER_ID="archlinux"
CRYPT_MAPPING="cryptroot"
DISK_PASSWORD=""
DRIVE=""
FS="ext4"

# System
ARCH_HOST=""
ARCH_USERNAME=""
ARCH_USERPASS=""
TIMEZONE=""
MIRROR_PROTOCOL="https"
RANK_MIRRORS=false
AUR_HELPER="yay"
MULTILIB=true

# Shadowsocks
SS_SERVER=""
SS_PASSWORD=""

# LVM
USE_LVM=true
LVM_ROOT_SIZE=""
LVM_HOME_SIZE=""
VG_NAME="vgDefault"

# LiveCD
ARCHISO_WORKING_DIR=""
ARCHISO_PROFILE="releng"

# Do not change these values
EFI="${DRIVE}1"
BOOT="${DRIVE}2"
ROOT="${DRIVE}3"

PACKAGES=(
  # minimal installation
  base
  linux
  linux-firmware
  grub
  dhcpcd
  zsh
  sudo
  git

  # optional packages
  anki
  archiso
  code
  eog                               # image viewer
  ffmpegthumbnailer
  file-roller
  firefox
  gdm
  gimp
  gnome-control-center
  gnome-software-packagekit-plugin  # pacman integration
  gnome-terminal
  gnome-tweaks
  gvfs-google
  htop
  libreoffice-fresh
  man
  nautilus
  neofetch
  neovim
  networkmanager                    # gui for gnome network
  openssh                           # ssh command
  rsync                             # copy with exclusion
  shadowsocks-libev
  sshfs
  transmission-gtk
  virt-manager
  vlc
  xdg-user-dirs-gtk                 # Additional directories for users
)

PACKAGES_AUR=(
  skypeforlinux-stable-bin
  telegram-desktop-bin
)

PACKAGES_ARCHISO=(
  gdm
  gnome-control-center
  gnome-terminal
  gnome-backgrounds                 # wallpapers
  networkmanager                    # gui for gnome
  gedit
  code
  eog                               # image viewer
  firefox
  ffmpegthumbnailer
  git
  pacman-contrib                    # rankmirrors
)

VSCODE_EXTENSIONS=(
  # extensions
  humao.rest-client                 # do requests in .http files
  oderwat.indent-rainbow            # customization
  coenraads.bracket-pair-colorizer  # customization
  eamodio.gitlens                   # git
  ms-python.python                  # python
  bungcip.better-toml               # toml
  rust-lang.rust                    # rust
  serayuzgur.crates                 # rust
  alexcvzz.vscode-sqlite            # sqlite
  coolbear.systemd-unit-file        # systemd
  esbenp.prettier-vscode            # formatter

  # themes
  # kesmarag.vscode-precision-theme
  robbowen.synthwave-vscode
)
