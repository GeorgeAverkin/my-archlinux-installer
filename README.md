# Arch Linux installer

## Description
The shell script to automate Arch Linux installation.
Requires connection to internet.

## Features
- Disk encryption
- LVM support
- UEFI/boot detection


## Installation
Download and extract the files using git or curl

Example:

`curl https://codeload.github.com/V-0-1-D/my-archlinux-installer/zip/master`

Edit `config.sh` to customize the installation (see configuration).

Run `install.sh` to begin the installation.

## Configuration
### Required
The name of the disk, where the system will be installed, without `/dev/`.  
WARNING! This disk will be wiped during the installation.  
`DRIVE=...`

### Optional
Filter mirrors in `/etc/pacman.d/mirrorlist` by protocol usage.  
`MIRROR_PROTOCOL=http|https|all`

Sort mirrors before installation  
`RANK_MIRRORS=false|true`

Install on the LVM partition  
`USE_LVM=false|true`

Install on the encrypted disk  
`ENCRYPT_DISK=false|true`

Multilib support
`MULTILIB=true|false`

The profile for building a LiveCD  
`ARCHISO_PROFILE=releng|baseline`

The path where archiso will be built. By default points to `$BASE_PATH/tmp/live`
`ARCHISO_WORKING_DIR <PATH>`

## TODO
- Restart failed or stuck commands
- Add DANGER_NOCONFIRM to skip confirmations
- Add configuration validator
- Update README.md
- Fix grub configuration