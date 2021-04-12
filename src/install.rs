use crate::{
    config::InstallStep,
    errors::{DeviceNotFoundError, NetworkError},
};

use {
    crate::{
        config::Config,
        errors::ALIResult,
        utils::{answer, command::Command, exe_dir, Mounted},
    },
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

const EFI_GUID: [u8; 16] = [
    40, 115, 42, 193, 31, 248, 210, 17, 186, 75, 0, 160, 201, 62, 201, 59,
];

const LINUX_GUID: [u8; 16] = [
    0xAF, 0x3D, 0xC6, 0x0F, 0x83, 0x84, 0x72, 0x47, 0x8E, 0x79, 0x3D, 0x69, 0xD8, 0x47, 0x7D, 0xE4,
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
    config: &'a mut Config,
    efi: bool,
    fs_root: Option<PathBuf>,
    efi_formatted: bool,
    boot_formatted: bool,
    // root_formatted: bool,
}

impl Installer<'_> {
    pub fn new(config: &mut Config) -> Installer {
        let efi = Path::new("/sys/firmware/efi").exists();
        let mut fs_root: Option<PathBuf> = None;

        if config.drive().encryption() {
            fs_root = Some(config.drive().crypt_mapping_path());
        }
        Installer {
            config,
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
            .arg(self.config.partitions.root().unwrap())
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
            .arg(self.config.partitions.root().unwrap())
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

    pub fn contains_step(&self, step: InstallStep) -> bool {
        let from = self.config.install_step_from();
        let to = self.config.install_step_to();

        if from.is_none() && to.is_none() {
            return true;
        }
        let from = from.map(|step| step as i8).unwrap_or(-1);
        let to = to.map(|step| step as i8).unwrap_or(127);
        let step = step as i8;
        from <= step && step <= to
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

    pub fn check_partitions_formatted(&mut self) -> &mut Self {
        let boot_fs_type = self.get_fs_type(self.config.partitions.boot().unwrap());

        match boot_fs_type.as_str() {
            "ext4" => self.boot_formatted = true,
            "" => (),
            fs_type => panic!("Expected boot fs to be ext4, got: {}", fs_type),
        }
        if self.efi {
            let efi_fs_type = self.get_fs_type(self.config.partitions.efi().unwrap());

            match efi_fs_type.as_str() {
                "vfat" => self.efi_formatted = true,
                "" => self.efi_formatted = false,
                fs_type => panic!("Expected efi fs to be vfat, got: {}", fs_type),
            }
        }
        self
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
            return Err(NetworkError {}.into());
        }
        Ok(self)
    }

    pub fn check_drive(&mut self) -> ALIResult<&mut Self> {
        let drive = self.config.drive().device_path();

        if !drive.exists() {
            return Err(DeviceNotFoundError {
                device: drive.to_string_lossy().into_owned(),
            }
            .into());
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
                panic!("TODO check_mounted");
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

        if self.efi && self.config.partitions.efi().is_none() {
            add_efi_partition(&mut gpt, unused_indexes.pop().unwrap());
        }
        if self.config.partitions.boot().is_none() {
            add_boot_partition(&mut gpt, unused_indexes.pop().unwrap());
        }
        if self.config.partitions.root().is_none() {
            add_root_partition(&mut gpt, unused_indexes.pop().unwrap());
        }
        gpt.write_into(&mut drive).unwrap();
        drop(drive);

        // Crutch: lsblk doesn't keep up without timeout.
        std::thread::sleep(std::time::Duration::from_secs(1));

        self.config.partitions.update();

        // Crutch: make sure partitions are found.
        assert!(self.config.partitions.efi().is_some() == self.efi);
        assert!(self.config.partitions.boot().is_some());
        assert!(self.config.partitions.root().is_some());

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
            .arg(self.config.partitions.boot().unwrap())
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
                .arg(self.config.partitions.efi().unwrap())
                .spawn()
                .unwrap()
                .wait()
                .unwrap();

            assert!(status.success());
            self.efi_formatted = true;
        }
        self
    }

    pub fn mount_partitions(&mut self) -> &mut Self {
        self.mount(self.fs_root.as_ref().unwrap(), "/mnt");
        create_dir("/mnt/boot").unwrap();

        self.mount(self.config.partitions.boot().unwrap(), "/mnt/boot");

        if self.efi {
            create_dir("/mnt/efi").unwrap();
            self.mount(self.config.partitions.efi().unwrap(), "/mnt/efi");
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
            .args(&["/mnt", "/root/installer/cli", "chroot-install"])
            .spawn()
            .unwrap()
            .wait()
            .unwrap();

        assert!(status.success());
    }

    pub fn run(&mut self) -> ALIResult<()> {
        let encryption = self.config.drive().encryption();
        let multilib = self.config.system().multilib();

        self.check_network()?
            .check_drive()?
            // .check_mounted()?
            // .check_luks_open()?
            ;

        if self.contains_step(InstallStep::Partition) {
            self.partition()?;
        }
        if self.contains_step(InstallStep::Encrypt) && encryption {
            self.encrypt_partition();
        }
        if self.contains_step(InstallStep::Format) {
            self.format_partitions();
        }
        if self.contains_step(InstallStep::Mount) {
            self.mount_partitions();
        }
        if self.contains_step(InstallStep::Mirrors) {
            self.configure_mirrors();
        }
        if self.contains_step(InstallStep::Multilib) && multilib {
            self.enable_multilib();
        }
        if self.contains_step(InstallStep::Pacstrap) {
            self.pacstrap();
        }
        if self.contains_step(InstallStep::Fstab) {
            self.generate_fstab();
        }
        if self.contains_step(InstallStep::Chroot) {
            self.chroot();
        }
        Ok(())
    }
}

pub fn main(config: &mut Config) -> ALIResult<()> {
    Installer::new(config).run()
}
