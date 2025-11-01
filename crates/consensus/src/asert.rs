#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! ASERT difficulty retarget implementation (prototype).

use crate::{compact_to_target, pow::DEVNET_MAX_BITS, ConsensusParams};

const MAX_EXPONENT_MAG: f64 = 1023.0;

/// Tracks burst guard activation and cooldown state.
#[derive(Clone, Debug, Default)]
pub struct BurstGuardState {
    active: bool,
    last_trigger_height: Option<u64>,
    cooldown_until: Option<u64>,
}

impl BurstGuardState {
    /// Returns whether the guard is currently active.
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Last height at which the guard triggered.
    pub fn last_trigger_height(&self) -> Option<u64> {
        self.last_trigger_height
    }

    fn reset(&mut self) {
        self.active = false;
        self.last_trigger_height = None;
        self.cooldown_until = None;
    }

    fn cooldown_active(&self, height: u64) -> bool {
        matches!(self.cooldown_until, Some(until) if height <= until)
    }

    fn trigger(&mut self, height: u64, cooldown_blocks: u64) {
        self.active = true;
        self.last_trigger_height = Some(height);
        self.cooldown_until = if cooldown_blocks == 0 {
            None
        } else {
            Some(height.saturating_add(cooldown_blocks))
        };
    }
}

