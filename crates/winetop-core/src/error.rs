use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("process not found: {0}")]
    ProcessNotFound(u32),

    #[error("session not found: {0}")]
    SessionNotFound(String),

    #[error("kill failed for pid {pid}: {message}")]
    KillFailed { pid: u32, message: String },

    #[error("wineserver -k failed for prefix {prefix}: {message}")]
    WineServerKill { prefix: String, message: String },

    #[error("unsupported platform for process discovery")]
    UnsupportedPlatform,

    #[error("{0}")]
    Other(String),
}
