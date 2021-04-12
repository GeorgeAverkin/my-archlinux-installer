use {
    crate::{errors::ALIResult, utils::exe_dir},
    clap::ArgMatches,
    std::{path::PathBuf, rc::Rc},
};

mod partitions;
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

#[derive(Debug)]
pub struct LiveCD {
    toml: Rc<toml::Toml>,
    working_dir: PathBuf,
}

impl LiveCD {
    fn new(toml: Rc<toml::Toml>, cli: &ArgMatches) -> LiveCD {
        let default_working_dir = || {
            let mut dir = exe_dir();
            dir.push("archiso");
            dir
        };
        let working_dir = cli
            .subcommand_matches("archiso")
            .map(|x| x.args.get("working dir"))
            .and_then(|x| x)
            .map(|x| PathBuf::from(&x.vals[0]))
            .unwrap_or_else(default_working_dir);

        LiveCD { toml, working_dir }
    }
}

impl LiveCD {
    pub fn profile(&self) -> &str {
        self.toml.live_cd.profile()
    }

    pub fn installer_location(&self) -> PathBuf {
        self.toml.live_cd.installer_location()
    }

    pub fn working_dir(&self) -> &PathBuf {
        &self.working_dir
    }
}

// impl Partitions {
//     fn new(root: &str, boot: &str, efi: &str) -> Self {
//         let root = Partitions::str_to_path(root);
//         let boot = Partitions::str_to_path(boot);
//         let efi = Partitions::str_to_path(efi);

//         // TODO: validate partitions
//         Partitions { root, efi, boot }
//     }

//     fn str_to_path(str: &str) -> Option<PathBuf> {
//         if str.is_empty() {
//             return None;
//         }
//         if str.starts_with("/") {
//             Some(PathBuf::from(str))
//         } else {
//             let mut path = PathBuf::from("/dev");
//             path.push(str);
//             Some(path)
//         }
//     }

//     fn path_to_str(path: Option<&PathBuf>) -> Option<&str> {
//         let name = match path {
//             Some(name) => name,
//             _ => return None,
//         };
//         let name = name.iter().last().map(|os_str| os_str.to_str().unwrap());
//         name
//     }

//     pub fn root_absolute(&self) -> Option<&PathBuf> {
//         self.root.as_ref()
//     }

//     pub fn root(&self) -> Option<&str> {
//         Partitions::path_to_str(self.root.as_ref())
//     }

//     pub fn boot_absolute(&self) -> Option<&PathBuf> {
//         self.boot.as_ref()
//     }

//     pub fn boot(&self) -> Option<&str> {
//         Partitions::path_to_str(self.boot.as_ref())
//     }

//     pub fn efi_absolute(&self) -> Option<&PathBuf> {
//         self.efi.as_ref()
//     }

//     pub fn efi(&self) -> Option<&str> {
//         Partitions::path_to_str(self.efi.as_ref())
//     }
// }

#[derive(Debug)]
pub struct Config {
    toml: Rc<toml::Toml>,
    install_from: Option<InstallStep>,
    install_to: Option<InstallStep>,
    live_cd: LiveCD,
    pub partitions: partitions::Partitions,
}

impl Config {
    pub fn new(path: &mut PathBuf, cli: &ArgMatches) -> ALIResult<Config> {
        let toml: toml::Toml = toml::Toml::new(path)?;
        let toml = Rc::new(toml);
        let (install_from, install_to) = Config::set_install_steps(cli);
        let live_cd = LiveCD::new(toml.clone(), cli);

        let partitions = partitions::Partitions::new(
            toml.clone().drive.device_path(),
            toml.partitions.root(),
            toml.partitions.boot(),
            toml.partitions.efi(),
        );

        let config = Config {
            toml,
            install_from,
            install_to,
            live_cd,
            partitions,
        };
        Ok(config)
    }

    fn parse_install_steps(step: &str) -> Option<InstallStep> {
        match step {
            "partition" => Some(InstallStep::Partition),
            "encrypt" => Some(InstallStep::Encrypt),
            "format" => Some(InstallStep::Format),
            "mount" => Some(InstallStep::Mount),
            "mirrors" => Some(InstallStep::Mirrors),
            "multilib" => Some(InstallStep::Multilib),
            "pacstrap" => Some(InstallStep::Pacstrap),
            "fstab" => Some(InstallStep::Fstab),
            "chroot" => Some(InstallStep::Chroot),
            // TODO: use Result
            _ => panic!("Unknown installation step: {}", step),
        }
    }

    fn set_install_steps(cli: &ArgMatches) -> (Option<InstallStep>, Option<InstallStep>) {
        let install_cmd = match cli.subcommand_matches("install") {
            Some(cmd) => cmd,
            _ => return (None, None),
        };
        let steps = match install_cmd.args.get("steps") {
            Some(steps) => steps.vals.get(0),
            _ => return (None, None),
        };
        let steps = match steps {
            Some(steps) => steps,
            _ => return (None, None),
        };
        let mut steps = steps.to_str().unwrap_or("").split("..");
        let from = steps.next().unwrap();

        let from = match from {
            "" => Some(InstallStep::Partition),
            step => Config::parse_install_steps(step),
        };
        let to = match steps.next() {
            Some(step) => step,
            _ => return (from, from),
        };
        let to = match to {
            "" => Some(InstallStep::Chroot),
            step => Config::parse_install_steps(step),
        };
        (from, to)
    }

    pub fn validate(&self) -> ALIResult<()> {
        validator::ConfigValidator::new(self.toml.clone()).validate()?;
        Ok(())
    }

    pub fn drive(&self) -> &toml::Drive {
        &self.toml.drive
    }

    pub fn system(&self) -> &toml::System {
        &self.toml.system
    }

    pub fn shadowsocks(&self) -> &toml::ShadowSocks {
        &self.toml.shadowsocks
    }

    pub fn packages(&self) -> &toml::Packages {
        &self.toml.packages
    }

    pub fn install_step_from(&self) -> Option<InstallStep> {
        self.install_from
    }

    pub fn install_step_to(&self) -> Option<InstallStep> {
        self.install_to
    }

    pub fn live_cd(&self) -> &LiveCD {
        &self.live_cd
    }
}
