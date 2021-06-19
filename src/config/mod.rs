use std::{
    convert::Infallible,
    ops::{Deref, RangeInclusive},
    str::FromStr,
};

use {
    crate::{errors::ALIResult, utils::exe_dir},
    std::{path::PathBuf, rc::Rc},
};

mod toml;
mod validator;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum InstallStep {
    Partition,
    Encrypt,
    Format,
    Mount,
    Mirrors,
    Multilib,
    Pacstrap,
    Fstab,
    Chroot,
}

impl FromStr for InstallStep {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "partition" => Ok(InstallStep::Partition),
            "encrypt" => Ok(InstallStep::Encrypt),
            "format" => Ok(InstallStep::Format),
            "mount" => Ok(InstallStep::Mount),
            "mirrors" => Ok(InstallStep::Mirrors),
            "multilib" => Ok(InstallStep::Multilib),
            "pacstrap" => Ok(InstallStep::Pacstrap),
            "fstab" => Ok(InstallStep::Fstab),
            "chroot" => Ok(InstallStep::Chroot),
            // TODO: use Result
            _ => panic!("Unknown installation step: {}", s),
        }
    }
}

#[derive(Debug)]
pub struct InstallStepRange(RangeInclusive<InstallStep>);

impl Deref for InstallStepRange {
    type Target = RangeInclusive<InstallStep>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FromStr for InstallStepRange {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (start, end) = s.split_once("..").unwrap();

        let start = match start {
            "" => InstallStep::Partition,
            step => InstallStep::from_str(step)?,
        };
        let end = match end {
            "" => InstallStep::Chroot,
            step => InstallStep::from_str(step)?,
        };
        Ok(Self(RangeInclusive::new(start, end)))
    }
}

#[derive(Debug)]
pub(crate) struct LiveCD {
    toml: Rc<toml::Toml>,
    working_dir: PathBuf,
}

impl LiveCD {
    fn new(toml: Rc<toml::Toml>, working_dir: Option<PathBuf>) -> LiveCD {
        let default_working_dir = || {
            let mut dir = exe_dir();
            dir.push("archiso");
            dir
        };
        let working_dir = working_dir.unwrap_or_else(default_working_dir);
        LiveCD { toml, working_dir }
    }
}

impl LiveCD {
    pub(crate) fn profile(&self) -> &str {
        self.toml.live_cd.profile()
    }

    pub(crate) fn installer_location(&self) -> PathBuf {
        self.toml.live_cd.installer_location()
    }

    pub(crate) fn working_dir(&self) -> &PathBuf {
        &self.working_dir
    }
}

#[derive(Debug)]
pub struct Config {
    toml: Rc<toml::Toml>,
    pub(crate) live_cd: LiveCD,
}

impl Config {
    pub fn new(path: &mut PathBuf, archiso_working_dir: Option<PathBuf>) -> ALIResult<Config> {
        let toml: toml::Toml = toml::Toml::new(path)?;
        let toml = Rc::new(toml);
        let live_cd = LiveCD::new(toml.clone(), archiso_working_dir);

        let config = Config { toml, live_cd };
        Ok(config)
    }

    pub fn validate(&self) -> ALIResult<()> {
        println!("{:?}", self.toml.partitions);
        validator::ConfigValidator::new(self.toml.clone()).validate()?;
        Ok(())
    }

    pub(crate) fn partitions(&self) -> &toml::Partitions {
        &self.toml.partitions
    }

    pub(crate) fn system(&self) -> &toml::System {
        &self.toml.system
    }

    pub(crate) fn shadowsocks(&self) -> &toml::ShadowSocks {
        &self.toml.shadowsocks
    }

    pub(crate) fn packages(&self) -> &toml::Packages {
        &self.toml.packages
    }
}
