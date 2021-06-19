use crate::errors::ALIResult;

use {
    crate::{
        config::Config,
        utils::{command::Command, git_clone, pacman_install, su_command, Service},
    },
    std::{
        fs::{self, create_dir_all, File},
        io::prelude::*,
        path::{Path, PathBuf},
    },
};

pub(crate) struct Stage2PackageConfigurator<'a> {
    config: &'a Config,
}

impl<'a> Stage2PackageConfigurator<'a> {
    pub(crate) fn new(config: &'a Config) -> Stage2PackageConfigurator {
        Stage2PackageConfigurator { config }
    }

    fn home<P: AsRef<Path>>(&self, path: Option<P>) -> PathBuf {
        let mut full_path = PathBuf::from("/home");
        full_path.push(self.config.system().arch_username());

        if let Some(path) = path {
            full_path.push(path);
        }
        full_path
    }

    fn chown_to_user<P: AsRef<Path>>(&self, path: P) {
        let mut user = self.config.system().arch_username().to_owned();
        user.push(':');

        let status = Command::new("chown")
            .args(&["-R", &user])
            .arg(path.as_ref())
            .spawn()
            .unwrap()
            .wait()
            .unwrap();

        assert!(status.success());
    }

    pub(crate) fn run(&mut self) {
        let packages = self.config.packages().pacman_system();

        if packages.contains(&"zsh") {
            self.configure_zsh().unwrap();
        }
        if packages.contains(&"code") {
            self.configure_code();
        }
        if packages.contains(&"shadowsocks-libev") {
            self.configure_shadowsocks_libev();
        }
        if packages.contains(&"gvfs-google") {
            self.configure_gvfs_google();
        }
        if packages.contains(&"virt-manager") {
            self.configure_virt_manager();
        }
        if packages.contains(&"gdm") {
            self.configure_gdm();
        }
    }

    fn configure_zsh(&mut self) -> ALIResult<&mut Self> {
        let change_shell = |user: &str| {
            let status = Command::new("chsh")
                .args(&["--shell=/bin/zsh", user])
                .spawn()
                .unwrap()
                .wait()
                .unwrap();

            assert!(status.success());
        };
        let username = self.config.system().arch_username();
        change_shell("root");
        change_shell(username);
        fs::rename("/root/installer/zshrc", self.home(Some(".zshrc"))).unwrap();
        self.chown_to_user(self.home(Some(".zshrc")));

        git_clone(
            "https://github.com/ohmyzsh/ohmyzsh.git",
            self.home(Some(".oh-my-zsh")),
        )?;
        git_clone(
            "https://github.com/zsh-users/zsh-syntax-highlighting.git",
            self.home(Some(".oh-my-zsh/custom/plugins/zsh-syntax-highlighting")),
        )?;
        git_clone(
            "https://github.com/zsh-users/zsh-autosuggestions.git",
            self.home(Some(".oh-my-zsh/custom/plugins/zsh-autosuggestions")),
        )?;
        git_clone(
            "https://github.com/bhilburn/powerlevel9k.git",
            self.home(Some(".oh-my-zsh/custom/themes/powerlevel9k")),
        )?;
        self.chown_to_user(self.home(Some(".oh-my-zsh")));
        Ok(self)
    }

    fn configure_code(&mut self) -> &mut Self {
        pacman_install(&["ttf-droid", "ttf-ubuntu-font-family"]);
        create_dir_all("/root/.config/Code - OSS/User").unwrap();

        fs::rename(
            "/root/installer/vscode_root.json",
            "/root/.config/Code - OSS/User/settings.json",
        )
        .unwrap();

        let username = self.config.system().arch_username();
        let mut config_path = PathBuf::from("/home");
        config_path.push(username);
        config_path.push(".config");

        let mut path = config_path.clone();
        path.push("Code - OSS/User");
        create_dir_all(&path).unwrap();
        path.push("settings.json");
        fs::rename("/root/installer/vscode.json", &path).unwrap();

        let status = Command::new("chown")
            .args(&["-Rv", &format!("{}:", username)])
            .arg(&config_path)
            .spawn()
            .unwrap()
            .wait()
            .unwrap();

        assert!(status.success());
        let packages = self.config.packages().vscode();

        packages.iter().for_each(|pkg| {
            su_command(username, "code", &["--install-extension", pkg])
                .spawn()
                .unwrap()
                .wait()
                .unwrap();
        });
        if packages
            .iter()
            .any(|&pkg| pkg == "robbowen.synthwave-vscode")
        {
            // TODO: show failed packages
            Command::new("code")
                .args(&["--install-extension", "RobbOwen.synthwave-vscode"])
                .spawn()
                .unwrap()
                .wait()
                .unwrap();
        }
        self
    }

    fn configure_shadowsocks_libev(&mut self) -> &mut Self {
        fs::rename(
            "/root/installer/ss-local.service",
            "/etc/systemd/system/ss-local.service",
        )
        .unwrap();

        create_dir_all("/etc/shadowsocks-libev").unwrap();

        fs::rename(
            "/root/installer/ss-config.json",
            "/etc/shadowsocks-libev/config.json",
        )
        .unwrap();

        let mut buffer = String::new();

        File::open("/etc/shadowsocks-libev/config.json")
            .unwrap()
            .read_to_string(&mut buffer)
            .unwrap();

        let server = self.config.shadowsocks().server();
        let password = self.config.shadowsocks().password();

        buffer = buffer.replace("\"server\": \"\"", &format!("\"server\": \"{}\"", server));

        buffer = buffer.replace(
            "\"password\": \"\"",
            &format!("\"password\": \"{}\"", password),
        );

        File::create("/etc/shadowsocks-libev/config.json")
            .unwrap()
            .write_all(buffer.as_bytes())
            .unwrap();

        Service("ss-local").enable();
        self
    }

    fn configure_gvfs_google(&mut self) -> &mut Self {
        pacman_install(&["gnome-keyring"]);
        self
    }

    fn configure_virt_manager(&mut self) -> &mut Self {
        pacman_install(&["dnsmasq", "ebtables", "qemu-headless"]);
        Service("libvirtd").enable();
        self
    }

    fn configure_gdm(&mut self) -> &mut Self {
        Service("gdm").enable();
        self
    }
}
