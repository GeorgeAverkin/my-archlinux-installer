use snafu::ResultExt;

use crate::errors;

use {
    crate::{errors::ALIResult, utils::exe_dir},
    serde::Deserialize,
    std::{
        fs::File,
        io::prelude::*,
        path::{Path, PathBuf},
    },
    toml,
};

fn dev_path<P: AsRef<Path>>(path: Option<P>) -> PathBuf {
    let mut base_path = PathBuf::from("/dev");

    if let Some(path) = path {
        base_path.push(path);
    }
    base_path
}

#[derive(Deserialize, Debug)]
pub(crate) struct Partitions {
    pub(crate) efi: PartitionEfi,
    pub(crate) boot: PartitionBoot,
    pub(crate) root: PartitionRoot,
}

#[derive(Deserialize, Debug)]
pub(crate) struct PartitionEfi {
    pub(crate) device: String,
    pub(crate) bootloader_id: String,
}

#[derive(Deserialize, Debug)]
pub(crate) struct PartitionBoot {
    pub(crate) device: String,
}

#[derive(Deserialize, Debug)]
pub(crate) struct PartitionRoot {
    pub(crate) device: String,
    pub(crate) crypt_mapping: String,
    pub(crate) password: String,
    pub(crate) encryption: bool,
    pub(crate) fs: String,
}

impl PartitionRoot {
    pub(crate) fn crypt_mapping_path(&self) -> PathBuf {
        dev_path(Some(format!("mapper/{}", self.crypt_mapping)))
    }
}

#[derive(Deserialize, Debug)]
pub(crate) struct System {
    pub(crate) drive: String,
    arch_host: String,
    arch_username: String,
    arch_userpass: String,
    aur_helper: String,
    mirror_protocol: String,
    multilib: bool,
    timezone: PathBuf,
    pub(crate) connection_command: Vec<String>,
}

impl System {
    pub(crate) fn arch_host(&self) -> &str {
        &self.arch_host
    }

    pub(crate) fn arch_username(&self) -> &str {
        &self.arch_username
    }

    pub(crate) fn arch_userpass(&self) -> &str {
        &self.arch_userpass
    }

    pub(crate) fn aur_helper(&self) -> &str {
        &self.aur_helper
    }

    pub(crate) fn mirror_protocol(&self) -> &str {
        &self.mirror_protocol
    }

    pub(crate) fn multilib(&self) -> bool {
        self.multilib
    }

    pub(crate) fn timezone(&self) -> PathBuf {
        let mut tz = PathBuf::from("/usr/share/zoneinfo");
        tz.push(&self.timezone);
        tz
    }

    pub(crate) fn drive_path(&self) -> PathBuf {
        dev_path(Some(&self.drive))
    }
}

#[derive(Deserialize, Debug)]
pub(crate) struct ShadowSocks {
    password: String,
    server: String,
}

impl ShadowSocks {
    pub(crate) fn password(&self) -> &str {
        &self.password
    }

    pub(crate) fn server(&self) -> &str {
        &self.server
    }
}

#[derive(Deserialize, Debug)]
pub(crate) struct LiveCD {
    profile: String,
    installer_location: PathBuf,
}

impl LiveCD {
    pub(crate) fn profile(&self) -> &str {
        &self.profile
    }
    pub(crate) fn installer_location(&self) -> PathBuf {
        let mut location = exe_dir();
        location.push(&self.installer_location);
        location
    }
}

#[derive(Deserialize, Debug)]
pub(crate) struct Packages {
    archiso: Vec<String>,
    aur: Vec<String>,
    pacman: Vec<String>,
    pacman_system: Vec<String>,
    vscode: Vec<String>,
}

impl Packages {
    pub(crate) fn archiso(&self) -> Vec<&str> {
        self.archiso.iter().map(|pkg| &**pkg).collect()
    }

    pub(crate) fn aur(&self) -> Vec<&str> {
        self.aur.iter().map(|pkg| &**pkg).collect()
    }

    pub(crate) fn pacman(&self) -> Vec<&str> {
        self.pacman.iter().map(|pkg| &**pkg).collect()
    }

    pub(crate) fn pacman_system(&self) -> Vec<&str> {
        self.pacman_system.iter().map(|pkg| &**pkg).collect()
    }

    pub(crate) fn vscode(&self) -> Vec<&str> {
        self.vscode.iter().map(|pkg| &**pkg).collect()
    }
}

#[derive(Deserialize, Debug)]
pub(crate) struct Toml {
    pub(crate) partitions: Partitions,
    pub(crate) system: System,
    pub(crate) shadowsocks: ShadowSocks,
    pub(crate) live_cd: LiveCD,
    pub(crate) packages: Packages,
}

// fn map_to_conf_not_found_err(path: &mut PathBuf) -> ConfigNotFoundError {
//     ConfigNotFoundError {
//         path: path.to_string_lossy().into_owned(),
//     }
// }

impl Toml {
    pub(crate) fn new(path: &mut PathBuf) -> ALIResult<Toml> {
        let mut file = File::open(path).with_context(|_| errors::ConfigOpenSnafu)?;

        let mut buffer = Vec::new();

        file.read_to_end(&mut buffer)
            .with_context(|_| errors::ConfigReadSnafu)?;

        let toml: Toml = toml::from_slice(&buffer).unwrap();
        Ok(toml)
    }
}
