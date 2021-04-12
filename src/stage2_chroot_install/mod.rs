use crate::{config::Config, errors::ALIResult};

pub mod stage2_package_configurator;

mod private {
    use {
        super::stage2_package_configurator::Stage2PackageConfigurator,
        crate::{
            config::Config,
            utils::{pacman_install, su_command, sudo_passwd_off, sudo_passwd_on},
        },
        std::env::{current_dir, set_current_dir},
    };

    pub struct Stage2ChrootInstaller<'a> {
        config: &'a Config,
    }

    impl<'a> Stage2ChrootInstaller<'a> {
        pub fn new(config: &'a Config) -> Stage2ChrootInstaller {
            Stage2ChrootInstaller { config }
        }

        pub fn install_packages(&mut self) -> &mut Self {
            pacman_install(&self.config.packages().pacman());
            self
        }

        pub fn configure_packages(&mut self) -> &mut Self {
            Stage2PackageConfigurator::new(self.config).run();
            self
        }

        pub fn install_aur_helper(&mut self) -> &mut Self {
            let helper = self.config.system().aur_helper();

            if helper.is_empty() {
                return self;
            }
            pacman_install(&["base-devel"]);
            let working_dir = format!("/tmp/{}", helper);
            let url = format!("https://aur.archlinux.org/{}.git", helper);
            let user = self.config.system().arch_username();

            su_command(user, "git", &["clone", &url, &working_dir])
                .spawn()
                .unwrap()
                .wait()
                .unwrap();

            let installer_dir = current_dir().unwrap();
            set_current_dir(working_dir).unwrap();
            sudo_passwd_off(user);

            su_command(user, "makepkg", &["-si", "--noconfirm"])
                .spawn()
                .unwrap()
                .wait()
                .unwrap();

            sudo_passwd_on(user);
            set_current_dir(installer_dir).unwrap();
            self
        }

        pub fn install_aur_packages(&mut self) -> &mut Self {
            let cmd = self.config.system().aur_helper();
            let user = self.config.system().arch_username();

            if cmd.is_empty() {
                return self;
            }
            sudo_passwd_off(user);
            self.config.packages().aur().into_iter().for_each(|pkg| {
                su_command(user, cmd, &["-S", "--noconfirm", pkg])
                    .spawn()
                    .unwrap()
                    .wait()
                    .unwrap();
            });
            sudo_passwd_on(user);
            self
        }
    }
}

pub fn main(config: &Config) -> ALIResult<()> {
    private::Stage2ChrootInstaller::new(config)
        .install_packages()
        .configure_packages()
        .install_aur_helper()
        .install_aur_packages();
    Ok(())
}
