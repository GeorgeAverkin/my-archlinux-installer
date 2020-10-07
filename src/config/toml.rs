use crate::errors::ConfigNotFoundError;

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

fn map_to_conf_not_found_err(path: &mut PathBuf) -> ConfigNotFoundError {
    ConfigNotFoundError {
        path: path.to_string_lossy().into_owned(),
    }
}

#[derive(Deserialize, Debug)]
pub struct Drive {
    bootloader_id: String,
    crypt_mapping: String,
    password: String,
    device: String,
    encryption: bool,
    fs: String,
}

impl Drive {
    pub fn bootloader_id(&self) -> &str {
        &self.bootloader_id
    }

    pub fn crypt_mapping(&self) -> &str {
        &self.crypt_mapping
    }

    pub fn crypt_mapping_path(&self) -> PathBuf {
        dev_path(Some(format!("mapper/{}", self.crypt_mapping)))
    }

    pub fn password(&self) -> &str {
        &self.password
    }

    pub fn device(&self) -> &str {
        &self.device
    }

    pub fn device_path(&self) -> PathBuf {
        dev_path(Some(&self.device))
    }

    pub fn encryption(&self) -> bool {
        self.encryption
    }

    pub fn fs(&self) -> &str {
        &self.fs
    }
}

#[derive(Deserialize, Debug)]
pub struct Partitions {
    root: String,
    boot: String,
    efi: String,
}

impl Partitions {
    pub fn root(&self) -> &str {
        &self.root
    }

    pub fn boot(&self) -> &str {
        &self.boot
    }

    pub fn efi(&self) -> &str {
        &self.efi
    }
}

#[derive(Deserialize, Debug)]
pub struct System {
    arch_host: String,
    arch_username: String,
    arch_userpass: String,
    aur_helper: String,
    mirror_protocol: String,
    multilib: bool,
    timezone: PathBuf,
}

impl System {
    pub fn arch_host(&self) -> &str {
        &self.arch_host
    }

    pub fn arch_username(&self) -> &str {
        &self.arch_username
    }

    pub fn arch_userpass(&self) -> &str {
        &self.arch_userpass
    }

    pub fn aur_helper(&self) -> &str {
        &self.aur_helper
    }

    pub fn mirror_protocol(&self) -> &str {
        &self.mirror_protocol
    }

    pub fn multilib(&self) -> bool {
        self.multilib
    }

    pub fn timezone(&self) -> PathBuf {
        let mut tz = PathBuf::from("/usr/share/zoneinfo");
        tz.push(&self.timezone);
        tz
    }
}

#[derive(Deserialize, Debug)]
pub struct ShadowSocks {
    password: String,
    server: String,
}

impl ShadowSocks {
    pub fn password(&self) -> &str {
        &self.password
    }

    pub fn server(&self) -> &str {
        &self.server
    }
}

#[derive(Deserialize, Debug)]
pub struct LiveCD {
    profile: String,
    installer_location: PathBuf,
}

impl LiveCD {
    pub fn profile(&self) -> &str {
        &self.profile
    }
    pub fn installer_location(&self) -> PathBuf {
        let mut location = exe_dir();
        location.push(&self.installer_location);
        location
    }
}

#[derive(Deserialize, Debug)]
pub struct Packages {
    archiso: Vec<String>,
    aur: Vec<String>,
    pacman: Vec<String>,
    vscode: Vec<String>,
}

impl Packages {
    pub fn archiso(&self) -> Vec<&str> {
        self.archiso.iter().map(|pkg| &**pkg).collect()
    }

    pub fn aur(&self) -> Vec<&str> {
        self.aur.iter().map(|pkg| &**pkg).collect()
    }

    pub fn pacman(&self) -> Vec<&str> {
        self.pacman.iter().map(|pkg| &**pkg).collect()
    }

    pub fn vscode(&self) -> Vec<&str> {
        self.vscode.iter().map(|pkg| &**pkg).collect()
    }
}

#[derive(Deserialize, Debug)]
pub struct Toml {
    pub drive: Drive,
    pub partitions: Partitions,
    pub system: System,
    pub shadowsocks: ShadowSocks,
    pub live_cd: LiveCD,
    pub packages: Packages,
}

impl Toml {
    pub fn new(path: &mut PathBuf) -> ALIResult<Toml> {
        let file = {
            let path: &PathBuf = &path;
            File::open(path)
        };
        let mut file = file.map_err(|_| map_to_conf_not_found_err(path))?;

        let mut buffer = Vec::new();

        file.read_to_end(&mut buffer)
            .map_err(|_| map_to_conf_not_found_err(path))?;

        let toml: Toml = toml::from_slice(&buffer).unwrap();
        Ok(toml)
    }
}
