#!/usr/bin/zsh
set -e
BASE_PATH="$(dirname $(readlink -f $0))"
source "$BASE_PATH/config.sh"
source "$BASE_PATH/utils.sh"

user_do() {
  su -c "$*" $ARCH_USERNAME
}

sudo_password_off() {
  sed -i "s/^$ARCH_USERNAME ALL=(ALL) ALL/$ARCH_USERNAME ALL=(ALL) NOPASSWD: ALL/" /etc/sudoers
}

sudo_password_on() {
  sed -i "s/^$ARCH_USERNAME ALL=(ALL) NOPASSWD: ALL/$ARCH_USERNAME ALL=(ALL) ALL/" /etc/sudoers
}

generate_grub_cmdline() {
  if [[ $USE_LVM == true && $ENCRYPT_DISK == true ]]; then
  echo "loglevel=3 quiet cryptdevice=UUID=$uuid\:$CRYPT_MAPPING root=/dev/$VG_NAME/root"
    return
  fi
  if [[ $USE_LVM == true ]]; then
    # TODO
    return
  fi
  if [[ $ENCRYPT_DISK == true ]]; then
    # TODO
    return
  fi
  echo "loglevel=3 quiet"
}

configure_system() {
  sed -i -e "s/#en_US.UTF-8/en_US.UTF-8/" /etc/locale.gen
  locale-gen
  echo "LANG=en_US.UTF-8" >> /etc/locale.conf

  echo "$ARCH_HOST" >> /etc/hostname

  ln -sf /usr/share/zoneinfo/"$TIMEZONE" /etc/localtime
  timedatectl set-ntp true

  uuid=$(lsblk -dno UUID /dev/"$ROOT")

  local cmdline=generate_grub_cmdline
  sed -i -e "s!loglevel=3 quiet!$cmdline!" /etc/default/grub
  grub-mkconfig -o /boot/grub/grub.cfg

  if [[ $USE_LVM = true ]]; then
    persist pacman -S --noconfirm --needed lvm2
    local hook_lvm=lvm2
  fi
  if [[ $ENCRYPT_DISK = true ]]; then
    local hook_encrypt=encrypt
  fi
  hooks_source=(
    base udev autodetect modconf block filesystems keyboard fsck
  )
  hooks_target=(
    base udev autodetect modconf block keyboard $hook_encrypt $hook_lvm filesystems fsck
  )
  sed -i -e "s/HOOKS=(${hooks_source})/HOOKS=(${hooks_target})/" /etc/mkinitcpio.conf
  mkinitcpio -p linux

  useradd -m "$ARCH_USERNAME"

  local password
  if [[ $ARCH_USERPASS ]]; then
    password=$ARCH_USERPASS
  else
    askpass password "Enter a password for the user \"$ARCH_USERNAME\""
  fi

  echo "$ARCH_USERNAME:$password" | chpasswd

  mv -v /root/installer/gitconfig "/home/$ARCH_USERNAME/.gitconfig"
  chown -v $ARCH_USERNAME: "/home/$ARCH_USERNAME/.gitconfig"
}

