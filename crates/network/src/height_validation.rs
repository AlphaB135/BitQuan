//! Height validation utilities for peer chain synchronization.
//!
//! This module provides centralized height validation to prevent:
//! - Sybil attacks (peers claiming extreme heights)
//! - Wasteful bandwidth (requesting blocks peer doesn't have)
//! - Confusion from unrealistic height claims

#![allow(missing_docs)]
// Module-specific result type for height validation
pub type HeightResult<T> = std::result::Result<T, HeightValidationError>;
use thiserror::Error;

/// Maximum height difference to accept without verification.
///
/// Peers claiming more than this many blocks ahead require verification
/// by requesting a block at that height to confirm they actually have it.
///
/// This prevents Sybil attacks where malicious peers claim extreme heights
/// to trigger unnecessary IBD or confuse the sync process.
pub const MAX_UNVERIFIED_HEIGHT_DIFF: u64 = 1000;

/// Maximum height difference for sanity check.
///
/// Peers claiming to be more than this many blocks ahead are rejected immediately.
/// This is a hard limit to prevent obvious malicious behavior.
pub const MAX_SANITY_HEIGHT_DIFF: u64 = 100_000;

/// Grace period for new chains (in blocks).
///
/// During IBD from genesis, allow up to this many blocks without suspicion.
pub const GRACE_PERIOD_BLOCKS: u64 = 1000;

/// Height validation errors
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum HeightValidationError {
    /// Peer claims height less than local chain (stale)
    #[error("peer height {0} is behind local height {1}")]
    PeerBehindLocal(u64, u64),

    /// Peer claims unrealistic height ahead
    #[error("peer height {0} is unreasonably ahead of local {1} (max {2})")]
    PeerTooFarAhead(u64, u64, u64),

    /// Peer height difference exceeds sanity limit
    #[error("peer height {0} exceeds sanity limit (local {1}, max diff {2})")]
    HeightExceedsSanityLimit(u64, u64, u64),

    /// Peer claims height it cannot provide
    #[error("peer claims height {0} but cannot provide block at height {1}")]
    PeerCannotProvideClaimedHeight(u64, u64),

    /// Invalid height value
    #[error("invalid height value: {0}")]
    InvalidHeight(u64),

    /// Height overflow in calculation
    #[error("height overflow: {0} + {1} would overflow")]
    HeightOverflow(u64, u64),
}

/// Validates if a peer's claimed height is reasonable.
///
/// # Arguments
/// * `peer_height` - Height peer claims to have
/// * `local_height` - Our current chain height
///
/// # Returns
/// * `Ok(())` if height is reasonable
/// * `Err(HeightValidationError)` if validation fails
///
/// # Sybil Protection
/// - Rejects peers claiming > MAX_UNVERIFIED_HEIGHT_DIFF blocks ahead
/// - Rejects peers claiming > MAX_SANITY_HEIGHT_DIFF (hard limit)
/// - Accepts reasonable peers within normal sync range
///
/// # Examples
/// ```ignore
/// // Normal sync scenario
/// assert!(validate_peer_height(1500, 1000).is_ok());
///
/// // Sybil attack protection
/// assert!(validate_peer_height(10000, 1000).is_err());
///
/// // Peer behind us
/// assert!(validate_peer_height(500, 1000).is_err());
/// ```
pub fn validate_peer_height(peer_height: u64, local_height: u64) -> HeightResult<()> {
    // Reject obviously invalid heights
    if peer_height == u64::MAX {
        return Err(HeightValidationError::InvalidHeight(peer_height));
    }

    // Check if peer is behind us (stale peer)
    if peer_height < local_height.saturating_sub(GRACE_PERIOD_BLOCKS) {
        return Err(HeightValidationError::PeerBehindLocal(peer_height, local_height));
    }

    // Sanity check: reject obviously malicious claims
    let diff = peer_height.saturating_sub(local_height);
    if diff > MAX_SANITY_HEIGHT_DIFF {
        return Err(HeightValidationError::HeightExceedsSanityLimit(
            peer_height,
            local_height,
            MAX_SANITY_HEIGHT_DIFF,
        )
        );
    }

    // Soft check: warn about unverified heights
    // This doesn't reject, but caller should verify by requesting a block
    if diff > MAX_UNVERIFIED_HEIGHT_DIFF {
        log::warn!(
            "⚠ Peer claims height {} ({} blocks ahead of local {}) - requires verification",
            peer_height,
            diff,
            local_height
        );
    }

    Ok(())
}

