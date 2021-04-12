use std::fmt;

pub type ALIResult<T> = anyhow::Result<T>;

#[derive(Debug)]
pub struct UnknownArchISOProfileError {
    pub profile: String,
}

impl fmt::Display for UnknownArchISOProfileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Unknown profile, expected either \"releng\" or \"baseline\", got: \"{}\"",
            self.profile
        )
    }
}

impl std::error::Error for UnknownArchISOProfileError {}

#[derive(Debug)]
pub struct ConfigNotFoundError {
    pub path: String,
}

impl fmt::Display for ConfigNotFoundError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Configuration file \"{}\" not found", self.path)
    }
}

impl std::error::Error for ConfigNotFoundError {}

#[derive(Debug)]
pub struct ConfigInvalidError {
    pub desc: String,
}

impl fmt::Display for ConfigInvalidError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Configuration invalid: {}", self.desc)
    }
}

impl std::error::Error for ConfigInvalidError {}

#[derive(Debug)]
pub struct NetworkError;

impl fmt::Display for NetworkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Error: network access required")
    }
}

impl std::error::Error for NetworkError {}

#[derive(Debug)]
pub struct SudoRequiredError;

impl fmt::Display for SudoRequiredError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Error: command must be executed from super user")
    }
}

impl std::error::Error for SudoRequiredError {}

// TODO: set umask directly
#[derive(Debug)]
pub struct InvalidUmaskError {
    pub expected: u32,
    pub got: u32,
}

impl fmt::Display for InvalidUmaskError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Invalid umask, expected {}, got {}",
            self.expected, self.got
        )
    }
}

impl std::error::Error for InvalidUmaskError {}

#[derive(Debug)]
pub struct DeviceNotFoundError {
    pub device: String,
}

impl fmt::Display for DeviceNotFoundError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Device \"{}\" not found", self.device)
    }
}

impl std::error::Error for DeviceNotFoundError {}

#[derive(Debug)]
pub struct CommandExecutionError(pub String);

impl fmt::Display for CommandExecutionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Command \"{}\" failed", self.0)
    }
}

impl std::error::Error for CommandExecutionError {}
