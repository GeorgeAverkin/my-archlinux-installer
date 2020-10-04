use {
    crate::{errors::ALIResult, utils::exe_dir},
    clap::ArgMatches,
    std::path::PathBuf,
    std::rc::Rc,
};

mod toml_conf {
    use {
        crate::{
            errors::{ALIError, ALIResult, ErrorKind},
            utils::exe_dir,
        },
        serde::Deserialize,
        std::{
            fs::File,
            io::prelude::*,
            mem::swap,
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

    fn map_to_conf_not_found_err(path: &mut PathBuf) -> ALIError {
        let mut owned_path = PathBuf::new();
        swap(&mut owned_path, path);
        ALIError::new(ErrorKind::ConfigNotFound(owned_path))
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
    pub struct Lvm {
        lvm_home_size: usize,
        lvm_root_size: usize,
        enabled: bool,
        vg_name: String,
    }

    impl Lvm {
        pub fn lvm_home_size(&self) -> usize {
            self.lvm_home_size
        }

        pub fn lvm_root_size(&self) -> usize {
            self.lvm_root_size
        }

        pub fn enabled(&self) -> bool {
            self.enabled
        }

        pub fn vg_name(&self) -> &str {
            &self.vg_name
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
        pub system: System,
        pub shadowsocks: ShadowSocks,
        pub lvm: Lvm,
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
}

#[derive(Debug)]
pub struct LiveCD {
    toml: Rc<toml_conf::Toml>,
    working_dir: PathBuf,
}

impl LiveCD {
    fn new(toml: Rc<toml_conf::Toml>, cli: &ArgMatches) -> LiveCD {
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

#[derive(Debug)]
struct InnerConfig {
    live_cd: LiveCD,
}

impl InnerConfig {
    fn new(toml: Rc<toml_conf::Toml>, cli: &ArgMatches) -> InnerConfig {
        let live_cd = LiveCD::new(toml, cli);
        InnerConfig { live_cd }
    }
}

#[derive(Debug)]
pub struct Config {
    toml: Rc<toml_conf::Toml>,
    inner: InnerConfig,
}

impl Config {
    pub fn new(path: &mut PathBuf, cli: &ArgMatches) -> ALIResult<Config> {
        let toml: toml_conf::Toml = toml_conf::Toml::new(path)?;
        let toml = Rc::new(toml);
        let inner = InnerConfig::new(toml.clone(), cli);
        let config = Config { toml, inner };
        Ok(config)
    }

    pub fn drive(&self) -> &toml_conf::Drive {
        &self.toml.drive
    }

    pub fn system(&self) -> &toml_conf::System {
        &self.toml.system
    }

    pub fn shadowsocks(&self) -> &toml_conf::ShadowSocks {
        &self.toml.shadowsocks
    }

    pub fn lvm(&self) -> &toml_conf::Lvm {
        &self.toml.lvm
    }

    pub fn live_cd(&self) -> &LiveCD {
        &self.inner.live_cd
    }

    pub fn packages(&self) -> &toml_conf::Packages {
        &self.toml.packages
    }
}