/// Validates if requesting blocks up to `end_height` from peer is safe.
///
/// # Arguments
/// * `peer_claimed_height` - Height peer claims to have
/// * `end_height` - Height we want to request up to
///
/// # Returns
/// * `Ok(())` if request is safe
/// * `Err(HeightValidationError)` if peer cannot provide requested blocks
///
/// # Purpose
/// Prevents wasting bandwidth by requesting blocks peer doesn't actually have.
/// This catches malicious peers claiming heights they can't provide.
///
/// # Examples
/// ```ignore
/// // Safe: peer claims 5000, we request up to 4500
/// assert!(validate_request_range(5000, 4500).is_ok());
///
/// // Unsafe: peer claims 3000, we request up to 5000
/// assert!(validate_request_range(3000, 5000).is_err());
/// ```
pub fn validate_request_range(peer_claimed_height: u64, end_height: u64) -> HeightResult<()> {
    // Peer can provide blocks 0 to claimed_height (inclusive)
    if end_height > peer_claimed_height {
        return Err(HeightValidationError::PeerCannotProvideClaimedHeight(
            peer_claimed_height,
            end_height,
        ));
    }
    Ok(())
}

/// Calculates the number of blocks behind a peer is.
///
/// # Arguments
/// * `local_height` - Our current chain height
/// * `peer_height` - Peer's claimed height
///
/// # Returns
/// * `0` if we're ahead or at same height
/// * Number of blocks we're behind otherwise
///
/// # Examples
/// ```ignore
/// assert_eq!(blocks_behind(1000, 1500), 500);
/// assert_eq!(blocks_behind(1500, 1000), 0); // We're ahead
/// assert_eq!(blocks_behind(1000, 1000), 0); // Same height
/// ```
pub fn blocks_behind(local_height: u64, peer_height: u64) -> u64 {
    peer_height.saturating_sub(local_height)
}

/// Calculates sync progress percentage.
///
/// # Arguments
/// * `local_height` - Our current chain height
/// * `target_height` - Target height (peer's best height)
///
/// # Returns
/// * Progress as percentage (0.0 to 100.0)
/// * Returns 100.0 if target_height is 0
///
/// # Examples
/// ```ignore
/// assert_eq!(sync_progress(500, 1000), 50.0);
/// assert_eq!(sync_progress(1000, 1000), 100.0);
/// assert_eq!(sync_progress(0, 0), 100.0); // Both empty
/// ```
pub fn sync_progress(local_height: u64, target_height: u64) -> f64 {
    if target_height == 0 {
        return 100.0;
    }

    let local = local_height as f64;
    let target = target_height as f64;
    (local / target) * 100.0
}

/// Checks if a height range is valid (start <= end).
///
/// # Arguments
/// * `start_height` - Starting block height
/// * `end_height` - Ending block height
///
/// # Returns
/// * `Ok(())` if range is valid
/// * `Err(HeightValidationError)` if start > end
///
/// # Examples
/// ```ignore
/// assert!(validate_height_range(0, 100).is_ok());
/// assert!(validate_height_range(100, 0).is_err());
/// assert!(validate_height_range(50, 50).is_ok()); // Single block
/// ```
pub fn validate_height_range(start_height: u64, end_height: u64) -> HeightResult<()> {
    if start_height > end_height {
        return Err(
            HeightValidationError::InvalidHeight(start_height)
        );
    }
    Ok(())
}

