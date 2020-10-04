use crate::{config::Config, errors::ALIResult};

mod private {
    use {
        crate::{
            config::Config,
            errors::{ALIError, ALIResult, ErrorKind},
            utils::{answer, exe_dir, Mounted, Partitions},
        },
        gptman::GPT,
        std::{
            fs::{self, create_dir, File},
            io::{prelude::*, stdin},
            path::{Path, PathBuf},
            process::{Command, Stdio},
        },
    };

    fn partition_table_exists(disk: &PathBuf) -> bool {
        let mut buffer = String::new();

        let mut fdisk = Command::new("fdisk")
            .arg("-l")
            .arg(disk)
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();

        fdisk
            .stdout
            .as_mut()
            .unwrap()
            .read_to_string(&mut buffer)
            .unwrap();

        buffer.contains("Disklabel type: gpt")
    }

    const EFI_GUID: [u8; 16] = [
        40, 115, 42, 193, 31, 248, 210, 17, 186, 75, 0, 160, 201, 62, 201, 59,
    ];

    const LINUX_GUID: [u8; 16] = [
        0xAF, 0x3D, 0xC6, 0x0F, 0x83, 0x84, 0x72, 0x47, 0x8E, 0x79, 0x3D, 0x69, 0xD8, 0x47, 0x7D,
        0xE4,
    ];

    fn add_efi_partition(gpt: &mut GPT, part_index: u32) {
        let partition_size = 1024 * 1024 * 64 / gpt.sector_size;
        let ending_lba = gpt.header.first_usable_lba + partition_size - 1;

        gpt[part_index] = gptman::GPTPartitionEntry {
            partition_type_guid: EFI_GUID,
            unique_partition_guid: *uuid::Uuid::new_v4().as_bytes(),
            starting_lba: gpt.header.first_usable_lba,
            ending_lba,
            attribute_bits: 0,
            partition_name: "EFI system partition".into(),
        };
    }

    fn add_boot_partition(gpt: &mut GPT, part_index: u32) {
        let starting_lba: u64;
        let ending_lba: u64;
        let partition_size = 1024 * 1024 * 256 / gpt.sector_size;

        if part_index == 1 {
            starting_lba = gpt.header.first_usable_lba;
            ending_lba = gpt.header.first_usable_lba + partition_size - 1;
        } else {
            starting_lba = gpt[part_index - 1].ending_lba + 1;
            ending_lba = gpt[part_index - 1].ending_lba + partition_size;
        }

        gpt[part_index] = gptman::GPTPartitionEntry {
            partition_type_guid: LINUX_GUID,
            unique_partition_guid: *uuid::Uuid::new_v4().as_bytes(),
            starting_lba,
            ending_lba,
            attribute_bits: 0,
            partition_name: "Arch Linux boot partition".into(),
        };
    }

    fn add_root_partition(gpt: &mut GPT, part_index: u32) {
        let starting_lba = gpt[part_index - 1].ending_lba + 1;
        let ending_lba = gpt.header.last_usable_lba;

        gpt[part_index] = gptman::GPTPartitionEntry {
            partition_type_guid: LINUX_GUID,
            unique_partition_guid: *uuid::Uuid::new_v4().as_bytes(),
            starting_lba,
            ending_lba,
            attribute_bits: 0,
            partition_name: "Arch Linux root partition".into(),
        };
    }

