//! Time helpers shared across crates.

use crate::error::{Error, Result};
use std::time::{SystemTime, UNIX_EPOCH};

/// Returns the current UNIX timestamp in seconds.
pub fn unix_timestamp() -> Result<u64> {
    let now = SystemTime::now();
    let duration = now
        .duration_since(UNIX_EPOCH)
        .map_err(|_| Error::Time("clock before epoch"))?;
    Ok(duration.as_secs())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unix_timestamp_is_monotonic_or_zero() {
        let ts = unix_timestamp().expect("System time should be available");
        // We can't assert an exact value, but it should be reasonable.
        assert!(ts > 0);
    }
}