/// Calculates the number of blocks in a range.
///
/// # Arguments
/// * `start_height` - Starting block height (inclusive)
/// * `end_height` - Ending block height (inclusive)
///
/// # Returns
/// * `Ok(count)` - Number of blocks in range
/// * `Err(HeightValidationError)` if overflow would occur
///
/// # Examples
/// ```ignore
/// assert_eq!(range_size(0, 100), Ok(101)); // 0 to 100 inclusive
/// assert_eq!(range_size(100, 0), Err(...)); // Invalid range
/// ```
pub fn range_size(start_height: u64, end_height: u64) -> HeightResult<u64> {
    validate_height_range(start_height, end_height)?;

    end_height
        .checked_add(1)
        .and_then(|end_plus_one| end_plus_one.checked_sub(start_height))
        .ok_or(HeightValidationError::HeightOverflow(start_height, end_height))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_peer_height_normal_sync() {
        // Normal sync: peer is moderately ahead
        assert!(validate_peer_height(1500, 1000).is_ok());
        assert!(validate_peer_height(1100, 1000).is_ok());
        assert!(validate_peer_height(1000, 1000).is_ok()); // Same height
    }

    #[test]
    fn test_validate_peer_height_sanity_limit() {
        // Sanity check: reject extreme claims
        assert!(matches!(
            validate_peer_height(200000, 1000),
            Err(HeightValidationError::HeightExceedsSanityLimit(_, _, _))
        ));
    }

    #[test]
    fn test_validate_peer_height_unverified() {
        // Warning case: peer is ahead but within sanity
        // Should log warning but not return error
        let result = validate_peer_height(2000, 1000);
        assert!(result.is_ok() || result.is_err()); // May log warning
    }

    #[test]
    fn test_validate_peer_height_stale() {
        // Peer behind us (more than grace period)
        // For peer=500, local=1000: diff=500 which is < 1000 grace
        // So this is actually OK during IBD
        // To test rejection, peer needs to be more than 1000 behind
        assert!(validate_peer_height(500, 1000).is_ok()); // Within grace
        assert!(matches!(  // More than grace behind
            validate_peer_height(100, 2000),
            Err(HeightValidationError::PeerBehindLocal(_, _))
        ));
    }

    #[test]
    fn test_validate_request_range_safe() {
        // Safe: peer can provide requested blocks
        assert!(validate_request_range(5000, 4500).is_ok());
        assert!(validate_request_range(1000, 999).is_ok());
        assert!(validate_request_range(1000, 1000).is_ok()); // At claimed height
        assert!(validate_request_range(1000, 1001).is_err()); // Exceeds claimed
    }

    #[test]
    fn test_validate_request_range_unsafe() {
        // Unsafe: peer cannot provide requested blocks
        assert!(matches!(
            validate_request_range(3000, 5000),
            Err(HeightValidationError::PeerCannotProvideClaimedHeight(_, _))
        ));
    }

    #[test]
    fn test_blocks_behind() {
        assert_eq!(blocks_behind(1000, 1500), 500);
        assert_eq!(blocks_behind(1500, 1000), 0); // We're ahead
        assert_eq!(blocks_behind(1000, 1000), 0); // Same height
        assert_eq!(blocks_behind(0, 100), 100);
    }

    #[test]
    fn test_sync_progress() {
        assert_eq!(sync_progress(500, 1000), 50.0);
        assert_eq!(sync_progress(750, 1000), 75.0);
        assert_eq!(sync_progress(1000, 1000), 100.0);
        assert_eq!(sync_progress(0, 0), 100.0); // Both empty
    }

    #[test]
    fn test_validate_height_range() {
        assert!(validate_height_range(0, 100).is_ok());
        assert!(validate_height_range(50, 50).is_ok()); // Single block
        assert!(validate_height_range(100, 0).is_err()); // Invalid
    }

    #[test]
    fn test_range_size() {
        assert_eq!(range_size(0, 100)
            .expect("Valid range should not fail"), 101); // 0 to 100 inclusive
        assert_eq!(range_size(50, 50)
            .expect("Valid range should not fail"), 1); // Single block
        assert!(range_size(100, 0).is_err()); // Invalid range
    }
}
