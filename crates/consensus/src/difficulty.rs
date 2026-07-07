#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! Difficulty conversion utilities and ASERT-backed retarget state.

use crate::{
    asert_next_target, pow::compact_to_target_bytes, BurstGuardState, ConsensusParams, GuardContext,
};

/// Converts a 32-byte big-endian target back to compact `bits` form.
///
/// Follows Bitcoin's compact encoding: size byte (bits 24-31) + 23-bit
/// mantissa (bits 0-22). The mantissa is the most significant 3 bytes
/// of the target (excluding leading zeros), with the sign bit cleared.
pub fn target_to_compact(target: &[u8; 32]) -> u32 {
    // Find first non-zero byte
    let mut size = 32usize;
    for (i, &b) in target.iter().enumerate() {
        if b != 0 {
            size = 32 - i;
            break;
        }
    }

    if size == 0 {
        return 0;
    }
    if size > 32 {
        return 0;
    }

    // Extract top 3 bytes as mantissa
    let start = 32 - size;
    let mantissa = if size >= 3 {
        ((target[start] as u32) << 16)
            | ((target[start + 1] as u32) << 8)
            | (target[start + 2] as u32)
    } else {
        let mut m = 0u32;
        for &b in &target[start..] {
            m = (m << 8) | (b as u32);
        }
        m <<= (3 - size as u32) * 8;
        m
    };

    // Clear sign bit (bit 23 of mantissa)
    let compact = mantissa & 0x007f_ffff;

    compact | ((size as u32) << 24)
}

/// Backward-compatible alias: converts u64 target to compact form.
///
/// Note: This loses precision for targets exceeding u64 range.
/// Prefer `target_to_compact` with [u8; 32] for full 256-bit support.
pub fn target_to_compact_u64(target: u64) -> u32 {
    if target == 0 {
        return 0;
    }

    let mut size = (64 - target.leading_zeros()).div_ceil(8);
    let mut compact = if size <= 3 {
        (target << (8 * (3 - size))) as u32
    } else {
        (target >> (8 * (size - 3))) as u32
    };

    if compact & 0x00800000 != 0 {
        compact >>= 8;
        size += 1;
    }

    compact | (size << 24)
}

/// Converts compact `bits` to a 32-byte big-endian target.
///
/// Wrapper around `compact_to_target_bytes` from pow module.
/// Returns [0; 32] on invalid bits (instead of error).
pub fn compact_to_target(bits: u32) -> [u8; 32] {
    compact_to_target_bytes(bits).unwrap_or([0u8; 32])
}

/// Difficulty adjustment state for ASERT algorithm.
#[derive(Clone, Debug)]
pub struct DifficultyState {
    /// Height of the anchor block.
    pub anchor_height: u64,
    /// Timestamp of the anchor block.
    pub anchor_time: u64,
    /// Target (bits) of the anchor block.
    pub anchor_bits: u32,
    /// Target ([u8; 32]) of the anchor block.
    pub anchor_target: [u8; 32],
    guard_state: BurstGuardState,
    guard_activation_height: u64,
}

impl DifficultyState {
    /// Creates a new difficulty state from the given anchor block parameters.
    pub fn new(
        anchor_height: u64,
        anchor_time: u64,
        anchor_bits: u32,
        guard_activation_height: u64,
    ) -> Self {
        Self {
            anchor_height,
            anchor_time,
            anchor_bits,
            anchor_target: compact_to_target(anchor_bits),
            guard_state: BurstGuardState::default(),
            guard_activation_height,
        }
    }

    /// Returns the compact representation of the anchor target.
    pub fn anchor_bits(&self) -> u32 {
        target_to_compact(&self.anchor_target)
    }

    /// Computes the expected compact target for `next_height`/`next_timestamp` **without**
    /// modifying the anchor state.
    ///
    /// Use this for validation — callers that need to verify an incoming block's `bits`
    /// against the ASERT-computed target should call this instead of `update()`, which
    /// would corrupt the difficulty state by advancing the anchor prematurely.
    pub fn peek_next_bits(&self, next_height: u64, next_timestamp: u64, params: &ConsensusParams) -> u32 {
        let height_delta = next_height as i64 - self.anchor_height as i64;
        let time_delta = next_timestamp as i64 - self.anchor_time as i64;
        // Pass `None` for the burst guard so we do not mutate guard state.
        // The guard is for mining-path retargets; validation uses the raw ASERT output.
        let next_target = asert_next_target(
            self.anchor_target,
            height_delta,
            time_delta,
            params,
            None,
        );
        target_to_compact_u64(next_target)
    }

    /// Computes the next target for the specified block height/timestamp and updates the anchor.
    pub fn update(
        &mut self,
        next_height: u64,
        next_timestamp: u64,
        params: &ConsensusParams,
    ) -> u32 {
        let height_delta = next_height as i64 - self.anchor_height as i64;
        let time_delta = next_timestamp as i64 - self.anchor_time as i64;
        let next_target = asert_next_target(
            self.anchor_target,
            height_delta,
            time_delta,
            params,
            Some(GuardContext {
                state: &mut self.guard_state,
                current_height: next_height,
                activation_height: self.guard_activation_height,
            }),
        );

        self.anchor_height = next_height;
        self.anchor_time = next_timestamp;
        self.anchor_target = next_target;

        target_to_compact(&next_target)
    }

