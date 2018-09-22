#!/usr/bin/zsh

source /root/installer/config.sh
cryptsetup open /dev/"$ROOT" cryptroot
mount /dev/mapper/cryptroot /mnt
mount /dev/"$BOOT" /mnt/boot