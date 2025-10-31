#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! ASERT difficulty retarget implementation (prototype).

use crate::{compact_to_target, pow::DEVNET_MAX_BITS, ConsensusParams};

const MAX_EXPONENT_MAG: f64 = 1023.0;

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
    let half_life = params.difficulty_half_life as f64;
    let min_target = 1.0;
    let max_target = compact_to_target(DEVNET_MAX_BITS);

    let anchor = clamp_target(anchor_target, min_target, max_target);
    let expected_time = expected_window_time(height_delta, params.target_block_time);
    let time_delta_f = saturating_i64_to_f64(time_delta);

    let mut exponent = if half_life > 0.0 {
        (time_delta_f - expected_time) / half_life
    } else {
        0.0
    };
    exponent = exponent.clamp(-MAX_EXPONENT_MAG, MAX_EXPONENT_MAG);

    let factor = 2_f64.powf(exponent);
    let mut next = saturating_mul(anchor, factor, max_target);

    if height_delta > 0 && (height_delta as u64) >= params.burst_guard_window && time_delta > 0 {
        if expected_time > 0.0 && time_delta_f < expected_time * params.burst_guard_floor_ratio {
            let multiplier = params.burst_guard_multiplier.max(1.0);
            next = saturating_div(next, multiplier, min_target);
        }
    }

    clamp_target(next, min_target, max_target)
}

fn clamp_target(value: f64, min_target: f64, max_target: f64) -> f64 {
    let mut v = value;
    if !v.is_finite() || v.is_nan() {
        v = max_target;
    }
    if v < min_target {
        min_target
    } else if v > max_target {
        max_target
    } else {
        v
    }
}

fn expected_window_time(height_delta: i64, block_time: u64) -> f64 {
    if height_delta <= 0 {
        return 0.0;
    }
    let hd = height_delta as u128;
    let bt = block_time as u128;
    let product = bt.saturating_mul(hd);
    product as f64
}

fn saturating_mul(lhs: f64, rhs: f64, max_target: f64) -> f64 {
    if lhs <= 0.0 || rhs <= 0.0 {
        return (lhs.max(0.0)) * (rhs.max(0.0));
    }
    let product = lhs * rhs;
    if !product.is_finite() || product.is_nan() || product > max_target {
        max_target
    } else {
        product
    }
}

fn saturating_div(value: f64, divisor: f64, min_target: f64) -> f64 {
    if divisor <= 0.0 {
        return clamp_target(value, min_target, f64::MAX);
    }
    let result = value / divisor;
    if !result.is_finite() || result.is_nan() {
        min_target
    } else if result < min_target {
        min_target
    } else {
        result
    }
}

