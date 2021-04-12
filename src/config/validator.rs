use crate::errors::ConfigInvalidError;

use {super::toml, crate::errors::ALIResult, std::rc::Rc};

pub struct ConfigValidator {
    toml: Rc<toml::Toml>,
}

impl ConfigValidator {
    pub fn new(toml: Rc<toml::Toml>) -> Self {
        ConfigValidator { toml }
    }

    pub fn validate(&self) -> ALIResult<()> {
        self.validate_drive()?;
        self.validate_system()?;
        Ok(())
    }

    pub fn validate_drive(&self) -> ALIResult<()> {
        if self.toml.drive.device().is_empty() {
            return Err(ConfigInvalidError {
                desc: "drive device not set".to_owned(),
            }
            .into());
        }
        if self.toml.drive.encryption() && self.toml.drive.password().is_empty() {
            return Err(ConfigInvalidError {
                desc: "drive password not set".to_owned(),
            }
            .into());
        }
        Ok(())
    }

    pub fn validate_system(&self) -> ALIResult<()> {
        if self.toml.system.arch_host().is_empty() {
            return Err(ConfigInvalidError {
                desc: "hostname not set".to_owned(),
            }
            .into());
        }
        if self.toml.system.arch_username().is_empty() {
            return Err(ConfigInvalidError {
                desc: "username not set".to_owned(),
            }
            .into());
        }
        if self.toml.system.arch_userpass().is_empty() {
            return Err(ConfigInvalidError {
                desc: "user password not set".to_owned(),
            }
            .into());
        }
        if self.toml.system.timezone().as_os_str().is_empty() {
            return Err(ConfigInvalidError {
                desc: "timezone not set".to_owned(),
            }
            .into());
        }
        Ok(())
    }
}
