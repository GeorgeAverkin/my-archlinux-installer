use crate::constants::BIOS_BOOT_GUID;

use {
    crate::utils::command::Command,
    gptman::GPT,
    std::fs::File,
    std::{io::Read, path::PathBuf, process::Stdio},
};

#[derive(Debug)]
pub(crate) struct Partitions {
    drive: PathBuf,
    efi: Option<PathBuf>,
    boot: Option<PathBuf>,
    root: Option<PathBuf>,
}

impl Partitions {
    pub(crate) fn new(drive: PathBuf, root: &str, boot: &str, efi: &str) -> Self {
        let root = Partitions::str_to_path(root);
        let boot = Partitions::str_to_path(boot);
        let efi = Partitions::str_to_path(efi);

        // TODO: validate partitions
        Partitions {
            drive,
            root,
            efi,
            boot,
        }
        //     partitions.update();
    }

    fn str_to_path(str: &str) -> Option<PathBuf> {
        if str.is_empty() {
            return None;
        }
        if str.starts_with("/") {
            Some(PathBuf::from(str))
        } else {
            let mut path = PathBuf::from("/dev");
            path.push(str);
            Some(path)
        }
    }

    pub(crate) fn update(&mut self) {
        let mut buffer = String::new();

        let mut lsblk = Command::new("lsblk")
            .args(&["--output=PATH,PARTLABEL", "--noheadings"])
            .arg(&self.drive)
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();

        lsblk
            .stdout
            .as_mut()
            .unwrap()
            .read_to_string(&mut buffer)
            .unwrap();

        let status = lsblk.wait().unwrap();
        assert!(status.success());

        let buffer: Vec<(&str, &str)> = buffer
            .lines()
            .map(|line| {
                let mut line = line.splitn(2, ' ');
                let path = line.next().unwrap();
                let label = line.next().unwrap().trim();
                (path, label)
            })
            .collect();

        buffer.into_iter().for_each(|(path, label)| {
            if label == "EFI system partition" {
                self.efi = Some(PathBuf::from(path));
                return;
            }
            if label == "Arch Linux boot partition" {
                self.boot = Some(PathBuf::from(path));
                return;
            }
            if label == "Arch Linux root partition" {
                self.root = Some(PathBuf::from(path));
                return;
            }
        });
    }

    pub(crate) fn bios_partition_exists(&self) -> bool {
        let mut file = File::open(&self.drive).unwrap();
        let gpt = GPT::find_from(&mut file).unwrap();

        for (_, partition) in gpt.iter() {
            if partition.partition_type_guid == BIOS_BOOT_GUID {
                return true;
            }
        }
        false
    }

    pub(crate) fn efi(&self) -> Option<&PathBuf> {
        self.efi.as_ref()
    }

    pub(crate) fn boot(&self) -> Option<&PathBuf> {
        self.boot.as_ref()
    }

    pub(crate) fn root(&self) -> Option<&PathBuf> {
        self.root.as_ref()
    }
}
