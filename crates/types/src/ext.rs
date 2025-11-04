//! Helper extensions to simplify error handling.

use crate::error::{Error, Result};

/// Extension trait to enrich errors with static context.
pub trait ResultExt<T> {
    /// Maps any error into [`Error::Invalid`] with the supplied message.
    fn ctx(self, msg: &'static str) -> Result<T>;
}

impl<T, E: std::fmt::Debug> ResultExt<T> for std::result::Result<T, E> {
    #[inline]
    fn ctx(self, msg: &'static str) -> Result<T> {
        self.map_err(|_| Error::Invalid(msg.to_string()))
    }
}

/// Extension trait to convert Option to Result with context.
pub trait OptionExt<T> {
    /// Converts None into [`Error::NotFound`] with the supplied message.
    fn ctx(self, msg: &'static str) -> Result<T>;
}

impl<T> OptionExt<T> for Option<T> {
    #[inline]
    fn ctx(self, msg: &'static str) -> Result<T> {
        self.ok_or_else(|| Error::NotFound(msg.to_string()))
    }
}

/// Checked arithmetic helper returning [`Error::Overflow`] on failure.
#[macro_export]
macro_rules! checked {
    ($expr:expr, $msg:expr) => {
        $expr.ok_or($crate::error::Error::Overflow($msg))
    };
}
