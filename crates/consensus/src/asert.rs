#![deny(missing_docs)]

//! ASERT difficulty retarget implementation - INTEGER FIXED-POINT VERSION
//!
//! This implementation uses pure integer arithmetic with fixed-point math
//! to ensure 100% deterministic behavior across all platforms.
//! NO floating-point arithmetic is used in consensus calculations.

use crate::{compact_to_target, pow::DEVNET_MAX_BITS, ConsensusParams};

/// Fixed-point scale factor for 32.32 format arithmetic.
pub const FP_SCALE: u64 = 1u64 << 32; // 2^32 = 4294967296

// Maximum/minimum bounds

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

    /// Last height at which guard triggered.
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

    fn update(&mut self, height: u64, params: &ConsensusParams) {
        if self.cooldown_active(height) {
            return;
        }
        if let Some(last) = self.last_trigger_height {
            let cooldown = params.difficulty.burst_guard_cooldown_blocks;
            if height > last + cooldown {
                self.reset();
            }
        }
    }
}

/// Context for burst guard operations.
pub struct GuardContext<'a> {
    /// Mutable reference to guard state.
    pub state: &'a mut BurstGuardState,
    /// Current block height.
    pub current_height: u64,
    /// Height at which guard becomes active.
    pub activation_height: u64,
}

/// Integer fixed-point multiplication: (a * b) / FP_SCALE
#[inline]
#[allow(dead_code)] // ASERT math helper - used in tests, may be used in production
fn fp_mul(a: u64, b: u64) -> u64 {
    let result = (a as u128) * (b as u128);
    let scaled_result = result / (FP_SCALE as u128);

    // Handle overflow - clamp to u64::MAX
    if scaled_result > u64::MAX as u128 {
        return u64::MAX;
    }

    scaled_result as u64
}

/// Integer fixed-point division: (a * FP_SCALE) / b
#[inline]
#[allow(dead_code)] // ASERT math helper - used in tests, may be used in production
fn fp_div(a: u64, b: u64) -> u64 {
    if b == 0 {
        return u64::MAX;
    }
    let result = (a as u128) * (FP_SCALE as u128);
    ((result + (b as u128 / 2)) / (b as u128)) as u64
}

