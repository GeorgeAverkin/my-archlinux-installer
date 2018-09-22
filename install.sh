#!/usr/bin/zsh
set -e
BASE_PATH="$(dirname $(readlink -f $0))"
source "$BASE_PATH/config.sh"
source "$BASE_PATH/utils.sh"

grub_install() {
  if [ -d /sys/firmware/efi ]; then
    grub-install \
      --target=x86_64-efi \
      --efi-directory=/mnt/efi \
      --bootloader-id=$BOOTLOADER_ID \
      --boot-directory=/mnt/boot
  else
    grub-install --target=i386-pc --boot-directory=/mnt/boot /dev/"$DRIVE"
  fi
}

clean_mountpoint() {
  echo "WARNING!!! Disk $DRIVE will be wiped. Type \"DO IT\" to continue "
  read

  if [[ ! $REPLY = "DO IT" ]]; then
    exit 1
  fi

  find /mnt -maxdepth 1 \
    -not -wholename /mnt \
    -not -wholename /mnt/boot \
    -not -wholename /mnt/efi \
    -not -wholename /mnt/home \
    -print0 | xargs -0r rm -rv --

  rm -rv /mnt/home/* /mnt/boot/*
}

partition_disk() {
  if [[ $(findmnt -M /mnt) ]]; then
    # echo "Device is mounted, skip partitioning"
    # clean_mountpoint
    # return
    umount -Rv /mnt
  fi

  if [ -e "/dev/mapper/$CRYPT_MAPPING" ]; then
    # echo "Cryptdevice is active, skip partitioning"
    # return
    cryptsetup close $CRYPT_MAPPING
  fi

  fdisk -l /dev/"$DRIVE"
  echo "WARNING!!! Disk $DRIVE will be wiped. Type \"DO IT\" to continue "
  read

  if [[ ! $REPLY = "DO IT" ]]; then
    exit 1
  fi

  sgdisk --zap-all /dev/"$DRIVE"
  wipefs -a /dev/"$DRIVE"
  gdisk_instruction | gdisk /dev/"$DRIVE"
  wipefs -a /dev/"$EFI"
  wipefs -a /dev/"$BOOT"
  wipefs -a /dev/"$ROOT"
}

encrypt_disk() {
  local boot=$1
  local root=$2
  sgdisk --zap-all /dev/"$BOOT"
  wipefs -a /dev/"$BOOT"
  sgdisk --zap-all /dev/"$ROOT"
  wipefs -a /dev/"$ROOT"

  local password
  if [[ $DISK_PASSWORD ]]; then
    password=$DISK_PASSWORD
  else
    askpass password "Enter a password for the disk decryption"
  fi
  
  echo "Encrypting the disk..."
  echo -e "$password\n" | cryptsetup -v luksFormat /dev/"$ROOT"
  echo -e "$password\n" | cryptsetup open /dev/"$ROOT" $CRYPT_MAPPING
  
  if [ -e /dev/mapper/$CRYPT_MAPPING ]; then
    pvremove --force --force /dev/mapper/$CRYPT_MAPPING
  fi
  pvcreate /dev/mapper/$CRYPT_MAPPING
  vgcreate $VG_NAME /dev/mapper/$CRYPT_MAPPING
  lvcreate -L $LVM_ROOT_SIZE $VG_NAME -n root
  lvcreate -L $LVM_HOME_SIZE $VG_NAME -n home
}

format_disk() {
  if [[ $ENCRYPT_DISK = true ]]; then
    encrypt_disk
  fi
  mkfs."$FS" -O \^64bit /dev/"$BOOT"

  if [[ $USE_LVM = true ]]; then
    mkfs."$FS" /dev/$VG_NAME/root
    mkfs."$FS" /dev/$VG_NAME/home
  else
    mkfs."$FS" /dev/$ROOT
  fi

  if [ -d /sys/firmware/efi ]; then
    mkfs.fat -F32 /dev/$EFI
  fi
}

mount_fs() {
  if [[ $USE_LVM = true ]]; then
    mount /dev/$VG_NAME/root /mnt
    mkdir /mnt/home
    mount /dev/$VG_NAME/home /mnt/home
  else
    mount /dev/$ROOT /mnt
  fi

  mkdir /mnt/boot
  mount /dev/"$BOOT" /mnt/boot

  if [ -d /sys/firmware/efi ]; then
    mkdir -v /mnt/efi
    mount /dev/$EFI /mnt/efi
  fi
}

gdisk_instruction() {
  local do_gpt="o\ny\n"
  local do_bios="n\n\n\n+1M\nEF02\n"
  local do_efi="n\n\n\n+64M\nEF00\n"
  local do_boot="n\n\n\n+256M\n\n"
  local do_root="n\n\n\n\n\n"
  local do_write="w\ny\n"

  if [ -d /sys/firmware/efi ]; then
    echo -e $do_gpt$do_efi$do_boot$do_root$do_write
  else
    echo -e $do_gpt$do_bios$do_boot$do_root$do_write
  fi
}

configure_mirrors() {
  mv -v /etc/pacman.d/mirrorlist /etc/pacman.d/mirrorlist.backup
  local mirror_list="https://www.archlinux.org/mirrorlist/?protocol=$MIRROR_PROTOCOL&use_mirror_status=on"
  persist curl -s "$mirror_list" >> /etc/pacman.d/mirrorlist
  sed -i "s/#Server/Server/" /etc/pacman.d/mirrorlist

  if [[ $RANK_MIRRORS = true ]]; then
    if [[ ! $(command -v pacman-contrib) ]]; then
      persist pacman -Sy
      persist pacman -S pacman-contrib
    fi
    sed -i "/^#/d" /etc/pacman.d/mirrorlist
    cat /etc/pacman.d/mirrorlist | rankmirrors -n 0 - >> /etc/pacman.d/mirrorlist
  fi
}

bootstrap() {
  if [[ $MULTILIB = true ]]; then
    cat /etc/pacman.conf \
      | tr '\n' '\r' \
      | sed 's:#\[multilib\]\r#:\[\multilib]\r:' \
      | tr '\r' '\n' \
      > /etc/pacman.conf
  fi

  persist pacstrap /mnt $PACKAGES
  genfstab /mnt >> /mnt/etc/fstab
}

configure_chroot() {
  # clean the directory if previous installation failed
  if [ -d /mnt/root/installer ]; then
    rm -rv /mnt/root/installer
  fi
  cp -rv "$BASE_PATH" /mnt/root
  arch-chroot /mnt zsh /root/installer/chroot_install.sh
  rm -rv /mnt/root/installer
}

main() {
  partition_disk
  format_disk
  mount_fs
  grub_install
  configure_mirrors
  bootstrap
  configure_chroot
  echo -e "\nSuccessfully installed\n"
}

main
