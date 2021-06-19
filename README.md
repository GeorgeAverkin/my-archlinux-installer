# Arch Linux installer

## Description

The shell script to automate Arch Linux installation.
Requires connection to internet.

## Features

- Disk encryption
- UEFI/boot detection

## Installation

Download and extract the files using git or curl

Example:

`curl https://codeload.github.com/GeorgeAverkin/my-archlinux-installer/zip/master`

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

Install on the encrypted disk  
`ENCRYPT_DISK=false|true`

Multilib support
`MULTILIB=true|false`

The profile for building a LiveCD  
`ARCHISO_PROFILE=releng|baseline`

The path where archiso will be built. By default points to `$BASE_PATH/tmp/live`
`ARCHISO_WORKING_DIR <PATH>`

## Development

### EFI support for virt-manager
Open the XML file (typically stored in the `/etc/libvirt/qemu` directory).
Append the following in the `os` element:
```xml
<loader type='rom'>/usr/share/edk2-ovmf/x64/OVMF.fd</loader>
``` 

## TODO

- Restart failed or stuck commands
- Add DANGER_NOCONFIRM to skip confirmations
- Add configuration validator
- Update README.md
- Fix grub configuration



```xml
  <qemu:commandline>
    <qemu:arg value="-drive"/>
    <qemu:arg value="file=/home/[USER]/Documents/vm pool/archlinux.qcow2,format=raw,if=none,id=NVME1"/>
    <qemu:arg value="-device"/>
    <qemu:arg value="nvme,drive=NVME1,serial=nvme-1"/>
  </qemu:commandline>
```

```xml
    <disk type="file" device="disk">
      <driver name="qemu" type="qcow2"/>
      <source file="/home/[USER]/Documents/vm pool/archlinux.qcow2"/>
      <target dev="sdb" bus="sata"/>
      <address type="drive" controller="0" bus="0" target="0" unit="1"/>
    </disk>
```