    #[cfg(test)]
    pub(crate) fn guard_state(&self) -> &BurstGuardState {
        &self.guard_state
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ConsensusParams, DifficultyParams, RewardSchedule};

    fn params() -> ConsensusParams {
        ConsensusParams {
            block_weight_cap: 4_000_000,
            signature_weight_alpha: 384,
            difficulty: DifficultyParams {
                target_block_time: 120, // Updated for Phase 4
                difficulty_half_life: 14_400,
                burst_guard_window: 11,
                burst_guard_floor_ratio_fp: 1417339207, // 0.33 in 32.32 fixed-point
                burst_guard_release_ratio_fp: 1632087572, // 0.38 in 32.32 fixed-point
                burst_guard_multiplier_fp: 6442450944,  // 1.5 in 32.32 fixed-point
                burst_guard_cooldown_blocks: 5,
                burst_guard_activation_height: 0,
            },
            reward_schedule: RewardSchedule::phase3_defaults(),
            pow_set: crate::PowSetParams::mainnet(),
        }
    }

    #[test]
    fn conversion_round_trip_reasonable() {
        let bits = 0x1d00ffff;
        let target = compact_to_target(bits);
        let reconverted = target_to_compact(&target);
        assert!(reconverted > 0);
    }

    #[test]
    fn state_updates_anchor() {
        let mut state = DifficultyState::new(100, 1_000_000, 0x1d00ffff, 0);
        let next = state.update(101, 1_000_600, &params());
        assert!(next > 0);
        assert_eq!(state.anchor_height, 101);
        assert_eq!(state.anchor_time, 1_000_600);
    }

    #[test]
    fn guard_inactive_before_activation_height() {
        let mut params = params();
        params.difficulty.burst_guard_activation_height = 200;
        let anchor_height = 150u64;
        let anchor_time = 1_000_000u64;
        let anchor_bits = 0x1d00ffff;
        let mut state = DifficultyState::new(
            anchor_height,
            anchor_time,
            anchor_bits,
            params.difficulty.burst_guard_activation_height,
        );

        // Fast window but below activation height
        let window = params.difficulty.burst_guard_window;
        let expected = (params.difficulty.target_block_time * window) as f64;
        let floor_ratio =
            params.difficulty.burst_guard_floor_ratio_fp as f64 / crate::asert::FP_SCALE as f64;
        let fast_delta = (expected * floor_ratio * 0.5).max(1.0) as u64;

        let next_height = anchor_height + window;
        let next_time = anchor_time + fast_delta;
        let _ = state.update(next_height, next_time, &params);
        assert!(state.guard_state().last_trigger_height().is_none());
    }

    #[test]
    fn guard_active_at_activation_height() {
        let mut params = params();
        params.difficulty.burst_guard_activation_height = 50;
        let anchor_height = 50u64;
        let anchor_time = 2_000_000u64;
        let anchor_bits = 0x1d00ffff;
        let mut state = DifficultyState::new(
            anchor_height,
            anchor_time,
            anchor_bits,
            params.difficulty.burst_guard_activation_height,
        );

        let window = params.difficulty.burst_guard_window;
        let expected = (params.difficulty.target_block_time * window) as f64;
        let floor_ratio =
            params.difficulty.burst_guard_floor_ratio_fp as f64 / crate::asert::FP_SCALE as f64;
        let fast_delta = (expected * floor_ratio * 0.5).max(1.0) as u64;

        let next_height = anchor_height + window;
        let next_time = anchor_time + fast_delta;
        let _ = state.update(next_height, next_time, &params);
        assert!(state.guard_state().is_active());
    }

    #[test]
    fn sign_bit_handling_increases_size() {
        // Test that when the sign bit is set, size is incremented
        // This is the CORRECT Bitcoin compact format behavior
        let target = 0x00ff_ffff_0000u64; // Will set sign bit after shift
        let compact = target_to_compact_u64(target);

        // Verify size byte was incremented (bits 24-31)
        let size = compact >> 24;
        assert!(
            size > 3,
            "Size should be incremented when sign bit is set, got size={}",
            size
        );

        // Verify mantissa is valid (bits 0-23)
        assert_eq!(
            compact & 0x007fffff,
            0x0000ffff,
            "Mantissa should be preserved"
        );
    }

    #[test]
    fn target_to_compact_zero_edge_case() {
        // Test that zero target is handled correctly
        let compact = target_to_compact_u64(0);
        assert_eq!(compact, 0, "Zero target should produce zero compact");
    }

    #[test]
    fn compact_to_target_reconstructs_mantissa() {
        // 0x1d7fffff: exponent=29, mantissa=0x7fffff (max 23-bit mantissa)
        let bits = 0x1d7fffff;
        let target = compact_to_target(bits);

        // Round-trip: compact -> target -> compact should preserve bits
        let roundtrip = target_to_compact(&target);
        assert_eq!(roundtrip, bits, "Round-trip should preserve compact bits");

        // Verify mantissa bytes at correct position
        // exponent=29, byte_pos=26, start=32-26-3=3
        assert_eq!(target[3], 0x7f);
        assert_eq!(target[4], 0xff);
        assert_eq!(target[5], 0xff);
    }
}