    pub struct Installer<'a> {
        config: &'a Config,
        efi: bool,
        format_efi: bool,
        fs_root: Option<PathBuf>,
        partitions: Partitions,
    }

    impl Installer<'_> {
        pub fn new(config: &Config) -> Installer {
            let efi = Path::new("/sys/firmware/efi").exists();
            let mut fs_root: Option<PathBuf> = None;

            if config.drive().encryption() {
                fs_root = Some(config.drive().crypt_mapping_path());
            }
            Installer {
                config,
                efi,
                fs_root,
                format_efi: false,
                partitions: Partitions::new(config.drive().device_path()),
            }
        }

        fn luks_format(&mut self, password: &str) {
            let mut cryptsetup = Command::new("cryptsetup")
                .args(&["-v", "luksFormat"])
                .arg(self.partitions.root().unwrap())
                .stdin(Stdio::piped())
                .spawn()
                .unwrap();

            cryptsetup
                .stdin
                .as_mut()
                .unwrap()
                .write_all(password.as_bytes())
                .unwrap();

            let status = cryptsetup.wait().unwrap();
            assert!(status.success());
        }

        fn luks_open(&mut self, password: &str) {
            let mut cryptsetup = Command::new("cryptsetup")
                .stdin(Stdio::piped())
                .arg("open")
                .arg(self.partitions.root().unwrap())
                .arg(self.fs_root.as_ref().unwrap().file_name().unwrap())
                .spawn()
                .unwrap();

            cryptsetup
                .stdin
                .as_mut()
                .unwrap()
                .write_all(password.as_bytes())
                .unwrap();

            let status = cryptsetup.wait().unwrap();
            assert!(status.success());
        }

        fn create_partition_table(&self) {
            println!(
                "WARNING!!! Disk \"{}\" will be wiped. Type \"DO IT\" to continue.",
                self.config.drive().device(),
            );
            let mut answer = String::new();
            stdin().read_line(&mut answer).unwrap();

            if answer != "DO IT\n" {
                panic!();
                // return Err(ALIError::new(ErrorKind::EarlyExit));
            }
            let mut gdisk = Command::new("gdisk")
                .arg(&self.config.drive().device_path())
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .spawn()
                .unwrap();

            gdisk
                .stdin
                .as_mut()
                .unwrap()
                .write_all("o\ny\nw\ny\n".as_bytes())
                .unwrap();

            let status = gdisk.wait().unwrap();
            assert!(status.success());
        }

        pub fn check_network(&mut self) -> ALIResult<&mut Self> {
            let status = Command::new("ping")
                .args(&["-c", "1", "archlinux.org"])
                .stdout(Stdio::null())
                .spawn()
                .unwrap()
                .wait()
                .unwrap();

            if !status.success() {
                return Err(ALIError::generic(
                    "error: network access required".to_owned(),
                ));
            }
            Ok(self)
        }

        pub fn check_drive(&mut self) -> ALIResult<&mut Self> {
            let drive = self.config.drive().device_path();

            if !drive.exists() {
                let error_msg = format!("Device \"{}\" not found", drive.to_str().unwrap());
                return Err(ALIError::generic(error_msg));
            }
            Ok(self)
        }

        pub fn check_mounted(&mut self) -> ALIResult<&mut Self> {
            let mounted = Mounted::new();

            if mounted.find_by_mountpoint("/mnt").is_some() {
                if answer("\"/mnt\" is already mounted, unmount?") {
                    let status = Command::new("umount")
                        .args(&["-Rv", "/mnt"])
                        .spawn()
                        .unwrap()
                        .wait()
                        .unwrap();

                    assert!(status.success());
                } else {
                    return Err(ALIError::new(ErrorKind::EarlyExit));
                }
            }
            Ok(self)
        }

        pub fn check_luks_open(&mut self) -> ALIResult<&mut Self> {
            let mut crypt_mapping_path = PathBuf::from("/dev/mapper");
            let crypt_mapping = self.config.drive().crypt_mapping();
            crypt_mapping_path.push(crypt_mapping);

            if crypt_mapping_path.exists() {
                let question = format!("Crypt mapper \"{}\" is in use, close?", crypt_mapping);

                if answer(question) {
                    let status = Command::new("cryptsetup")
                        .arg("close")
                        .arg(crypt_mapping)
                        .spawn()
                        .unwrap()
                        .wait()
                        .unwrap();

                    assert!(status.success());
                } else {
                    return Err(ALIError::new(ErrorKind::EarlyExit));
                }
            }
            Ok(self)
        }

        pub fn partition(&mut self) -> ALIResult<&mut Self> {
            let drive_path = self.config.drive().device_path();

            if !partition_table_exists(&drive_path) {
                self.create_partition_table();
            }

            let mut drive = fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(&drive_path)
                .unwrap();

            let mut gpt = GPT::find_from(&mut drive).unwrap();

            let mut unused_indexes: Vec<u32> = gpt
                .iter()
                .filter(|(_, partition)| partition.is_unused())
                .map(|(index, _)| index)
                .collect();

            unused_indexes.reverse();

            if self.efi && self.partitions.efi().is_none() {
                add_efi_partition(&mut gpt, unused_indexes.pop().unwrap());
            }
            if self.partitions.boot().is_none() {
                add_boot_partition(&mut gpt, unused_indexes.pop().unwrap());
            }
            if self.partitions.root().is_none() {
                add_root_partition(&mut gpt, unused_indexes.pop().unwrap());
            }
            gpt.write_into(&mut drive).unwrap();
            drop(drive);

            // Crutch: lsblk doesn't keep up without timeout.
            std::thread::sleep(std::time::Duration::from_secs(1));

            self.partitions.update();

            // Crutch: make sure partitions are found.
            assert!(self.partitions.efi().is_some(), self.efi);
            assert!(self.partitions.boot().is_some());
            assert!(self.partitions.root().is_some());

            Ok(self)
        }

        pub fn encrypt_partition(&mut self) -> &mut Self {
            let mut password = self.config.drive().password().to_owned();

            if password.is_empty() {
                password.truncate(0);
                println!("Type the password for the disk decryption");
                stdin().read_line(&mut password).unwrap();
            } else {
                password.push('\n');
            }

            println!("Encrypting the disk...");
            self.luks_format(&password);
            self.luks_open(&password);
            self
        }

        pub fn format_partitions(&mut self) -> &mut Self {
            let mkfs = format!("mkfs.{}", self.config.drive().fs());

            // TODO: what is the meaning of \^64bit?
            let status = Command::new(&mkfs)
                .arg(self.partitions.boot().unwrap())
                .spawn()
                .unwrap()
                .wait()
                .unwrap();

            assert!(status.success());

            if self.config.lvm().enabled() {
                panic!();
            //   mkfs."$FS" /dev/$VG_NAME/root
            //   mkfs."$FS" /dev/$VG_NAME/home
            } else {
                let status = Command::new(&mkfs)
                    .arg(&self.fs_root.as_ref().unwrap())
                    .spawn()
                    .unwrap()
                    .wait()
                    .unwrap();

                assert!(status.success());
            }
            if self.efi && self.format_efi {
                let status = Command::new("mkfs.fat")
                    .arg("-F32")
                    .arg(self.partitions.efi().unwrap())
                    .spawn()
                    .unwrap()
                    .wait()
                    .unwrap();

                assert!(status.success());
            }
            self
        }

        pub fn mount_partitions(&mut self) -> &mut Self {
            if self.config.lvm().enabled() {
                // TODO: add lvm support
                //   mount /dev/$VG_NAME/root /mnt
                //   mkdir /mnt/home
                //   mount /dev/$VG_NAME/home /mnt/home
                panic!();
            } else {
                let status = Command::new("mount")
                    .arg(self.fs_root.as_ref().unwrap())
                    .arg("/mnt")
                    .spawn()
                    .unwrap()
                    .wait()
                    .unwrap();

                assert!(status.success());
            }
            create_dir("/mnt/boot").unwrap();

            let status = Command::new("mount")
                .arg(self.partitions.boot().unwrap())
                .arg("/mnt/boot")
                .spawn()
                .unwrap()
                .wait()
                .unwrap();
            assert!(status.success());

            if self.efi {
                create_dir("/mnt/efi").unwrap();

                let status = Command::new("mount")
                    .arg(self.partitions.efi().unwrap())
                    .arg("/mnt/efi")
                    .spawn()
                    .unwrap()
                    .wait()
                    .unwrap();
                assert!(status.success());
            }
            self
        }

        pub fn configure_mirrors(&mut self) -> &mut Self {
            if !Path::new("/etc/pacman.d/mirrorlist.backup").exists() {
                fs::rename(
                    "/etc/pacman.d/mirrorlist",
                    "/etc/pacman.d/mirrorlist.backup",
                )
                .unwrap();
            }
            let status = Command::new("reflector")
                .args(&[
                    "--protocol",
                    "https",
                    "--latest",
                    "70",
                    "--sort",
                    "rate",
                    "--save",
                    "/etc/pacman.d/mirrorlist",
                ])
                .spawn()
                .unwrap()
                .wait()
                .unwrap();

            assert!(status.success());
            self
        }

        pub fn enable_multilib(&mut self) -> &mut Self {
            let mut conf_content = String::new();
            let mut pacman_conf = File::open("/etc/pacman.conf").unwrap();
            pacman_conf.read_to_string(&mut conf_content).unwrap();

            let conf_content = conf_content.replace(
                "#[multilib]\n#Include = /etc/pacman.d/mirrorlist",
                "[multilib]\nInclude = /etc/pacman.d/mirrorlist",
            );
            let mut pacman_conf = File::create("/etc/pacman.conf").unwrap();
            pacman_conf.write_all(conf_content.as_bytes()).unwrap();
            self
        }

        pub fn pacstrap(&mut self) -> &mut Self {
            let packages = self.config.packages().pacman();

            let status = Command::new("pacstrap")
                .arg("/mnt")
                .args(&packages)
                .spawn()
                .unwrap()
                .wait()
                .unwrap();

            assert!(status.success());
            self
        }

        pub fn generate_fstab(&mut self) -> &mut Self {
            let file = fs::OpenOptions::new()
                .append(true)
                .open("/mnt/etc/fstab")
                .unwrap();

            let status = Command::new("genfstab")
                .arg("/mnt")
                .stdout(file)
                .spawn()
                .unwrap()
                .wait()
                .unwrap();

            assert!(status.success());
            self
        }

        pub fn chroot(&mut self) {
            let status = Command::new("cp")
                .arg("-rv")
                .arg(exe_dir())
                .arg("/mnt/root")
                .spawn()
                .unwrap()
                .wait()
                .unwrap();

            assert!(status.success());

            let status = Command::new("arch-chroot")
                .args(&["/mnt", "/root/installer/start", "chroot-install"])
                .spawn()
                .unwrap()
                .wait()
                .unwrap();

            assert!(status.success());
        }
    }
}

pub fn main(config: &Config) -> ALIResult<()> {
    let mut installer = private::Installer::new(config);

    installer
        .check_network()?
        .check_drive()?
        .check_mounted()?
        .check_luks_open()?
        .partition()?;

    if config.drive().encryption() {
        installer.encrypt_partition();
    }
    installer
        .format_partitions()
        .mount_partitions()
        .configure_mirrors();

    if config.system().multilib() {
        installer.enable_multilib();
    }
    installer.pacstrap().generate_fstab().chroot();
    Ok(())
}
