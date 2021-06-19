use snafu::ensure;

use crate::errors;

use {super::toml, crate::errors::ALIResult, std::rc::Rc};

pub(crate) struct ConfigValidator {
    toml: Rc<toml::Toml>,
}

impl ConfigValidator {
    pub(crate) fn new(toml: Rc<toml::Toml>) -> Self {
        ConfigValidator { toml }
    }

    pub(crate) fn validate(&self) -> ALIResult<()> {
        self.validate_drive()?;
        self.validate_system()?;
        Ok(())
    }

    pub(crate) fn validate_drive(&self) -> ALIResult<()> {
        ensure!(
            !self.toml.system.drive.is_empty(),
            errors::ConfigInvalidSnafu {
                desc: "drive device not set".to_owned()
            }
        );
        if self.toml.partitions.root.encryption {
            ensure!(
                !self.toml.partitions.root.password.is_empty(),
                errors::ConfigInvalidSnafu {
                    desc: "drive password not set".to_owned(),
                }
            );
        }
        Ok(())
    }

    pub(crate) fn validate_system(&self) -> ALIResult<()> {
        ensure!(
            !self.toml.system.arch_host().is_empty(),
            errors::ConfigInvalidSnafu {
                desc: "hostname not set".to_owned(),
            }
        );
        ensure!(
            !self.toml.system.arch_username().is_empty(),
            errors::ConfigInvalidSnafu {
                desc: "username not set".to_owned(),
            }
        );
        ensure!(
            !self.toml.system.arch_userpass().is_empty(),
            errors::ConfigInvalidSnafu {
                desc: "user password not set".to_owned(),
            }
        );
        ensure!(
            !self.toml.system.timezone().as_os_str().is_empty(),
            errors::ConfigInvalidSnafu {
                desc: "timezone not set".to_owned(),
            }
        );
        Ok(())
    }
}
