use std::{io, path::PathBuf};

use snafu::Snafu;

pub type ALIResult<T> = std::result::Result<T, Error>;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum Error {
    #[snafu(display(
        "Unknown profile, expected either \"releng\" or \"baseline\", got: {:?}",
        profile
    ))]
    UnknownArchISOProfile { profile: String },

    #[snafu(display("Configuration file {:?} not found", path))]
    ConfigNotFound { path: String },

    #[snafu(display("Unable to open configuration file"))]
    ConfigOpen { source: io::Error },

    #[snafu(display("Unable to read configuration file"))]
    ConfigRead { source: io::Error },

    #[snafu(display("Configuration invalid: {:?}", desc))]
    ConfigInvalid { desc: String },

    #[snafu(display("Error: network access required"))]
    Network,

    #[snafu(display("Error: command must be executed from super user"))]
    SudoRequired,

    // TODO: set umask directly
    #[snafu(display("Invalid umask, expected {}, got {}", expected, got))]
    InvalidUmask { expected: u32, got: u32 },

    #[snafu(display("Device {:?} not found", device))]
    DeviceNotFound { device: PathBuf },

    #[snafu(display("Command {:?} failed", message))]
    CommandExecution { message: String },
}