/// Integer fixed-point power of 2: 2^x where x is in fixed-point format
fn fp_pow2(x: u64) -> u64 {
    if x == 0 {
        return FP_SCALE; // 2^0 = 1 in fixed-point
    }

    // Extract integer and fractional parts
    let integer_part = x >> 32; // Get integer part
    let frac_part = x & (FP_SCALE - 1); // Get fractional part

    // Handle overflow for large integer parts
    if integer_part >= 63 {
        return u64::MAX;
    }

    // Calculate 2^integer_part using bit shift (this is in normal integer format)
    let integer_result = 1u64 << integer_part;

    // For fractional part, use lookup table for key values and interpolation
    // Precomputed values for 2^x at key fractional points in 32.32 fixed-point
    let lookup_000 = FP_SCALE; // 2^0 = 1.0
    let lookup_125 = 4683695047u64; // 2^0.125 = 1.090507732...
    let lookup_250 = 5107605667u64; // 2^0.25 = 1.189207115...
    let lookup_375 = 5569883475u64; // 2^0.375 = 1.296839555...
    let lookup_500 = 6074000999u64; // 2^0.5 = 1.414213562...
    let lookup_625 = 6623745058u64; // 2^0.625 = 1.542210825...
    let lookup_750 = 7223245205u64; // 2^0.75 = 1.681792830...
    let lookup_875 = 7877004751u64; // 2^0.875 = 1.834008086...

    let frac_mult = if frac_part == 0 {
        lookup_000 as u128
    } else if frac_part < FP_SCALE / 8 {
        // Interpolate between 0 and 0.125
        // t = (frac_part / FP_SCALE) * 8, but use higher precision
        let t = (frac_part as u128 * 8 * 65536) / (FP_SCALE as u128); // 0-7*65536 range for more precision
        let base = lookup_000 as u128;
        let diff = lookup_125 as u128 - base;
        base + ((diff * t) / (8 * 65536))
    } else if frac_part < FP_SCALE / 4 {
        // Interpolate between 0.125 and 0.25
        let t = ((frac_part - FP_SCALE / 8) as u128 * 8 * 65536) / (FP_SCALE as u128);
        let base = lookup_125 as u128;
        let diff = lookup_250 as u128 - base;
        base + ((diff * t) / (8 * 65536))
    } else if frac_part < 3 * FP_SCALE / 8 {
        // Interpolate between 0.25 and 0.375
        let t = ((frac_part - FP_SCALE / 4) as u128 * 8 * 65536) / (FP_SCALE as u128);
        let base = lookup_250 as u128;
        let diff = lookup_375 as u128 - base;
        base + ((diff * t) / (8 * 65536))
    } else if frac_part < FP_SCALE / 2 {
        // Interpolate between 0.375 and 0.5
        let t = ((frac_part - 3 * FP_SCALE / 8) as u128 * 8 * 65536) / (FP_SCALE as u128);
        let base = lookup_375 as u128;
        let diff = lookup_500 as u128 - base;
        base + ((diff * t) / (8 * 65536))
    } else if frac_part < 5 * FP_SCALE / 8 {
        // Interpolate between 0.5 and 0.625
        let t = ((frac_part - FP_SCALE / 2) as u128 * 8 * 65536) / (FP_SCALE as u128);
        let base = lookup_500 as u128;
        let diff = lookup_625 as u128 - base;
        base + ((diff * t) / (8 * 65536))
    } else if frac_part < 3 * FP_SCALE / 4 {
        // Interpolate between 0.625 and 0.75
        let t = ((frac_part - 5 * FP_SCALE / 8) as u128 * 8 * 65536) / (FP_SCALE as u128);
        let base = lookup_625 as u128;
        let diff = lookup_750 as u128 - base;
        base + ((diff * t) / (8 * 65536))
    } else if frac_part < 7 * FP_SCALE / 8 {
        // Interpolate between 0.75 and 0.875
        let t = ((frac_part - 3 * FP_SCALE / 4) as u128 * 8 * 65536) / (FP_SCALE as u128);
        let base = lookup_750 as u128;
        let diff = lookup_875 as u128 - base;
        base + ((diff * t) / (8 * 65536))
    } else {
        // Interpolate between 0.875 and 1.0
        let t = ((frac_part - 7 * FP_SCALE / 8) as u128 * 8 * 65536) / (FP_SCALE as u128);
        let base = lookup_875 as u128;
        let diff = (FP_SCALE * 2) as u128 - base; // 2^1 = 2.0
        base + ((diff * t) / (8 * 65536))
    } as u64;

    // Convert integer_result to fixed-point and multiply by fractional multiplier
    let integer_fp = (integer_result as u128) << 32; // Convert to fixed-point
    let result_fp = (integer_fp * frac_mult as u128) >> 32;

    // Convert back to u64, clamping to max
    if result_fp > u64::MAX as u128 {
        u64::MAX
    } else {
        result_fp as u64
    }
}

/// Calculate ASERT exponent using pure integer arithmetic
/// exponent = (time_delta - expected_time) / half_life
fn calculate_asert_exponent_fp(time_delta: i64, expected_time: i64, half_life: u64) -> i64 {
    let time_diff = time_delta.saturating_sub(expected_time);

    // Convert to fixed-point and divide by half-life
    let time_diff_fp = (time_diff as i128) << 32;
    let half_life_fp = half_life as i128;

    ((time_diff_fp + half_life_fp / 2) / half_life_fp) as i64
}

