use std::{
    error::Error,
    fmt::{self, Display},
    path::PathBuf,
};

pub type ALIResult<T> = Result<T, ALIError>;

#[derive(Debug)]
pub enum ErrorKind {
    UnknownArchISOProfile,
    // TODO: set umask directly
    InvalidUmask { expected: u32, got: u32 },
    ConfigNotFound(PathBuf),
    SudoRequired,
    EarlyExit,
    Generic { message: String },
}

#[derive(Debug)]
pub struct ALIError {
    kind: ErrorKind,
}

impl ALIError {
    pub fn new(kind: ErrorKind) -> ALIError {
        ALIError { kind }
    }

    pub fn generic(message: String) -> ALIError {
        let kind = ErrorKind::Generic { message };
        ALIError { kind }
    }
}

impl Display for ALIError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let buf: String;

        let msg = match &self.kind {
            ErrorKind::UnknownArchISOProfile => {
                "Unknown profile, expected either \"releng\" or \"baseline\""
            }
            ErrorKind::ConfigNotFound(path) => {
                buf = format!(
                    "Configuration file \"{}\" not found",
                    path.to_string_lossy()
                );
                &buf
            }
            ErrorKind::SudoRequired => "This command must be executed from super user",
            ErrorKind::InvalidUmask { expected, got } => {
                buf = format!("Invalid umask, expected {}, got {}", expected, got);
                &buf
            }
            ErrorKind::EarlyExit => "Installation aborted",
            ErrorKind::Generic { message } => message,
        };
        write!(f, "{}", msg)
    }
}

impl Error for ALIError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}
