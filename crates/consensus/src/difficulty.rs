#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! Difficulty conversion utilities and ASERT-backed retarget state.

use crate::{asert_next_target, ConsensusParams};

const POW_256: f64 = 256.0;

/// Converts compact representation (`bits`) into a floating-point target value.
pub fn compact_to_target(bits: u32) -> f64 {
    if bits == 0 {
        return 0.0;
    }
    let exponent = (bits >> 24) as i32;
    let mantissa = (bits & 0x007f_ffff) as f64;
    mantissa * POW_256.powi(exponent - 3)
}

/// Converts a floating-point target value into the compact representation used in block headers.
pub fn target_to_compact(target: f64) -> u32 {
    if target <= 0.0 {
        return 0;
    }

    let mut exponent = ((target.log(POW_256)).ceil() as i32) + 3;
    if exponent < 0 {
        exponent = 0;
    }

    let pow = POW_256.powi(exponent - 3);
    let mut mantissa = (target / pow).round();

    if mantissa >= 0x0080_0000 as f64 {
        mantissa /= 256.0;
        exponent += 1;
    }

    ((exponent as u32) << 24) | (mantissa as u32 & 0x007f_ffff)
}

/// Tracks the anchor information for ASERT difficulty adjustments.
#[derive(Clone, Debug)]
pub struct DifficultyState {
    anchor_height: u64,
    anchor_timestamp: u64,
    anchor_target: f64,
}

impl DifficultyState {
    /// Creates a new difficulty state from the given anchor block parameters.
    pub fn new(anchor_height: u64, anchor_timestamp: u64, anchor_bits: u32) -> Self {
        Self {
            anchor_height,
            anchor_timestamp,
            anchor_target: compact_to_target(anchor_bits),
        }
    }

    /// Returns the compact representation of the anchor target.
    pub fn anchor_bits(&self) -> u32 {
        target_to_compact(self.anchor_target)
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
        let next_target = asert_next_target(self.anchor_target, height_delta, time_delta, params);

        self.anchor_height = next_height;
        self.anchor_timestamp = next_timestamp;
        self.anchor_target = next_target;

        target_to_compact(next_target)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ConsensusParams, RewardSchedule};

    fn params() -> ConsensusParams {
        ConsensusParams {
            block_weight_cap: 4_000_000,
            signature_weight_alpha: 384,
            target_block_time: 600,
            difficulty_half_life: 86_400,
            reward_schedule: RewardSchedule::phase3_defaults(),
        }
    }

    #[test]
    fn conversion_round_trip_reasonable() {
        let bits = 0x1d00ffff;
        let target = compact_to_target(bits);
        let reconverted = target_to_compact(target);
        assert!(reconverted > 0);
    }

    #[test]
    fn state_updates_anchor() {
        let mut state = DifficultyState::new(100, 1_000_000, 0x1d00ffff);
        let next = state.update(101, 1_000_600, &params());
        assert!(next > 0);
        assert_eq!(state.anchor_height, 101);
        assert_eq!(state.anchor_timestamp, 1_000_600);
    }
}
