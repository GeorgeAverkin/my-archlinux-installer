use crate::{config::Config, errors::ALIResult};

pub mod package_configurator;

mod private {
    use {
        super::package_configurator::PackageConfigurator,
        crate::{
            config::Config,
            utils::{pacman_install, su_command, sudo_passwd_off, sudo_passwd_on, Partitions},
        },
        std::{
            env::{current_dir, set_current_dir},
            fs::{self, File},
            io::{prelude::*, stdin},
            os::unix::fs::symlink,
            path::Path,
            process::{Command, Stdio},
        },
    };

    pub struct ChrootInstaller<'a> {
        config: &'a Config,
        efi: bool,
    }

    impl<'a> ChrootInstaller<'a> {
        pub fn new(config: &'a Config) -> ChrootInstaller {
            let efi = Path::new("/sys/firmware/efi").exists();
            ChrootInstaller { config, efi }
        }

        fn get_uuid(&self) -> String {
            let mut buffer = String::new();
            let drive = self.config.drive().device_path();
            let partitions = Partitions::new(drive);

            let mut lsblk = Command::new("lsblk")
                .args(&["-dno", "UUID"])
                .arg(partitions.root().unwrap())
                .stdout(Stdio::piped())
                .spawn()
                .unwrap();

            lsblk
                .stdout
                .as_mut()
                .unwrap()
                .read_to_string(&mut buffer)
                .unwrap();

            lsblk.wait().unwrap();
            buffer.pop(); // remove newline
            buffer
        }

        fn grub_cmdline(&self, uuid: &str) -> String {
            if self.config.lvm().enabled() {
                panic!();
            }
            if self.config.drive().encryption() {
                let mapping = self.config.drive().crypt_mapping();

                format!(
                    "loglevel=3 quiet cryptdevice=UUID={}:{} root=/dev/mapper/{}",
                    uuid, mapping, mapping
                )
            } else {
                "loglevel=3 quiet".to_owned()
            }
            // if [[ $USE_LVM == true && $ENCRYPT_DISK == true ]]; then
            //   return
            // fi
            // if [[ $USE_LVM == true ]]; then
            //   # TODO
            //   return
            // fi
            // if [[ $ENCRYPT_DISK == true ]]; then
            //   # TODO
            //   return
            // fi
            // echo "loglevel=3 quiet"
            // cmdline
        }

        pub fn install_locales(&mut self) -> &mut Self {
            let mut buffer = String::new();
            let mut locale_gen = File::open("/etc/locale.gen").unwrap();
            locale_gen.read_to_string(&mut buffer).unwrap();
            buffer = buffer.replace("#en_US.UTF-8", "en_US.UTF-8");
            locale_gen = File::create("/etc/locale.gen").unwrap();
            locale_gen.write_all(buffer.as_bytes()).unwrap();

            Command::new("locale-gen").spawn().unwrap().wait().unwrap();

            File::create("/etc/locale.conf")
                .unwrap()
                .write_all("LANG=en_US.UTF-8\n".as_bytes())
                .unwrap();
            self
        }

        pub fn set_hostname(&mut self) -> &mut Self {
            File::create("/etc/hostname")
                .unwrap()
                .write_all(self.config.system().arch_host().as_bytes())
                .unwrap();
            self
        }

        pub fn set_timezone(&mut self) -> &mut Self {
            symlink(self.config.system().timezone(), "/etc/localtime").unwrap();

            Command::new("timedatectl")
                .args(&["set-ntp", "true"])
                .spawn()
                .unwrap()
                .wait()
                .unwrap();
            self
        }

        pub fn install_grub(&mut self) -> &mut Self {
            let mut grub = Command::new("grub-install");

            if self.efi {
                pacman_install(&["efibootmgr"]);
                let id = format!("--bootloader-id={}", self.config.drive().bootloader_id());

                grub.args(&[
                    "--target=x86_64-efi",
                    "--efi-directory=/efi",
                    "--boot-directory=/boot",
                    &id,
                ])
                .spawn()
                .unwrap()
                .wait()
                .unwrap();
            } else {
                grub.arg("--target=i386-pc")
                    .arg(self.config.drive().device_path())
                    .spawn()
                    .unwrap()
                    .wait()
                    .unwrap();
            }
            self
        }

        pub fn configure_grub(&mut self) -> &mut Self {
            // TODO: fix grub
            let uuid = self.get_uuid();
            let cmdline = self.grub_cmdline(&uuid);
            let mut buffer = String::new();

            File::open("/etc/default/grub")
                .unwrap()
                .read_to_string(&mut buffer)
                .unwrap();

            buffer = buffer.replace("loglevel=3 quiet", &cmdline);

            File::create("/etc/default/grub")
                .unwrap()
                .write_all(buffer.as_bytes())
                .unwrap();

            Command::new("grub-mkconfig")
                .args(&["-o", "/boot/grub/grub.cfg"])
                .spawn()
                .unwrap()
                .wait()
                .unwrap();
            self
        }

        pub fn set_mkinitcpio_hooks(&mut self) -> &mut Self {
            let mut buffer = String::new();

            File::open("/etc/mkinitcpio.conf")
                .unwrap()
                .read_to_string(&mut buffer)
                .unwrap();

            let hooks_source = vec![
                "base",
                "udev",
                "autodetect",
                "modconf",
                "block",
                "filesystems",
                "keyboard",
                "fsck",
            ];
            let mut hooks_target = Vec::from(&hooks_source[0..5]);
            hooks_target.push("keyboard");

            if self.config.drive().encryption() {
                hooks_target.push("encrypt");
            }
            if self.config.lvm().enabled() {
                // TODO: add lvm support
                //   persist pacman -S --noconfirm --needed lvm2
                hooks_target.push("lvm2");
            }
            hooks_target.push("filesystems");
            hooks_target.push("fsck");

            let format_hooks = |hooks: Vec<&str>| format!("HOOKS=({})", hooks.join(" "));
            let hooks_source = format_hooks(hooks_source);
            let hooks_target = format_hooks(hooks_target);
            buffer = buffer.replace(&hooks_source, &hooks_target);

            File::create("/etc/mkinitcpio.conf")
                .unwrap()
                .write_all(buffer.as_bytes())
                .unwrap();

            Command::new("mkinitcpio")
                .args(&["-p", "linux"])
                .spawn()
                .unwrap()
                .wait()
                .unwrap();
            self
        }

        pub fn add_user(&mut self) -> &mut Self {
            let user = self.config.system().arch_username();

            Command::new("useradd")
                .args(&["-m", user])
                .spawn()
                .unwrap()
                .wait()
                .unwrap();

            let mut password = self.config.system().arch_userpass().to_owned();

            if password.is_empty() {
                println!("Enter a password for the user \"{}\"", user);
                stdin().read_line(&mut password).unwrap();
            }
            let mut chpasswd = Command::new("chpasswd")
                .stdin(Stdio::piped())
                .spawn()
                .unwrap();

            let chpasswd_input = format!("{}:{}", user, password);
            let chpasswd_input = chpasswd_input.as_bytes();

            chpasswd
                .stdin
                .as_mut()
                .unwrap()
                .write_all(chpasswd_input)
                .unwrap();

            chpasswd.wait().unwrap();
            let gitconfig = format!("/home/{}/.gitconfig", user);

            fs::rename("/root/installer/gitconfig", &gitconfig).unwrap();

            Command::new("chown")
                .args(&[&format!("{}:", user), gitconfig.as_str()])
                .spawn()
                .unwrap()
                .wait()
                .unwrap();
            self
        }

        pub fn configure_packages(&mut self) -> &mut Self {
            PackageConfigurator::new(self.config).run();
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
    private::ChrootInstaller::new(config)
        .install_locales()
        .set_hostname()
        .set_timezone()
        .install_grub()
        .configure_grub()
        .set_mkinitcpio_hooks()
        .add_user()
        .configure_packages()
        .install_aur_helper()
        .install_aur_packages();
    Ok(())
}
