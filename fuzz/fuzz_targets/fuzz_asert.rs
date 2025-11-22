#![no_main]

use libfuzzer_sys::fuzz_target;
use bitquan_consensus::{asert_next_target, ConsensusParams, DifficultyParams, RewardSchedule, PowSetParams};

// Fuzz ASERT integer math for edge cases and overflow conditions
fuzz_target!(|data: &[u8]| {
    // Ensure we have enough data for all parameters
    if data.len() < 24 {
        return;
    }

    // Extract parameters from fuzz data (deterministic extraction)
    let mut bytes = [0u8; 24];
    bytes.copy_from_slice(&data[..24]);

    // Parse parameters with safe defaults
    let anchor_target = u64::from_le_bytes([
        bytes[0], bytes[1], bytes[2], bytes[3],
        bytes[4], bytes[5], bytes[6], bytes[7],
    ]);

    let height_delta = i64::from_le_bytes([
        bytes[8], bytes[9], bytes[10], bytes[11],
        bytes[12], bytes[13], bytes[14], bytes[15],
    ]);

    let time_delta = i64::from_le_bytes([
        bytes[16], bytes[17], bytes[18], bytes[19],
        bytes[20], bytes[21], bytes[22], bytes[23],
    ]);

    // Create test consensus parameters
    let params = ConsensusParams {
        block_weight_cap: 4_000_000,
        signature_weight_alpha: 384,
        witness_weight_beta: 0.5,
        reward_schedule: RewardSchedule::phase3_defaults(),
        difficulty: DifficultyParams::phase3_defaults(),
        pow_set: PowSetParams::mainnet(),
    };

    // Test ASERT calculation with extreme values
    // This should never panic or cause undefined behavior
    let _result = asert_next_target(
        anchor_target,
        height_delta,
        time_delta,
        &params,
        None, // No burst guard for basic fuzzing
    );

    // Test with burst guard enabled
    let _result_guarded = asert_next_target(
        anchor_target,
        height_delta,
        time_delta,
        &params,
        Some(bitquan_consensus::asert::GuardContext {
            state: &mut bitquan_consensus::asert::BurstGuardState::default(),
            current_height: height_delta.max(0) as u64,
            activation_height: 0,
        }),
    );

    // Test edge cases manually
    if data.len() >= 32 {
        // Test maximum values
        let max_anchor = u64::MAX;
        let max_height = i64::MAX;
        let max_time = i64::MAX;

        let _ = asert_next_target(max_anchor, max_height, max_time, &params, None);

        // Test minimum values
        let min_anchor = u64::MIN;
        let min_height = i64::MIN;
        let min_time = i64::MIN;

        let _ = asert_next_target(min_anchor, min_height, min_time, &params, None);

        // Test zero values
        let _ = asert_next_target(0, 0, 0, &params, None);

        // Test negative time delta (should handle gracefully)
        let _ = asert_next_target(anchor_target, height_delta, -1000, &params, None);
    }
});
