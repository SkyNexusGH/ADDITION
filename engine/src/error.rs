use std::fmt;

#[derive(thiserror::Error, Debug)]
pub enum EngineError {
    #[error("process not found: {0}")]
    ProcessNotFound(String),

    #[error("could not open process {pid}: {reason}")]
    OpenFailed { pid: u32, reason: String },

    #[error("memory read failed at {0}")]
    Read(Addr),

    #[error("memory write failed at {0}")]
    Write(Addr),

    #[error("module not loaded: {0}")]
    ModuleNotFound(String),

    #[error("byte pattern not found: {0}")]
    PatternNotFound(String),

    #[error("invalid input: {0}")]
    Invalid(String),

    #[error("anti-cheat detected ({0}). Trainers are disabled for this game")]
    AntiCheat(String),

    #[error("the game process has exited")]
    ProcessExited,

    #[error("cancelled")]
    Cancelled,

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    Other(String),
}

/// Displays an address as hex in error messages.
#[derive(Debug, Clone, Copy)]
pub struct Addr(pub u64);

impl fmt::Display for Addr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x{:X}", self.0)
    }
}

pub type Result<T> = std::result::Result<T, EngineError>;
