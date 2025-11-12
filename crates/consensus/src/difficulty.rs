#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! Difficulty conversion utilities and ASERT-backed retarget state.

use crate::{asert_next_target, BurstGuardState, ConsensusParams, GuardContext};



/// Converts compact representation (`bits`) into a 64-bit integer target value.
/// Uses pure integer arithmetic to ensure deterministic behavior.
pub fn compact_to_target(bits: u32) -> u64 {
    if bits == 0 {
        return 0;
    }
    let exponent = (bits >> 24) as i32;
    let mantissa = (bits & 0x007f_ffff) as u64;
    
    // Handle negative exponents (very small targets)
    if exponent <= 3 {
        return mantissa >> (3 - exponent);
    }
    
    // For positive exponents, be careful about overflow
    let shift = exponent - 3;
    if shift >= 64 {
        return u64::MAX; // Target is effectively infinite
    }
    
    // Use checked multiplication to prevent overflow
    mantissa.checked_shl(shift as u32).unwrap_or(u64::MAX)
}

/// Converts a difficulty target to its compact representation (integer version).
///
/// Safety: Uses integer arithmetic for deterministic behavior.
pub fn target_to_compact_u64(target: u64) -> u32 {
    if target == 0 {
        return 0;
    }

    // Find the highest set bit to determine exponent
    let leading_zeros = target.leading_zeros();
    let bit_length = 64 - leading_zeros;
    
    if bit_length <= 24 {
        // Target fits in mantissa directly
        target as u32
    } else {
        // Need to shift right to fit in 24 bits
        let shift = bit_length - 24;
        let mantissa = (target >> shift) as u32;
        let exponent = (bit_length - 1) as u32;
        ((exponent << 24) | mantissa) & 0x7fffffff
    }
}

/// Tracks the anchor information for ASERT difficulty adjustments.
#[derive(Clone, Debug)]
pub struct DifficultyState {
    anchor_height: u64,
    anchor_timestamp: u64,
    anchor_target: u64,
    guard_state: BurstGuardState,
    guard_activation_height: u64,
}

impl DifficultyState {
    /// Creates a new difficulty state from the given anchor block parameters.
    pub fn new(
        anchor_height: u64,
        anchor_timestamp: u64,
        anchor_bits: u32,
        guard_activation_height: u64,
    ) -> Self {
        Self {
            anchor_height,
            anchor_timestamp,
            anchor_target: compact_to_target(anchor_bits) as u64,
            guard_state: BurstGuardState::default(),
            guard_activation_height,
        }
    }

    /// Returns the compact representation of the anchor target.
    pub fn anchor_bits(&self) -> u32 {
        target_to_compact_u64(self.anchor_target)
    }

    /// Computes the next target for the specified block height/timestamp and updates the anchor.
    pub fn update(
        &mut self,
        next_height: u64,
        next_timestamp: u64,
        params: &ConsensusParams,
    ) -> u32 {
        let height_delta = next_height as i64 - self.anchor_height as i64;
        let time_delta = next_timestamp as i64 - self.anchor_timestamp as i64;
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
        self.anchor_timestamp = next_timestamp;
        self.anchor_target = next_target;

        target_to_compact_u64(next_target)
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
            witness_weight_beta: 0.5,
            difficulty: DifficultyParams {
                target_block_time: 600,
                difficulty_half_life: 14_400,
                burst_guard_window: 11,
                burst_guard_floor_ratio_fp: 1417339207, // 0.33 in 32.32 fixed-point
                burst_guard_release_ratio_fp: 1632087572, // 0.38 in 32.32 fixed-point
                burst_guard_multiplier_fp: 6442450944, // 1.5 in 32.32 fixed-point
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
        let reconverted = target_to_compact_u64(target);
        assert!(reconverted > 0);
    }

    #[test]
    fn state_updates_anchor() {
        let mut state = DifficultyState::new(100, 1_000_000, 0x1d00ffff, 0);
        let next = state.update(101, 1_000_600, &params());
        assert!(next > 0);
        assert_eq!(state.anchor_height, 101);
        assert_eq!(state.anchor_timestamp, 1_000_600);
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
        let floor_ratio = params.difficulty.burst_guard_floor_ratio_fp as f64 / crate::asert::FP_SCALE as f64;
        let fast_delta =
            (expected * floor_ratio * 0.5).max(1.0) as u64;

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
        let floor_ratio = params.difficulty.burst_guard_floor_ratio_fp as f64 / crate::asert::FP_SCALE as f64;
        let fast_delta =
            (expected * floor_ratio * 0.5).max(1.0) as u64;

        let next_height = anchor_height + window;
        let next_time = anchor_time + fast_delta;
        let _ = state.update(next_height, next_time, &params);
        assert!(state.guard_state().is_active());
    }
}
