use snafu::ensure;

use crate::{
    config::{Config, InstallStep, InstallStepRange},
    constants::{EFI_GUID, LINUX_GUID},
    errors::{self, ALIResult},
    utils::{answer, command::Command, exe_dir, partitions::Partitions, Mounted},
};

use {
    gptman::GPT,
    std::{
        fs::{self, create_dir, File},
        io::{prelude::*, stdin},
        path::{Path, PathBuf},
        process::Stdio,
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

pub(crate) struct Installer<'a, 'b> {
    config: &'a Config,
    partitions: Partitions,
    steps: &'b InstallStepRange,
    efi: bool,
    fs_root: Option<PathBuf>,
    efi_formatted: bool,
    boot_formatted: bool,
    // root_formatted: bool,
}

impl<'a, 'b> Installer<'a, 'b> {
    pub(crate) fn new(config: &'a Config, steps: &'b InstallStepRange) -> Installer<'a, 'b> {
        let efi = Path::new("/sys/firmware/efi").exists();
        let mut fs_root: Option<PathBuf> = None;

        let partitions = Partitions::new(
            config.system().drive_path(),
            &config.partitions().root.device,
            &config.partitions().boot.device,
            &config.partitions().efi.device,
        );
        if config.partitions().root.encryption {
            fs_root = Some(config.partitions().root.crypt_mapping_path());
        }
        Installer {
            config,
            partitions,
            steps,
            efi,
            fs_root,
            efi_formatted: false,
            boot_formatted: false,
            // root_formatted: false,
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
            self.config.system().drive,
        );
        let mut answer = String::new();
        stdin().read_line(&mut answer).unwrap();

        if answer != "DO IT\n" {
            panic!();
            // return Err(ALIError::new(ErrorKind::EarlyExit));
        }
        let mut gdisk = Command::new("gdisk")
            .arg(&self.config.system().drive_path())
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

    fn mount(&self, partition: &PathBuf, mount_point: &str) {
        let status = Command::new("mount")
            .arg(partition)
            .arg(mount_point)
            .spawn()
            .unwrap()
            .wait()
            .unwrap();

        assert!(status.success());
    }

    fn get_fs_type(&self, partition: &PathBuf) -> String {
        let mut buffer = String::new();

        let mut blkid = Command::new("blkid")
            .args(&["-o", "value", "-s", "TYPE"])
            .arg(partition)
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();

        blkid
            .stdout
            .as_mut()
            .unwrap()
            .read_to_string(&mut buffer)
            .unwrap();

        let status = blkid.wait().unwrap();
        assert!(status.success());
        buffer.trim().to_owned()
    }

    pub(crate) fn check_partitions_formatted(&mut self) -> &mut Self {
        let boot_fs_type = self.get_fs_type(self.partitions.boot().unwrap());

        match boot_fs_type.as_str() {
            "ext4" => self.boot_formatted = true,
            "" => (),
            fs_type => panic!("Expected boot fs to be ext4, got: {}", fs_type),
        }
        if self.efi {
            let efi_fs_type = self.get_fs_type(self.partitions.efi().unwrap());

            match efi_fs_type.as_str() {
                "vfat" => self.efi_formatted = true,
                "" => self.efi_formatted = false,
                fs_type => panic!("Expected efi fs to be vfat, got: {}", fs_type),
            }
        }
        self
    }

    pub(crate) fn network_available(&mut self) -> ALIResult<bool> {
        let status = Command::new("ping")
            .args(&["-c", "1", "archlinux.org"])
            .stdout(Stdio::null())
            .spawn()
            .unwrap()
            .wait()
            .unwrap();

        let result = status.success();
        Ok(result)
    }

    fn connect_to_network(&self) -> ALIResult<()> {
        let program = self.config.system().connection_command.get(0).unwrap();
        let args = &self.config.system().connection_command[1..];

        if program == "nmcli" {
            let mut network_manager_active = Command::new("systemctl")
                .args(&["status", "NetworkManager"])
                .stdout(Stdio::null())
                .spawn()
                .unwrap()
                .wait()
                .unwrap()
                .success();

            if !network_manager_active {
                network_manager_active = Command::new("systemctl")
                    .args(&["start", "NetworkManager"])
                    .stdout(Stdio::null())
                    .spawn()
                    .unwrap()
                    .wait()
                    .unwrap()
                    .success();
            }
            assert!(network_manager_active);
        }

        let status = Command::new(program)
            .args(args)
            .spawn()
            .unwrap()
            .wait()
            .unwrap();
        assert!(status.success());
        Ok(())
    }

    pub(crate) fn check_drive(&mut self) -> ALIResult<&mut Self> {
        let device = self.config.system().drive_path();
        ensure!(device.exists(), errors::DeviceNotFoundSnafu { device });
        Ok(self)
    }

    pub(crate) fn check_mounted(&mut self) -> ALIResult<&mut Self> {
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
                panic!("TODO check_mounted");
            }
        }
        Ok(self)
    }

    pub(crate) fn check_luks_open(&mut self) -> ALIResult<&mut Self> {
        let mut crypt_mapping_path = PathBuf::from("/dev/mapper");
        let crypt_mapping = &self.config.partitions().root.crypt_mapping;
        crypt_mapping_path.push(crypt_mapping);

        if crypt_mapping_path.exists() {
            let question = format!("Crypt mapper \"{}\" is in use, close?", crypt_mapping);
            // TODO: move somethere
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
                panic!("TODO: check_luks_open");
            }
        }
        Ok(self)
    }

    pub(crate) fn partition(&mut self) -> ALIResult<&mut Self> {
        let drive_path = self.config.system().drive_path();

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
        if self.efi {
            assert!(self.partitions.efi().is_some());
        } else {
            assert!(self.partitions.bios_partition_exists());
        }
        assert!(self.partitions.boot().is_some());
        assert!(self.partitions.root().is_some());

        Ok(self)
    }

    pub(crate) fn encrypt_partition(&mut self) -> &mut Self {
        let mut password = self.config.partitions().root.password.to_owned();

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

    pub(crate) fn format_partitions(&mut self) -> &mut Self {
        let mkfs = format!("mkfs.{}", self.config.partitions().root.fs);

        // TODO: what is the meaning of \^64bit?
        let status = Command::new(&mkfs)
            .arg(self.partitions.boot().unwrap())
            .spawn()
            .unwrap()
            .wait()
            .unwrap();

        assert!(status.success());

        let status = Command::new(&mkfs)
            .arg(&self.fs_root.as_ref().unwrap())
            .spawn()
            .unwrap()
            .wait()
            .unwrap();

        assert!(status.success());

        if self.efi && !self.efi_formatted {
            let status = Command::new("mkfs.fat")
                .arg("-F32")
                .arg(self.partitions.efi().unwrap())
                .spawn()
                .unwrap()
                .wait()
                .unwrap();

            assert!(status.success());
            self.efi_formatted = true;
        }
        self
    }

    pub(crate) fn mount_partitions(&mut self) -> &mut Self {
        self.mount(self.fs_root.as_ref().unwrap(), "/mnt");
        create_dir("/mnt/boot").unwrap();

        self.mount(self.partitions.boot().unwrap(), "/mnt/boot");

        if self.efi {
            create_dir("/mnt/efi").unwrap();
            self.mount(self.partitions.efi().unwrap(), "/mnt/efi");
        }
        self
    }

    pub(crate) fn configure_mirrors(&mut self) -> &mut Self {
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
                "8",
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

    pub(crate) fn enable_multilib(&mut self) -> &mut Self {
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

    pub(crate) fn pacstrap(&mut self) -> &mut Self {
        let packages = self.config.packages().pacman_system();

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

    pub(crate) fn generate_fstab(&mut self) -> &mut Self {
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

    pub(crate) fn chroot(&mut self) {
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
            .args(&["/mnt", "/root/installer/cli", "chroot-install"])
            .spawn()
            .unwrap()
            .wait()
            .unwrap();

        assert!(status.success());
    }

    pub(crate) fn run(&mut self) -> ALIResult<()> {
        let encryption = self.config.partitions().root.encryption;
        let multilib = self.config.system().multilib();

        if !self.network_available()? {
            self.connect_to_network()?;
        }
        self.check_drive()?;
        // .check_mounted()?
        // .check_luks_open()?

        if self.steps.contains(&InstallStep::Partition) {
            self.partition()?;
        }
        if self.steps.contains(&InstallStep::Encrypt) && encryption {
            self.encrypt_partition();
        }
        if self.steps.contains(&InstallStep::Format) {
            self.format_partitions();
        }
        if self.steps.contains(&InstallStep::Mount) {
            self.mount_partitions();
        }
        if self.steps.contains(&InstallStep::Mirrors) {
            self.configure_mirrors();
        }
        if self.steps.contains(&InstallStep::Multilib) && multilib {
            self.enable_multilib();
        }
        if self.steps.contains(&InstallStep::Pacstrap) {
            self.pacstrap();
        }
        if self.steps.contains(&InstallStep::Fstab) {
            self.generate_fstab();
        }
        if self.steps.contains(&InstallStep::Chroot) {
            self.chroot();
        }
        Ok(())
    }
}

pub fn main(config: &mut Config, steps: &InstallStepRange) -> ALIResult<()> {
    Installer::new(config, steps).run()
}