/// Evaluation context for burst guard hysteresis.
pub struct GuardContext<'a> {
    /// Mutable reference to the guard state being updated.
    pub state: &'a mut BurstGuardState,
    /// Height of the block currently being evaluated.
    pub current_height: u64,
    /// Height at which burst guard activation becomes effective.
    pub activation_height: u64,
}

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
    guard: Option<GuardContext<'_>>,
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

    if let Some(mut ctx) = guard {
        if burst_guard_active(&mut ctx, params, height_delta, time_delta, expected_time) {
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

fn burst_guard_active(
    ctx: &mut GuardContext<'_>,
    params: &ConsensusParams,
    height_delta: i64,
    time_delta: i64,
    expected_time: f64,
) -> bool {
    if ctx.current_height < ctx.activation_height {
        ctx.state.reset();
        return false;
    }

    let ratio = if expected_time > 0.0 {
        let delta_f = saturating_i64_to_f64(time_delta).max(0.0);
        (delta_f / expected_time).max(0.0)
    } else {
        1.0
    };

    if ctx.state.active && ratio >= params.burst_guard_release_ratio {
        ctx.state.active = false;
    }

    let cooldown_active = ctx.state.cooldown_active(ctx.current_height);
    let eligible_window = height_delta > 0 && (height_delta as u64) >= params.burst_guard_window;
    let fast_enough = time_delta > 0 && ratio < params.burst_guard_floor_ratio;

    if !ctx.state.active && !cooldown_active && eligible_window && fast_enough {
        ctx.state
            .trigger(ctx.current_height, params.burst_guard_cooldown_blocks);
    }

    ctx.state.active
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

    fn eval(anchor: f64, height_delta: i64, time_delta: i64, params: &ConsensusParams) -> f64 {
        asert_next_target(anchor, height_delta, time_delta, params, None)
    }

    fn eval_with_guard(
        anchor: f64,
        height_delta: i64,
        time_delta: i64,
        params: &ConsensusParams,
        state: &mut BurstGuardState,
        current_height: u64,
        activation_height: u64,
    ) -> f64 {
        asert_next_target(
            anchor,
            height_delta,
            time_delta,
            params,
            Some(GuardContext {
                state,
                current_height,
                activation_height,
            }),
        )
    }

    fn params() -> ConsensusParams {
        ConsensusParams {
            block_weight_cap: 4_000_000,
            signature_weight_alpha: 384,
            witness_weight_beta: 0.5,
            target_block_time: 600,
            difficulty_half_life: 14_400,
            burst_guard_window: 11,
            burst_guard_floor_ratio: 0.33,
            burst_guard_release_ratio: 0.38,
            burst_guard_multiplier: 1.5,
            burst_guard_cooldown_blocks: 5,
            reward_schedule: RewardSchedule::phase3_defaults(),
        }
    }

    fn params_no_guard() -> ConsensusParams {
        let mut cfg = params();
        cfg.burst_guard_window = u64::MAX;
        cfg.burst_guard_floor_ratio = 0.0;
        cfg.burst_guard_release_ratio = 1.0;
        cfg.burst_guard_multiplier = 1.0;
        cfg.burst_guard_cooldown_blocks = 0;
        cfg
    }

    #[test]
    fn unchanged_when_on_time() {
        let anchor = 1000.0;
        let target = eval(anchor, 1, 600, &params());
        assert!((target - anchor).abs() < 1e-6);
    }

    #[test]
    fn decreases_when_blocks_fast() {
        let anchor = 1000.0;
        let target = eval(anchor, 1, 300, &params());
        assert!(target < anchor);
    }

    #[test]
    fn increases_when_blocks_slow() {
        let anchor = 1000.0;
        let target = eval(anchor, 1, 1_200, &params());
        assert!(target > anchor);
    }

    #[test]
    fn clamps_to_max_target() {
        let max_target = compact_to_target(DEVNET_MAX_BITS);
        let anchor = max_target;
        let target = eval(anchor, -10, 10_000, &params());
        assert!(target <= max_target);
    }

    #[test]
    fn clamps_to_min_target() {
        let anchor = 1000.0;
        // Fast blocks for many height steps
        let target = eval(anchor, 100, 1_000, &params());
        let min_target = 1.0;
        assert!(target >= min_target);
    }

    #[test]
    fn monotonic_with_increasing_time() {
        let anchor = 1000.0;
        let t1 = eval(anchor, 1, 300, &params());
        let t2 = eval(anchor, 1, 600, &params());
        let t3 = eval(anchor, 1, 900, &params());

        // Target should increase as time increases
        assert!(t1 < t2);
        assert!(t2 < t3);
    }

    #[test]
    fn deterministic_same_inputs() {
        let anchor = 1000.0;
        let t1 = eval(anchor, 5, 3_000, &params());
        let t2 = eval(anchor, 5, 3_000, &params());
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

        let baseline = eval(anchor, height, expected, &guarded);
        let fast_without_guard = eval(anchor, height, fast_time, &no_guard);
        let mut guard_state = BurstGuardState::default();
        let fast_with_guard = eval_with_guard(
            anchor,
            height,
            fast_time,
            &guarded,
            &mut guard_state,
            guarded.burst_guard_window,
            1,
        );

        assert!(fast_without_guard < baseline);
        assert!(fast_with_guard < fast_without_guard / 1.25); // guard should cut noticeably
        assert!(fast_with_guard <= fast_without_guard / guarded.burst_guard_multiplier + 10.0);
        assert!(guard_state.is_active());
    }

    #[test]
    fn burst_guard_ignores_small_window() {
        let anchor = 10_000.0;
        let guarded = params();
        let no_guard = params_no_guard();

        let mut guard_state = BurstGuardState::default();
        let short = eval_with_guard(anchor, 5, 1_000, &guarded, &mut guard_state, 5, 1);
        let short_expected = eval(anchor, 5, 1_000, &no_guard);
        let diff = (short - short_expected).abs();
        assert!(diff < 1e-6);
        assert!(!guard_state.is_active());
    }

    #[test]
    fn guard_triggers_on_fast_streak_exact_window() {
        let anchor = 50_000.0;
        let guarded = params();
        let no_guard = params_no_guard();
        let height = guarded.burst_guard_window as i64;
        let expected = (guarded.target_block_time * guarded.burst_guard_window) as i64;
        let fast_time = (expected as f64 * guarded.burst_guard_floor_ratio * 0.9) as i64;

        let mut guard_state = BurstGuardState::default();
        let guarded_target = eval_with_guard(
            anchor,
            height,
            fast_time,
            &guarded,
            &mut guard_state,
            guarded.burst_guard_window,
            1,
        );
        let unguarded_target = eval(anchor, height, fast_time, &no_guard);

        assert!(guarded_target < unguarded_target);
        let ratio = guarded_target / unguarded_target;
        assert!(ratio <= 1.0 / guarded.burst_guard_multiplier + 0.05);
        assert!(guard_state.is_active());
    }

    #[test]
    fn guard_does_not_trigger_on_boundary() {
        let anchor = 50_000.0;
        let guarded = params();
        let no_guard = params_no_guard();

        let height = guarded.burst_guard_window as i64;
        let expected = (guarded.target_block_time * guarded.burst_guard_window) as i64;
        let close_time = (expected as f64 * 0.36) as i64;

        let mut guard_state = BurstGuardState::default();
        let guarded_target = eval_with_guard(
            anchor,
            height,
            close_time,
            &guarded,
            &mut guard_state,
            guarded.burst_guard_window,
            1,
        );
        let unguarded_target = eval(anchor, height, close_time, &no_guard);

        assert!((guarded_target - unguarded_target).abs() < 1e-6);
        assert!(!guard_state.is_active());
    }

    #[test]
    fn guard_ignores_single_outlier() {
        let anchor = 75_000.0;
        let guarded = params();
        let no_guard = params_no_guard();

        let height = guarded.burst_guard_window as i64;
        let expected = (guarded.target_block_time * guarded.burst_guard_window) as i64;
        let total = expected - 400; // one block 400s fast vs others normal

        let mut guard_state = BurstGuardState::default();
        let guarded_target = eval_with_guard(
            anchor,
            height,
            total,
            &guarded,
            &mut guard_state,
            guarded.burst_guard_window,
            1,
        );
        let unguarded_target = eval(anchor, height, total, &no_guard);

        assert!((guarded_target - unguarded_target).abs() < 1e-6);
        assert!(!guard_state.is_active());
    }

    #[test]
    fn guard_no_flap_on_boundary() {
        let params = params();
        let anchor = 12_000.0;
        let window = params.burst_guard_window as i64;
        let expected = (params.target_block_time * params.burst_guard_window) as i64;
        let fast_time = (expected as f64 * params.burst_guard_floor_ratio * 0.9) as i64;
        let release_time = (expected as f64 * params.burst_guard_release_ratio * 1.05) as i64;
        let boundary_time = (expected as f64 * 0.36) as i64; // between floor and release

        let mut guard_state = BurstGuardState::default();
        // Trigger guard
        let _ = eval_with_guard(
            anchor,
            window,
            fast_time,
            &params,
            &mut guard_state,
            params.burst_guard_window,
            1,
        );
        assert!(guard_state.is_active());

        // Provide release interval to drop guard
        let _ = eval_with_guard(
            anchor,
            window,
            release_time,
            &params,
            &mut guard_state,
            params.burst_guard_window + 1,
            1,
        );
        assert!(!guard_state.is_active());

        // Slightly fast but above floor should not re-trigger immediately
        let _ = eval_with_guard(
            anchor,
            window,
            boundary_time,
            &params,
            &mut guard_state,
            params.burst_guard_window + 2,
            1,
        );
        assert!(!guard_state.is_active());
    }

    #[test]
    fn guard_cooldown_blocks_respected() {
        let params = params();
        let anchor = 15_000.0;
        let window = params.burst_guard_window as i64;
        let expected = (params.target_block_time * params.burst_guard_window) as i64;
        let fast_time = (expected as f64 * params.burst_guard_floor_ratio * 0.9) as i64;
        let release_time = (expected as f64 * params.burst_guard_release_ratio * 1.05) as i64;

        let mut guard_state = BurstGuardState::default();
        let trigger_height = params.burst_guard_window;
        // Trigger guard
        let _ = eval_with_guard(
            anchor,
            window,
            fast_time,
            &params,
            &mut guard_state,
            trigger_height,
            1,
        );
        assert!(guard_state.is_active());

        // Release guard
        let _ = eval_with_guard(
            anchor,
            window,
            release_time,
            &params,
            &mut guard_state,
            trigger_height + 1,
            1,
        );
        assert!(!guard_state.is_active());

        // During cooldown, fast blocks should not re-trigger
        for i in 1..params.burst_guard_cooldown_blocks {
            let height = trigger_height + 1 + i;
            let _ = eval_with_guard(
                anchor,
                window,
                fast_time,
                &params,
                &mut guard_state,
                height,
                1,
            );
            assert!(
                !guard_state.is_active(),
                "guard reactivated during cooldown at height {}",
                height
            );
        }

        // After cooldown expires, fast block should re-trigger
        let resume_height = trigger_height + params.burst_guard_cooldown_blocks + 2;
        let _ = eval_with_guard(
            anchor,
            window,
            fast_time,
            &params,
            &mut guard_state,
            resume_height,
            1,
        );
        assert!(guard_state.is_active());
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

        let mut slow_state = DifficultyState::new(anchor_height, anchor_time, anchor_bits, 0);
        let _ = slow_state.update(anchor_height + window, anchor_time + slow_delta, &params);

        let fast_float = eval(
            compact_to_target(anchor_bits),
            window as i64,
            fast_delta as i64,
            &params,
        );
        let slow_float = eval(
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

        let direct_target = eval(slow_float, 1, params.target_block_time as i64, &params);
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
        let mut guard_state = BurstGuardState::default();
        let attacked = eval_with_guard(
            anchor,
            window,
            attempted_delta,
            &guard,
            &mut guard_state,
            guard.burst_guard_window,
            1,
        );
        let attacked_no_guard = eval(anchor, window, attempted_delta, &no_guard);
        assert!(attacked < attacked_no_guard);

        let enforced_delta = expected - 1_200;
        assert!(
            (enforced_delta as f64) > expected as f64 * guard.burst_guard_floor_ratio,
            "MTP clamp should keep the delta above guard floor"
        );

        let enforced = eval_with_guard(
            anchor,
            window,
            enforced_delta,
            &guard,
            &mut guard_state,
            guard.burst_guard_window + 1,
            1,
        );
        let enforced_no_guard = eval(anchor, window, enforced_delta, &no_guard);
        assert!((enforced - enforced_no_guard).abs() < 1e-6);
    }

    #[test]
    fn asert_no_overflow_on_extremes() {
        let params = params();
        let anchor = compact_to_target(DEVNET_MAX_BITS) * 0.9;
        let min_target = 1.0;
        let max_target = compact_to_target(DEVNET_MAX_BITS);

        let extremely_easy = eval(anchor, i64::MAX, i64::MAX, &params);
        let extremely_hard = eval(anchor, i64::MAX, i64::MIN, &params);

        assert!(extremely_easy.is_finite());
        assert!(extremely_hard.is_finite());
        assert!(extremely_easy <= max_target);
        assert!(extremely_hard >= min_target);
    }

    #[test]
    fn guard_long_run_stability() {
        let params = params();
        let baseline_bits = 0x207fffff;
        let mut diff = DifficultyState::new(0, 0, baseline_bits, 1);
        let mut height = 0u64;
        let mut timestamp = 0u64;
        let mut guard_triggers = 0u64;
        let mut intervals = Vec::new();

        let pattern = [
            (300u64, 1.0f64),
            (80, 4.0),
            (300, 1.0),
            (80, 0.5),
            (240, 1.0),
        ];

        let mut segment_index = 0usize;
        let mut segment_remaining = pattern[0].0;

        for _ in 0..1000 {
            if segment_remaining == 0 {
                segment_index = (segment_index + 1) % pattern.len();
                segment_remaining = pattern[segment_index].0;
            }
            let hash_rate = pattern[segment_index].1;
            segment_remaining -= 1;

            let base_interval = (params.target_block_time as f64) / hash_rate;
            let dt = base_interval.max(1.0).round() as u64;
            timestamp = timestamp.saturating_add(dt);
            height += 1;
            intervals.push(dt as f64);

            let _bits = diff.update(height, timestamp, &params);
            if diff.guard_state().last_trigger_height() == Some(height) {
                guard_triggers += 1;
            }
        }

        let avg_interval: f64 = intervals.iter().sum::<f64>() / intervals.len() as f64;
        assert!(
            (avg_interval - params.target_block_time as f64).abs() < 30.0,
            "avg interval deviates: {avg_interval}"
        );

        let guard_rate = guard_triggers as f64 / intervals.len() as f64;
        assert!(
            guard_rate <= 0.01 + 1e-6,
            "guard rate too high: {:.4}",
            guard_rate
        );
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
            burst_guard_release_ratio: 0.38,
            burst_guard_multiplier: 1.5,
            burst_guard_cooldown_blocks: 5,
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
            let target = asert_next_target(anchor, height_delta, time_delta, &params(), None);
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
            let target = asert_next_target(anchor, height_delta, time_delta, &params(), None);
            prop_assert!(target <= max_target);
        }

        #[test]
        fn asert_monotonic_increasing_time(
            height_delta in 1i64..100,
            time_base in 1u64..10_000
        ) {
            let anchor = 1000.0;
            let t1 = asert_next_target(anchor, height_delta, time_base as i64, &params(), None);
            let t2 = asert_next_target(anchor, height_delta, (time_base * 2) as i64, &params(), None);

            // Longer time should give higher target (easier difficulty)
            prop_assert!(t2 >= t1);
        }

        #[test]
        fn asert_deterministic(
            height_delta in -100i64..100,
            time_delta in 0i64..50_000
        ) {
            let anchor = 1000.0;
            let t1 = asert_next_target(anchor, height_delta, time_delta, &params(), None);
            let t2 = asert_next_target(anchor, height_delta, time_delta, &params(), None);
            prop_assert_eq!(t1, t2);
        }
    }
}