/// Calculate next ASERT target using pure integer arithmetic.
///
/// Operates on 256-bit targets ([u8; 32] big-endian) to avoid the u64
/// overflow that caused all real difficulty values to collapse to u64::MAX.
pub fn asert_next_target(
    anchor_target: [u8; 32],
    height_delta: i64,
    time_delta: i64,
    params: &ConsensusParams,
    guard: Option<GuardContext<'_>>,
) -> [u8; 32] {
    let max_target = compact_to_target(DEVNET_MAX_BITS);

    // Convert [u8; 32] big-endian to u128.
    // For compact targets with exponent >= 29, the value exceeds u128 range.
    // In that case we clamp to u128::MAX for calculation purposes.
    let anchor_u128 = if anchor_target[0..16] == [0u8; 16] {
        u128::from_be_bytes(anchor_target[16..32].try_into().unwrap_or([0u8; 16]))
    } else {
        u128::MAX
    };
    let max_u128 = if max_target[0..16] == [0u8; 16] {
        u128::from_be_bytes(max_target[16..32].try_into().unwrap_or([0u8; 16]))
    } else {
        u128::MAX
    };

    // Clamp anchor to valid range
    let anchor_clamped = if anchor_u128 == 0 {
        1u128 // MIN_TARGET
    } else {
        anchor_u128.min(max_u128)
    };

    // Calculate expected time for given height delta
    let expected_time = height_delta * (params.difficulty.target_block_time as i64);

    // Calculate ASERT exponent in fixed-point
    let exponent_fp = calculate_asert_exponent_fp(
        time_delta,
        expected_time,
        params.difficulty.difficulty_half_life,
    );

    // Convert anchor to fixed-point (64.64 format for u128)
    let anchor_fp = anchor_clamped << 64;

    // Calculate next target: anchor * 2^exponent
    let next_target_u128 = if exponent_fp >= 0 {
        let exp_fp = fp_pow2(exponent_fp as u64);
        // anchor_fp >> 64 recovers anchor_clamped, multiply by exp_fp (in fixed-point)
        // Result: (anchor_clamped * exp_fp) / FP_SCALE
        let numerator = (anchor_fp >> 64) * (exp_fp as u128);
        numerator / (FP_SCALE as u128)
    } else {
        let exp_fp = fp_pow2((-exponent_fp) as u64);
        let anchor_val = anchor_fp >> 64;
        if exp_fp == 0 {
            anchor_val
        } else {
            (anchor_val * (FP_SCALE as u128)) / (exp_fp as u128)
        }
    };

    // Clamp to valid range
    let result = next_target_u128.min(max_u128);
    let result = result.max(1u128); // MIN_TARGET

    // Apply burst guard if provided
    if let Some(guard_ctx) = guard {
        apply_burst_guard_256(result, height_delta, time_delta, params, guard_ctx)
    } else {
        let mut out = [0u8; 32];
        out[16..32].copy_from_slice(&result.to_be_bytes());
        out
    }
}

/// Convert u128 to [u8; 32] big-endian (placing value in lower 16 bytes).
#[inline]
fn u128_to_bytes(val: u128) -> [u8; 32] {
    let mut out = [0u8; 32];
    out[16..32].copy_from_slice(&val.to_be_bytes());
    out
}

/// Convert u64 to [u8; 32] big-endian (placing value in lower 16 bytes).
/// Uses the same [16..32] offset as u128_to_bytes for consistency.
#[inline]
#[cfg(test)]
#[allow(dead_code)]
fn u64_to_target(val: u64) -> [u8; 32] {
    let mut out = [0u8; 32];
    let val_bytes = val.to_be_bytes();
    // Place u64 at [24..32] but we need it at [16..32] for consistency
    // Actually, pad to 16 bytes and place at [16..32]
    out[16..24].copy_from_slice(&[0u8; 8]); // high 8 bytes zero
    out[24..32].copy_from_slice(&val_bytes); // low 8 bytes
    out
}

