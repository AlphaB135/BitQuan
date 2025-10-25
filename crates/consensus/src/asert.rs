#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! ASERT difficulty retarget implementation (prototype).

use crate::ConsensusParams;

/// Maximum target corresponding to the easiest difficulty (Bitcoin-style).
const MAX_TARGET: u128 = u128::MAX;

/// Computes the next target using the ASERT algorithm.
///
/// - `anchor_target`: Difficulty target of the anchor block (compact-form decoded).
/// - `height_delta`: Height difference between the evaluated block and the anchor (may be negative for reorg).
/// - `time_delta`: Timestamp difference (seconds) between evaluated block and anchor.
pub fn asert_next_target(
    anchor_target: u128,
    height_delta: i64,
    time_delta: i64,
    params: &ConsensusParams,
) -> u128 {
    if anchor_target == 0 {
        return 1;
    }

    let block_time = params.target_block_time as f64;
    let half_life = params.difficulty_half_life as f64;

    let exponent = (time_delta as f64 - block_time * height_delta as f64) / half_life;
    let factor = 2_f64.powf(exponent);

    let mut next = (anchor_target as f64) * factor;
    if next < 1.0 {
        next = 1.0;
    }

    let next = next.round() as u128;
    next.min(MAX_TARGET)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params() -> ConsensusParams {
        ConsensusParams {
            block_weight_cap: 4_000_000,
            signature_weight_alpha: 384,
            target_block_time: 600,
            difficulty_half_life: 86_400,
            reward_schedule: crate::RewardSchedule::phase3_defaults(),
        }
    }

    #[test]
    fn unchanged_when_on_time() {
        let anchor = 1_000_000u128;
        let target = asert_next_target(anchor, 1, 600, &params());
        assert_eq!(target, anchor);
    }

    #[test]
    fn decreases_when_blocks_fast() {
        let anchor = 1_000_000u128;
        let target = asert_next_target(anchor, 1, 300, &params());
        assert!(target < anchor);
    }

    #[test]
    fn increases_when_blocks_slow() {
        let anchor = 1_000_000u128;
        let target = asert_next_target(anchor, 1, 1_200, &params());
        assert!(target > anchor);
    }

    #[test]
    fn clamps_to_max_target() {
        let anchor = MAX_TARGET;
        let target = asert_next_target(anchor, -10, 10_000, &params());
        assert_eq!(target, MAX_TARGET);
    }
}