configure_packages() {
  if [[ " ${PACKAGES[@]} " =~ " sudo " ]]; then
    echo "$ARCH_USERNAME ALL=(ALL) ALL" >> /etc/sudoers
  fi

  if [[ " ${PACKAGES[@]} " =~ " zsh " ]]; then
    chsh --shell=$(which zsh)
    chsh --shell=$(which zsh) $ARCH_USERNAME
    mv -v /root/installer/zshrc "/home/$ARCH_USERNAME/.zshrc"
    chown -v $ARCH_USERNAME: "/home/$ARCH_USERNAME/.zshrc"

    persist git clone https://github.com/ohmyzsh/ohmyzsh.git \
      /home/$ARCH_USERNAME/.oh-my-zsh
    
    persist git clone https://github.com/zsh-users/zsh-syntax-highlighting.git \
      /home/$ARCH_USERNAME/.oh-my-zsh/custom/plugins/zsh-syntax-highlighting
    
    persist git clone https://github.com/zsh-users/zsh-autosuggestions.git \
      /home/$ARCH_USERNAME/.oh-my-zsh/custom/plugins/zsh-autosuggestions
    
    persist git clone https://github.com/bhilburn/powerlevel9k.git \
      /home/$ARCH_USERNAME/.oh-my-zsh/custom/themes/powerlevel9k

    chown -v $ARCH_USERNAME: "/home/$ARCH_USERNAME/.oh-my-zsh"
  fi

  if [[ " ${PACKAGES[@]} " =~ " code " ]]; then
    persist pacman -S --noconfirm --needed ttf-droid ttf-ubuntu-font-family
    mkdir -pv "/root/.config/Code - OSS/User"
    cp -v /root/installer/vscode_root.json "/root/.config/Code - OSS/User/settings.json"
    mkdir -pv "/home/$ARCH_USERNAME/.config/Code - OSS/User"
    mv -v /root/installer/vscode.json "/home/$ARCH_USERNAME/.config/Code - OSS/User/settings.json"
    chown -Rv $ARCH_USERNAME: "/home/$ARCH_USERNAME/.config"
    
    for extension in $VSCODE_EXTENSIONS; do
      persist user_do code --install-extension $extension
    done

    if [[ " ${VSCODE_EXTENSIONS[@]} " =~ " robbowen.synthwave-vscode " ]]; then
      persist code --install-extension RobbOwen.synthwave-vscode
    fi
  fi

  if [[ " ${PACKAGES[@]} " =~ " shadowsocks-libev " ]]; then
    cp -v /root/installer/ss-local.service /etc/systemd/system
    mkdir -v /etc/shadowsocks-libev
    cp -v /root/installer/ss-config.json /etc/shadowsocks-libev/config.json

    sed -i  -e "s#\"server\": \"\"#\"server\": \"$SS_SERVER\"#" \
            -e "s#\"password\": \"\"#\"password\": \"$SS_PASSWORD\"#" \
            /etc/shadowsocks-libev/config.json
    
    systemctl enable ss-local.service
  fi

  if [[ " ${PACKAGES[@]} " =~ " gvfs-google " ]]; then
    persist pacman -S --noconfirm --needed gnome-keyring
  fi

  if [[ " ${PACKAGES[@]} " =~ " virt-manager " ]]; then
    persist pacman -S --noconfirm --needed dnsmasq ebtables qemu-headless
    systemctl enable libvirtd.service
  fi

  if [[ " ${PACKAGES[@]} " =~ " dhcpcd " ]]; then
    systemctl enable dhcpcd.service
  fi

  if [[ " ${PACKAGES[@]} " =~ " gdm " ]]; then
    systemctl enable gdm.service
  fi

  if [[ " ${PACKAGES[@]} " =~ " networkmanager " ]]; then
    systemctl enable NetworkManager.service
  fi
}

install_aur_helper() {
  if [[ ! $AUR_HELPER ]]; then
    return
  fi
  persist pacman -S --noconfirm --needed base-devel
  local working_dir="/tmp/$AUR_HELPER"
  user_do git clone https://aur.archlinux.org/$AUR_HELPER.git $working_dir

  pushd $working_dir
  sudo_password_off
  persist user_do makepkg -si --noconfirm
  sudo_password_on
  popd

  # local pkgbuild_checksum="998564692c672a6ae4f48a9509ef00b6fae77b58  PKGBUILD"
  # if [[ "$(sha1sum PKGBUILD)" != $pkgbuild_checksum ]]; then
  #   echo "\n\n\n\n"
  #   cat PKGBUILD
  #   echo "\n\n\n\n"
  #   echo "
  #   WARNING! checksum of PKGBUILD does not match.
  #   If you want to verify files before the installation.
  #   To do this press ctrl+z and after you finished use \"fg\" command
  #   to return to the installation.

  #   To abort press ctrl+c.
  #   To continue press enter.
  #   "
  # fi
}

install_aur_packages() {
  if [[ ! $AUR_HELPER ]]; then
    return
  fi

  sudo_password_off
  for extension in $PACKAGES_AUR; do
    persist user_do $AUR_HELPER -S --noconfirm $extension
  done
  sudo_password_on
}

main() {
  configure_system
  configure_packages
  install_aur_helper
  install_aur_packages
}

main
