//! Shared error type for BitQuan crates.

use thiserror::Error;

/// Result alias using the shared [`enum@Error`] type.
pub type Result<T> = std::result::Result<T, Error>;

/// Unified error enumeration for workspace crates.
#[derive(Debug, Error)]
pub enum Error {
    /// Filesystem or IO failure.
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    /// HTTP protocol error (header/response construction).
    #[error("http: {0}")]
    Http(#[from] http::Error),
    /// Serialization/deserialization error.
    #[error("serde: {0}")]
    Serde(#[from] serde_json::Error),
    /// System time issues (e.g. clock before UNIX epoch).
    #[error("time: {0}")]
    Time(&'static str),
    /// Arithmetic overflow/underflow.
    #[error("overflow: {0}")]
    Overflow(&'static str),
    /// Invalid input/data detected.
    #[error("invalid: {0}")]
    Invalid(String),
    /// Missing resource or lookup failure.
    #[error("not found: {0}")]
    NotFound(String),
    /// Networking error (connection / protocol level).
    #[error("net: {0}")]
    Net(String),
    /// Operation timed out.
    #[error("timeout: {0}")]
    Timeout(String),
    /// Request was rate limited.
    #[error("rate limited")]
    RateLimited,
    /// Fatal unrecoverable error (replaces panic! in production).
    #[error("fatal: {0}")]
    Fatal(&'static str),
    /// Internal error with dynamic message.
    #[error("internal: {0}")]
    Internal(String),
}

impl From<crate::ValidationError> for Error {
    fn from(err: crate::ValidationError) -> Self {
        Error::Invalid(err.to_string())
    }
}