fn saturating_i64_to_f64(value: i64) -> f64 {
    if value >= 0 {
        (value as u64) as f64
    } else {
        -(value.unsigned_abs() as f64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        compact_to_target, pow::DEVNET_MAX_BITS, target_to_compact, ConsensusParams,
        DifficultyState, RewardSchedule,
    };

    fn params() -> ConsensusParams {
        ConsensusParams {
            block_weight_cap: 4_000_000,
            signature_weight_alpha: 384,
            witness_weight_beta: 0.5,
            target_block_time: 600,
            difficulty_half_life: 14_400,
            burst_guard_window: 11,
            burst_guard_floor_ratio: 0.33,
            burst_guard_multiplier: 1.5,
            reward_schedule: RewardSchedule::phase3_defaults(),
        }
    }

    fn params_no_guard() -> ConsensusParams {
        let mut cfg = params();
        cfg.burst_guard_window = u64::MAX;
        cfg.burst_guard_floor_ratio = 0.0;
        cfg.burst_guard_multiplier = 1.0;
        cfg
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
        let max_target = compact_to_target(DEVNET_MAX_BITS);
        let anchor = max_target;
        let target = asert_next_target(anchor, -10, 10_000, &params());
        assert!(target <= max_target);
    }

    #[test]
    fn clamps_to_min_target() {
        let anchor = 1000.0;
        // Fast blocks for many height steps
        let target = asert_next_target(anchor, 100, 1_000, &params());
        let min_target = 1.0;
        assert!(target >= min_target);
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

    #[test]
    fn burst_guard_triggers_for_fast_blocks() {
        let anchor = 10_000.0;
        let guarded = params();
        let no_guard = params_no_guard();

        let height = guarded.burst_guard_window as i64;
        let expected = (guarded.target_block_time * guarded.burst_guard_window) as i64;
        let fast_time = (expected as f64 * guarded.burst_guard_floor_ratio * 0.9) as i64;

        let baseline = asert_next_target(anchor, height, expected, &guarded);
        let fast_without_guard = asert_next_target(anchor, height, fast_time, &no_guard);
        let fast_with_guard = asert_next_target(anchor, height, fast_time, &guarded);

        assert!(fast_without_guard < baseline);
        assert!(fast_with_guard < fast_without_guard / 1.25); // guard should cut noticeably
        assert!(fast_with_guard <= fast_without_guard / guarded.burst_guard_multiplier + 10.0);
    }

    #[test]
    fn burst_guard_ignores_small_window() {
        let anchor = 10_000.0;
        let guarded = params();
        let no_guard = params_no_guard();

        let short = asert_next_target(anchor, 5, 1_000, &guarded);
        let short_expected = asert_next_target(anchor, 5, 1_000, &no_guard);
        let diff = (short - short_expected).abs();
        assert!(diff < 1e-6);
    }

    #[test]
    fn guard_triggers_on_fast_streak_exact_window() {
        let anchor = 50_000.0;
        let guarded = params();
        let no_guard = params_no_guard();
        let height = guarded.burst_guard_window as i64;
        let expected = (guarded.target_block_time * guarded.burst_guard_window) as i64;
        let fast_time = (expected as f64 * guarded.burst_guard_floor_ratio * 0.9) as i64;

        let guarded_target = asert_next_target(anchor, height, fast_time, &guarded);
        let unguarded_target = asert_next_target(anchor, height, fast_time, &no_guard);

        assert!(guarded_target < unguarded_target);
        let ratio = guarded_target / unguarded_target;
        assert!(ratio <= 1.0 / guarded.burst_guard_multiplier + 0.05);
    }

    #[test]
    fn guard_does_not_trigger_on_boundary() {
        let anchor = 50_000.0;
        let guarded = params();
        let no_guard = params_no_guard();

        let height = guarded.burst_guard_window as i64;
        let expected = (guarded.target_block_time * guarded.burst_guard_window) as i64;
        let close_time = (expected as f64 * 0.36) as i64;

        let guarded_target = asert_next_target(anchor, height, close_time, &guarded);
        let unguarded_target = asert_next_target(anchor, height, close_time, &no_guard);

        assert!((guarded_target - unguarded_target).abs() < 1e-6);
    }

    #[test]
    fn guard_ignores_single_outlier() {
        let anchor = 75_000.0;
        let guarded = params();
        let no_guard = params_no_guard();

        let height = guarded.burst_guard_window as i64;
        let expected = (guarded.target_block_time * guarded.burst_guard_window) as i64;
        let total = expected - 400; // one block 400s fast vs others normal

        let guarded_target = asert_next_target(anchor, height, total, &guarded);
        let unguarded_target = asert_next_target(anchor, height, total, &no_guard);

        assert!((guarded_target - unguarded_target).abs() < 1e-6);
    }

    #[test]
    fn guard_post_reorg_consistency() {
        let params = params();
        let anchor_height = 10_000u64;
        let anchor_time = 1_000_000u64;
        let anchor_bits = 0x1d00ffff;
        let window = params.burst_guard_window;

        let fast_delta = ((params.target_block_time * window) as f64
            * params.burst_guard_floor_ratio
            * 0.9) as u64;
        let slow_delta = params.target_block_time * window + 1_200;

        let mut slow_state = DifficultyState::new(anchor_height, anchor_time, anchor_bits);
        let _ = slow_state.update(anchor_height + window, anchor_time + slow_delta, &params);

        let fast_float = asert_next_target(
            compact_to_target(anchor_bits),
            window as i64,
            fast_delta as i64,
            &params,
        );
        let slow_float = asert_next_target(
            compact_to_target(anchor_bits),
            window as i64,
            slow_delta as i64,
            &params,
        );

        assert!(fast_float < slow_float);

        let mut slow_state_after = slow_state.clone();
        let next_height = anchor_height + window + 1;
        let next_timestamp = anchor_time + slow_delta + params.target_block_time;
        let via_state_bits = slow_state_after.update(next_height, next_timestamp, &params);

        let direct_target =
            asert_next_target(slow_float, 1, params.target_block_time as i64, &params);
        let direct_bits = target_to_compact(direct_target);

        assert_eq!(via_state_bits, direct_bits);
    }

    #[test]
    fn timestamp_attack_resilience() {
        let guard = params();
        let no_guard = params_no_guard();
        let anchor = 42_000.0;
        let window = guard.burst_guard_window as i64;
        let expected = (guard.target_block_time * guard.burst_guard_window) as i64;

        let attempted_delta =
            ((expected as f64) * guard.burst_guard_floor_ratio * 0.5).max(1.0) as i64;
        let attacked = asert_next_target(anchor, window, attempted_delta, &guard);
        let attacked_no_guard = asert_next_target(anchor, window, attempted_delta, &no_guard);
        assert!(attacked < attacked_no_guard);

        let enforced_delta = expected - 1_200;
        assert!(
            (enforced_delta as f64) > expected as f64 * guard.burst_guard_floor_ratio,
            "MTP clamp should keep the delta above guard floor"
        );

        let enforced = asert_next_target(anchor, window, enforced_delta, &guard);
        let enforced_no_guard = asert_next_target(anchor, window, enforced_delta, &no_guard);
        assert!((enforced - enforced_no_guard).abs() < 1e-6);
    }

    #[test]
    fn asert_no_overflow_on_extremes() {
        let params = params();
        let anchor = compact_to_target(DEVNET_MAX_BITS) * 0.9;
        let min_target = 1.0;
        let max_target = compact_to_target(DEVNET_MAX_BITS);

        let extremely_easy = asert_next_target(anchor, i64::MAX, i64::MAX, &params);
        let extremely_hard = asert_next_target(anchor, i64::MAX, i64::MIN, &params);

        assert!(extremely_easy.is_finite());
        assert!(extremely_hard.is_finite());
        assert!(extremely_easy <= max_target);
        assert!(extremely_hard >= min_target);
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
            difficulty_half_life: 14_400,
            burst_guard_window: 11,
            burst_guard_floor_ratio: 0.33,
            burst_guard_multiplier: 1.5,
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
            let max_target = compact_to_target(DEVNET_MAX_BITS);
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
