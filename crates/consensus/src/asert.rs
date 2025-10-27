#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! ASERT difficulty retarget implementation (prototype).

use crate::ConsensusParams;

/// Computes the next target using the ASERT algorithm (returns target as a float).
///
/// - `anchor_target`: Difficulty target of the anchor block.
/// - `height_delta`: Height difference between the evaluated block and the anchor.
/// - `time_delta`: Timestamp difference (seconds) between evaluated block and anchor.
pub fn asert_next_target(
    anchor_target: f64,
    height_delta: i64,
    time_delta: i64,
    params: &ConsensusParams,
) -> f64 {
    let block_time = params.target_block_time as f64;
    let half_life = params.difficulty_half_life as f64;

    let exponent = (time_delta as f64 - block_time * height_delta as f64) / half_life;
    let factor = 2_f64.powf(exponent);

    let mut next = anchor_target * factor;
    let max_target = 65_535.0 * 2f64.powi(208);
    if next < 1.0 {
        next = 1.0;
    }
    if next > max_target {
        next = max_target;
    }

    next
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ConsensusParams, RewardSchedule};

    fn params() -> ConsensusParams {
        ConsensusParams {
            block_weight_cap: 4_000_000,
            signature_weight_alpha: 384,
            witness_weight_beta: 0.5,
            target_block_time: 600,
            difficulty_half_life: 86_400,
            reward_schedule: RewardSchedule::phase3_defaults(),
        }
    }

    #[test]
    fn unchanged_when_on_time() {
        let anchor = 1000.0;
        let target = asert_next_target(anchor, 1, 600, &params());
        assert!((target - anchor).abs() < 1e-6);
    }

    #[test]
    fn decreases_when_blocks_fast() {
        let anchor = 1000.0;
        let target = asert_next_target(anchor, 1, 300, &params());
        assert!(target < anchor);
    }

    #[test]
    fn increases_when_blocks_slow() {
        let anchor = 1000.0;
        let target = asert_next_target(anchor, 1, 1_200, &params());
        assert!(target > anchor);
    }

    #[test]
    fn clamps_to_max_target() {
        let max_target = 65_535.0 * 2f64.powi(208);
        let anchor = max_target;
        let target = asert_next_target(anchor, -10, 10_000, &params());
        assert!(target <= max_target);
    }

    #[test]
    fn clamps_to_min_target() {
        let anchor = 1000.0;
        // Fast blocks for many height steps
        let target = asert_next_target(anchor, 100, 1_000, &params());
        assert!(target >= 1.0);
    }

    #[test]
    fn monotonic_with_increasing_time() {
        let anchor = 1000.0;
        let t1 = asert_next_target(anchor, 1, 300, &params());
        let t2 = asert_next_target(anchor, 1, 600, &params());
        let t3 = asert_next_target(anchor, 1, 900, &params());
        
        // Target should increase as time increases
        assert!(t1 < t2);
        assert!(t2 < t3);
    }

    #[test]
    fn deterministic_same_inputs() {
        let anchor = 1000.0;
        let t1 = asert_next_target(anchor, 5, 3_000, &params());
        let t2 = asert_next_target(anchor, 5, 3_000, &params());
        assert_eq!(t1, t2);
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    fn params() -> ConsensusParams {
        ConsensusParams {
            block_weight_cap: 4_000_000,
            signature_weight_alpha: 384,
            witness_weight_beta: 0.5,
            target_block_time: 600,
            difficulty_half_life: 86_400,
            reward_schedule: crate::RewardSchedule::phase3_defaults(),
        }
    }

    proptest! {
        #[test]
        fn asert_always_positive(
            height_delta in -1000i64..1000,
            time_delta in 0i64..100_000
        ) {
            let anchor = 1000.0;
            let target = asert_next_target(anchor, height_delta, time_delta, &params());
            prop_assert!(target > 0.0);
            prop_assert!(target.is_finite());
        }

        #[test]
        fn asert_bounded_by_max_target(
            height_delta in -1000i64..1000,
            time_delta in 0i64..1_000_000
        ) {
            let max_target = 65_535.0 * 2f64.powi(208);
            let anchor = max_target / 2.0;
            let target = asert_next_target(anchor, height_delta, time_delta, &params());
            prop_assert!(target <= max_target);
        }

        #[test]
        fn asert_monotonic_increasing_time(
            height_delta in 1i64..100,
            time_base in 1u64..10_000
        ) {
            let anchor = 1000.0;
            let t1 = asert_next_target(anchor, height_delta, time_base as i64, &params());
            let t2 = asert_next_target(anchor, height_delta, (time_base * 2) as i64, &params());
            
            // Longer time should give higher target (easier difficulty)
            prop_assert!(t2 >= t1);
        }

        #[test]
        fn asert_deterministic(
            height_delta in -100i64..100,
            time_delta in 0i64..50_000
        ) {
            let anchor = 1000.0;
            let t1 = asert_next_target(anchor, height_delta, time_delta, &params());
            let t2 = asert_next_target(anchor, height_delta, time_delta, &params());
            prop_assert_eq!(t1, t2);
        }
    }
}
