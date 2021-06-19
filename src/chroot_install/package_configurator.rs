use {
    crate::{config::Config, utils::Service},
    std::{fs, io::prelude::*},
};

pub(crate) struct PackageConfigurator<'a> {
    config: &'a Config,
}

impl<'a> PackageConfigurator<'a> {
    pub(crate) fn new(config: &'a Config) -> PackageConfigurator {
        PackageConfigurator { config }
    }

    pub(crate) fn run(&mut self) {
        let packages = self.config.packages().pacman_system();

        if packages.contains(&"sudo") {
            self.configure_sudo();
        }
        if packages.contains(&"dhcpcd") {
            self.configure_dhcpcd();
        }
        if packages.contains(&"networkmanager") {
            self.configure_networkmanager();
        }
    }

    fn configure_sudo(&mut self) -> &mut Self {
        let buffer = format!("{} ALL=(ALL) ALL\n", self.config.system().arch_username());

        fs::OpenOptions::new()
            .append(true)
            .open("/etc/sudoers")
            .unwrap()
            .write_all(buffer.as_bytes())
            .unwrap();
        self
    }

    fn configure_dhcpcd(&mut self) -> &mut Self {
        Service("dhcpcd").enable();
        self
    }

    fn configure_networkmanager(&mut self) -> &mut Self {
        Service("NetworkManager").enable();
        self
    }
}