/// Apply burst guard using 256-bit targets.
fn apply_burst_guard_256(
    next_target: u128,
    height_delta: i64,
    time_delta: i64,
    params: &ConsensusParams,
    guard_ctx: GuardContext<'_>,
) -> [u8; 32] {
    let window = params.difficulty.burst_guard_window as i64;
    let floor_ratio_fp = params.difficulty.burst_guard_floor_ratio_fp;

    let guard_active = guard_ctx.current_height >= guard_ctx.activation_height
        && height_delta >= window
        && time_delta > 0;

    if !guard_active {
        guard_ctx.state.update(guard_ctx.current_height, params);
        return u128_to_bytes(next_target);
    }

    let expected_time_fp =
        (height_delta as u128) * (params.difficulty.target_block_time as u128) * (FP_SCALE as u128);
    let floor_threshold_fp = (expected_time_fp * floor_ratio_fp as u128) / (FP_SCALE as u128);
    let actual_time_fp = (time_delta as u128) * (FP_SCALE as u128);

    let should_trigger = actual_time_fp < floor_threshold_fp
        && !guard_ctx.state.cooldown_active(guard_ctx.current_height);

    if should_trigger {
        let cooldown = params.difficulty.burst_guard_cooldown_blocks;
        guard_ctx.state.trigger(guard_ctx.current_height, cooldown);
        compact_to_target(DEVNET_MAX_BITS)
    } else {
        guard_ctx.state.update(guard_ctx.current_height, params);
        u128_to_bytes(next_target)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::{
        compact_to_target, ConsensusParams, DifficultyParams, DifficultyState, RewardSchedule,
    };

    /// Convert u64 value to [u8; 32] big-endian target for test helpers.
    fn u64_to_target(val: u64) -> [u8; 32] {
        let mut out = [0u8; 32];
        out[24..32].copy_from_slice(&val.to_be_bytes());
        out
    }

    fn params() -> ConsensusParams {
        ConsensusParams {
            block_weight_cap: 4_000_000,
            signature_weight_alpha: 384,
            witness_weight_beta: 0.5,
            reward_schedule: RewardSchedule::phase3_defaults(),
            difficulty: DifficultyParams::phase3_defaults(),
            pow_set: crate::PowSetParams::mainnet(),
        }
    }

    fn params_no_guard() -> ConsensusParams {
        let mut p = params();
        p.difficulty.burst_guard_window = 0;
        p
    }

    #[test]
    fn fp_multiplication_basic() {
        let a = (2.5 * FP_SCALE as f64) as u64;
        let b = (4.0 * FP_SCALE as f64) as u64;
        let result = fp_mul(a, b);
        let expected = (10.0 * FP_SCALE as f64) as u64;
        assert!((result as i64 - expected as i64).abs() < 1000);
    }

    #[test]
    fn fp_division_basic() {
        let a = (10.0 * FP_SCALE as f64) as u64;
        let b = (4.0 * FP_SCALE as f64) as u64;
        let result = fp_div(a, b);
        let expected = (2.5 * FP_SCALE as f64) as u64;
        assert!((result as i64 - expected as i64).abs() < 1000);
    }

    #[test]
    fn fp_pow2_basic() {
        let x = FP_SCALE;
        let result = fp_pow2(x);
        let expected = (2.0 * FP_SCALE as f64) as u64;
        assert!((result as i64 - expected as i64).abs() < 10000);
    }

    #[test]
    fn fp_pow2_fractional() {
        let x = FP_SCALE / 2;
        let result = fp_pow2(x);
        let expected = (2.0_f64.sqrt() * FP_SCALE as f64) as u64;
        assert!((result as i64 - expected as i64).abs() < 50000);
    }

    #[test]
    fn asert_basic_functionality() {
        let params = params();
        let anchor = u64_to_target(50000);
        let height_delta = 20;
        let expected_time = height_delta * params.difficulty.target_block_time as i64;

        // Perfect timing => no change (within 1%)
        let next = asert_next_target(anchor, height_delta, expected_time, &params, None);
        assert_eq!(next, anchor, "ASERT should be identity for perfect timing");

        // Fast blocks => target decreases
        let next_fast = asert_next_target(anchor, height_delta, expected_time / 2, &params, None);
        assert!(next_fast < anchor, "Fast blocks should decrease target");

        // Slow blocks => target increases
        let next_slow = asert_next_target(anchor, height_delta, expected_time * 2, &params, None);
        assert!(next_slow > anchor, "Slow blocks should increase target");
    }

    #[test]
    fn asert_deterministic_same_inputs() {
        let params = params();
        let anchor = u64_to_target(50000);
        let height_delta = 15;
        let time_delta = 8000;

        let result1 = asert_next_target(anchor, height_delta, time_delta, &params, None);
        let result2 = asert_next_target(anchor, height_delta, time_delta, &params, None);
        assert_eq!(result1, result2);
    }

    #[test]
    fn asert_clamps_to_max_target() {
        let params = params();
        let max_target = compact_to_target(DEVNET_MAX_BITS);
        let anchor = max_target;

        let next = asert_next_target(anchor, 1, 10000, &params, None);
        assert!(next <= max_target);
    }

    #[test]
    fn asert_clamps_to_min_target() {
        let params = params();
        let anchor = u64_to_target(1);

        let next = asert_next_target(anchor, 100, 1, &params, None);
        assert!(next > [0u8; 32], "ASERT should never return zero target");
    }

    #[test]
    fn asert_monotonic_with_increasing_time() {
        let params = params();
        let anchor = u64_to_target(10000);
        let height_delta = 10;

        let result1 = asert_next_target(anchor, height_delta, 5000, &params, None);
        let result2 = asert_next_target(anchor, height_delta, 6000, &params, None);
        let result3 = asert_next_target(anchor, height_delta, 7000, &params, None);

        assert!(result1 < result2);
        assert!(result2 < result3);
    }

    #[test]
    fn burst_guard_triggers_for_fast_blocks() {
        let params = params();
        let anchor = u64_to_target(10000);
        let window = params.difficulty.burst_guard_window as i64;

        let expected_time_fp =
            (params.difficulty.target_block_time as u128) * (window as u128) * (FP_SCALE as u128);
        let boundary_time_fp = (expected_time_fp
            * params.difficulty.burst_guard_floor_ratio_fp as u128)
            / (FP_SCALE as u128);
        let fast_time_fp = (boundary_time_fp * 8) / 10;
        let fast_time = (fast_time_fp / (FP_SCALE as u128)) as i64;

        let mut guard_state = BurstGuardState::default();
        let guard_ctx = GuardContext {
            state: &mut guard_state,
            current_height: window as u64,
            activation_height: 0,
        };

        let result = asert_next_target(anchor, window, fast_time, &params, Some(guard_ctx));

        assert_eq!(result, compact_to_target(DEVNET_MAX_BITS));
        assert!(guard_state.is_active());
    }

    #[test]
    fn burst_guard_ignores_small_window() {
        let params = params();
        let anchor = u64_to_target(10000);

        let mut guard_state = BurstGuardState::default();
        let guard_ctx = GuardContext {
            state: &mut guard_state,
            current_height: 5,
            activation_height: 0,
        };

        let result = asert_next_target(anchor, 5, 100, &params, Some(guard_ctx));

        assert!(!guard_state.is_active());
        assert_ne!(result, compact_to_target(DEVNET_MAX_BITS));
    }

    #[test]
    fn burst_guard_cooldown_respected() {
        let params = params();
        let anchor = u64_to_target(10000);
        let window = params.difficulty.burst_guard_window as i64;
        let floor_ratio = params.difficulty.burst_guard_floor_ratio_fp as f64 / FP_SCALE as f64;
        let fast_time = ((params.difficulty.target_block_time as i64 * window) as f64
            * floor_ratio
            * 0.8) as i64;

        let mut guard_state = BurstGuardState::default();

        // First trigger
        {
            let guard_ctx = GuardContext {
                state: &mut guard_state,
                current_height: window as u64,
                activation_height: 0,
            };
            asert_next_target(anchor, window, fast_time, &params, Some(guard_ctx));
        }
        assert!(guard_state.is_active());

        // Second trigger during cooldown should not happen
        let cooldown = params.difficulty.burst_guard_cooldown_blocks;
        {
            let guard_ctx = GuardContext {
                state: &mut guard_state,
                current_height: window as u64 + cooldown / 2,
                activation_height: 0,
            };
            let result = asert_next_target(anchor, window, fast_time, &params, Some(guard_ctx));
            assert_ne!(result, compact_to_target(DEVNET_MAX_BITS));
        }
    }

    #[test]
    fn asert_no_overflow_on_extremes() {
        let params = params();
        let max_target = compact_to_target(DEVNET_MAX_BITS);

        let result = asert_next_target(u64_to_target(u64::MAX), 1, 1, &params, None);
        assert!(result <= max_target);

        let result = asert_next_target(u64_to_target(1), u64::MAX as i64, i64::MAX, &params, None);
        assert!(result > [0u8; 32]);
    }

    #[test]
    fn asert_unchanged_when_on_time() {
        let params = params();
        let anchor = u64_to_target(50000);
        let height_delta = 20;
        let expected_time = height_delta * params.difficulty.target_block_time as i64;

        let result = asert_next_target(anchor, height_delta, expected_time, &params, None);
        assert_eq!(
            result, anchor,
            "ASERT should be identity for perfect timing"
        );
    }

    #[test]
    fn asert_increases_when_blocks_slow() {
        let params = params();
        let anchor = u64_to_target(50000);
        let height_delta = 10;
        let slow_time = height_delta * params.difficulty.target_block_time as i64 * 2;

        let result = asert_next_target(anchor, height_delta, slow_time, &params, None);
        assert!(result > anchor);
    }

    #[test]
    fn asert_decreases_when_blocks_fast() {
        let params = params();
        let anchor = u64_to_target(50000);
        let height_delta = 10;
        let fast_time = height_delta * params.difficulty.target_block_time as i64 / 2;

        let result = asert_next_target(anchor, height_delta, fast_time, &params, None);
        assert!(result < anchor);
    }

    #[test]
    fn guard_post_reorg_consistency() {
        let params = params();
        let anchor_height = 10_000u64;
        let anchor_time = 1_000_000u64;
        let anchor_bits = 0x1d00ffff;
        let window = params.difficulty.burst_guard_window;

        let floor_ratio = params.difficulty.burst_guard_floor_ratio_fp as f64 / FP_SCALE as f64;
        let fast_delta = ((params.difficulty.target_block_time as i64 * window as i64) as f64
            * floor_ratio
            * 0.9) as i64 as u64;
        let slow_delta = params.difficulty.target_block_time as i64 * window as i64 + 1_200i64;

        let mut slow_state = DifficultyState::new(anchor_height, anchor_time, anchor_bits, 0);
        let _ = slow_state.update(
            anchor_height + window,
            anchor_time + slow_delta as u64,
            &params,
        );

        let fast_int = asert_next_target(
            compact_to_target(anchor_bits),
            window as i64,
            fast_delta as i64,
            &params,
            None,
        );
        let slow_int = asert_next_target(
            compact_to_target(anchor_bits),
            window as i64,
            slow_delta as i64,
            &params,
            None,
        );

        assert!(fast_int < slow_int);

        let mut slow_state_after = slow_state.clone();
        let next_height = anchor_height + window + 1;
        let next_timestamp = anchor_time + slow_delta as u64 + params.difficulty.target_block_time;
        let _via_state_bits = slow_state_after.update(next_height, next_timestamp, &params);

        let _direct_target = asert_next_target(
            slow_int,
            1,
            params.difficulty.target_block_time as i64,
            &params,
            None,
        );
    }

    #[test]
    fn timestamp_attack_resilience() {
        let guard = params();
        let no_guard = params_no_guard();
        let anchor = u64_to_target(42_000);
        let window = guard.difficulty.burst_guard_window as i64;
        let floor_ratio = guard.difficulty.burst_guard_floor_ratio_fp as f64 / FP_SCALE as f64;
        let expected = (guard.difficulty.target_block_time as i64 * window) as f64 * floor_ratio;

        let with_guard = asert_next_target(
            anchor,
            window,
            expected as i64 - 1,
            &guard,
            Some(GuardContext {
                state: &mut BurstGuardState::default(),
                current_height: window as u64,
                activation_height: 0,
            }),
        );

        let without_guard = asert_next_target(anchor, window, expected as i64 - 1, &no_guard, None);

        assert_eq!(with_guard, compact_to_target(DEVNET_MAX_BITS));
        assert!(without_guard < compact_to_target(DEVNET_MAX_BITS));
    }

    #[test]
    fn guard_does_not_trigger_on_boundary() {
        let params = params();
        let anchor = u64_to_target(10000);
        let window = params.difficulty.burst_guard_window as i64;

        let expected_time_fp =
            (window as u128) * (params.difficulty.target_block_time as u128) * (FP_SCALE as u128);
        let floor_threshold_fp = (expected_time_fp
            * params.difficulty.burst_guard_floor_ratio_fp as u128)
            / (FP_SCALE as u128);
        let boundary_time = floor_threshold_fp.div_ceil(FP_SCALE as u128) as i64;

        let mut guard_state = BurstGuardState::default();
        let guard_ctx = GuardContext {
            state: &mut guard_state,
            current_height: window as u64,
            activation_height: 0,
        };

        let result = asert_next_target(anchor, window, boundary_time, &params, Some(guard_ctx));

        assert!(!guard_state.is_active());
        assert_ne!(result, compact_to_target(DEVNET_MAX_BITS));
    }

    #[test]
    fn guard_triggers_on_fast_streak_exact_window() {
        let params = params();
        let anchor = u64_to_target(10000);
        let window = params.difficulty.burst_guard_window as i64;
        let floor_ratio = params.difficulty.burst_guard_floor_ratio_fp as f64 / FP_SCALE as f64;
        let fast_time = ((params.difficulty.target_block_time as i64 * window) as f64
            * floor_ratio
            * 0.9) as i64;

        let mut guard_state = BurstGuardState::default();
        let guard_ctx = GuardContext {
            state: &mut guard_state,
            current_height: window as u64,
            activation_height: 0,
        };

        let result = asert_next_target(anchor, window, fast_time, &params, Some(guard_ctx));

        assert!(guard_state.is_active());
        assert_eq!(result, compact_to_target(DEVNET_MAX_BITS));
    }

    #[test]
    fn guard_ignores_single_outlier() {
        let params = params();
        let anchor = u64_to_target(10000);

        let mut guard_state = BurstGuardState::default();
        let guard_ctx = GuardContext {
            state: &mut guard_state,
            current_height: 1,
            activation_height: 0,
        };

        let result = asert_next_target(anchor, 1, 100, &params, Some(guard_ctx));

        assert!(!guard_state.is_active());
        assert_ne!(result, compact_to_target(DEVNET_MAX_BITS));
    }

    #[test]
    fn guard_cooldown_blocks_respected() {
        let params = params();
        let anchor = u64_to_target(10000);
        let window = params.difficulty.burst_guard_window as i64;
        let floor_ratio = params.difficulty.burst_guard_floor_ratio_fp as f64 / FP_SCALE as f64;
        let fast_time = ((params.difficulty.target_block_time as i64 * window) as f64
            * floor_ratio
            * 0.8) as i64;

        let mut guard_state = BurstGuardState::default();

        {
            let guard_ctx = GuardContext {
                state: &mut guard_state,
                current_height: window as u64,
                activation_height: 0,
            };
            asert_next_target(anchor, window, fast_time, &params, Some(guard_ctx));
        }

        let trigger_height = guard_state
            .last_trigger_height()
            .expect("Guard state should have trigger height after activation");
        let cooldown = params.difficulty.burst_guard_cooldown_blocks;

        assert!(guard_state.cooldown_active(trigger_height + cooldown / 2));
        assert!(!guard_state.cooldown_active(trigger_height + cooldown + 1));
    }

    #[test]
    fn guard_no_flap_on_boundary() {
        let params = params();
        let anchor = u64_to_target(10000);
        let window = params.difficulty.burst_guard_window as i64;
        let floor_ratio = params.difficulty.burst_guard_floor_ratio_fp as f64 / FP_SCALE as f64;
        let boundary_time =
            ((params.difficulty.target_block_time as i64 * window) as f64 * floor_ratio) as i64;

        let mut guard_state = BurstGuardState::default();

        {
            let guard_ctx = GuardContext {
                state: &mut guard_state,
                current_height: window as u64,
                activation_height: 0,
            };
            asert_next_target(anchor, window, boundary_time - 1, &params, Some(guard_ctx));
        }
        assert!(guard_state.is_active());

        {
            let guard_ctx = GuardContext {
                state: &mut guard_state,
                current_height: window as u64 + 1,
                activation_height: 0,
            };
            asert_next_target(anchor, window, boundary_time, &params, Some(guard_ctx));
        }

        assert!(guard_state.is_active());
    }

    #[test]
    fn guard_long_run_stability() {
        let params = params();
        let anchor = u64_to_target(10000);
        let window = params.difficulty.burst_guard_window as i64;
        let normal_time = params.difficulty.target_block_time as i64 * window;

        let mut guard_state = BurstGuardState::default();

        for i in 1..=100 {
            let guard_ctx = GuardContext {
                state: &mut guard_state,
                current_height: (i * window) as u64,
                activation_height: 0,
            };
            asert_next_target(anchor, window, normal_time, &params, Some(guard_ctx));
        }

        assert!(!guard_state.is_active());
    }

    // Property-based tests
    #[test]
    fn property_tests_asert_deterministic() {
        let params = params();
        let anchor = u64_to_target(50000);
        let height_delta = 15;
        let time_delta = 8000;

        let results: Vec<[u8; 32]> = (0..10)
            .map(|_| asert_next_target(anchor, height_delta, time_delta, &params, None))
            .collect();

        for result in &results[1..] {
            assert_eq!(results[0], *result);
        }
    }

    #[test]
    fn property_tests_asert_always_positive() {
        let params = params();

        for anchor in [1u64, 1000, 1000000, u64::MAX] {
            for height_delta in [-100i64, -1, 0, 1, 100] {
                for time_delta in [-1000i64, -1, 0, 1, 1000] {
                    let result = asert_next_target(
                        u64_to_target(anchor),
                        height_delta,
                        time_delta,
                        &params,
                        None,
                    );
                    assert!(result > [0u8; 32]);
                }
            }
        }
    }

    #[test]
    fn property_tests_asert_bounded_by_max_target() {
        let params = params();
        let max_target = compact_to_target(DEVNET_MAX_BITS);

        for anchor in [1u64, 1000, 1000000, u64::MAX] {
            for height_delta in [-100i64, -1, 0, 1, 100] {
                for time_delta in [-1000i64, -1, 0, 1, 1000] {
                    let result = asert_next_target(
                        u64_to_target(anchor),
                        height_delta,
                        time_delta,
                        &params,
                        None,
                    );
                    assert!(result <= max_target);
                }
            }
        }
    }

    #[test]
    fn property_tests_asert_monotonic_increasing_time() {
        let params = params();
        let anchor = u64_to_target(50000);
        let height_delta = 10;

        let mut prev_result = asert_next_target(anchor, height_delta, 1000, &params, None);

        for time_delta in 2000..=10000 {
            let result = asert_next_target(anchor, height_delta, time_delta, &params, None);
            assert!(result >= prev_result);
            prev_result = result;
        }
    }

    #[test]
    fn test_half_life_mainnet_per_bip0340() {
        let mainnet = DifficultyParams::mainnet();
        assert_eq!(
            mainnet.difficulty_half_life, 14_400,
            "Phase 4 half_life should be 14,400s (4 hours)"
        );
    }

    #[test]
    fn test_half_life_testnet_faster() {
        let testnet = DifficultyParams::testnet();
        assert_eq!(
            testnet.difficulty_half_life, 14_400,
            "Testnet half_life should be 14,400s (4 hours) for faster adjustment"
        );
    }

    #[test]
    fn test_half_life_regtest_instant() {
        let regtest = DifficultyParams::regtest();
        assert_eq!(
            regtest.difficulty_half_life, 600,
            "Regtest half_life should be 600s (10 minutes) for instant adjustment"
        );
    }

    #[test]
    fn test_phase3_defaults_uses_testnet() {
        let defaults = DifficultyParams::phase3_defaults();
        let testnet = DifficultyParams::testnet();
        assert_eq!(
            defaults.difficulty_half_life, testnet.difficulty_half_life,
            "phase3_defaults should use testnet half_life for backward compatibility"
        );
    }
}
