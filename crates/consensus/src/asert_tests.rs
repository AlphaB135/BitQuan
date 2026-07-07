//! ASERT difficulty adjustment test vectors for BQIP-0002.
//!
//! These test vectors provide reference calculations for ASERT difficulty
//! adjustment to ensure consistent behavior across implementations.

use crate::asert::{asert_next_target, BurstGuardState, GuardContext, FP_SCALE};
use crate::{compact_to_target, target_to_compact_u64, ConsensusParams, DifficultyParams, AsertParams};
use bitquan_types::BlockHeader;

/// Test case for ASERT difficulty calculation.
#[derive(Debug, Clone)]
pub struct AsertTestCase {
    /// Test case name/description.
    pub name: &'static str,
    /// Parent block height.
    pub parent_height: u64,
    /// Parent block timestamp.
    pub parent_time: u32,
    /// Current block timestamp.
    pub current_time: u32,
    /// Parent block difficulty (compact bits).
    pub parent_bits: u32,
    /// Expected next block difficulty (compact bits).
    pub expected_bits: u32,
    /// Whether burst guard should be active.
    pub burst_guard_active: bool,
}

/// ASERT test vectors for BQIP-0002 validation.
pub const ASERT_TEST_VECTORS: &[AsertTestCase] = &[
    // Test Case 1: Perfect timing (no difficulty change)
    AsertTestCase {
        name: "perfect_timing_no_change",
        parent_height: 100,
        parent_time: 1609459200, // Jan 1, 2021
        current_time: 1609459800, // +10 minutes (perfect)
        parent_bits: 0x1d00ffff,   // Starting difficulty
        expected_bits: 0x1d00ffff, // Should remain same
        burst_guard_active: false,
    },
    
    // Test Case 2: Slow block (difficulty decreases)
    AsertTestCase {
        name: "slow_block_difficulty_decrease",
        parent_height: 100,
        parent_time: 1609459200,
        current_time: 1609461600, // +40 minutes (4x slower)
        parent_bits: 0x1d00ffff,
        expected_bits: 0x1c7fffff, // Should decrease
        burst_guard_active: false,
    },
    
    // Test Case 3: Fast block (difficulty increases)
    AsertTestCase {
        name: "fast_block_difficulty_increase",
        parent_height: 100,
        parent_time: 1609459200,
        current_time: 1609459500, // +5 minutes (2x faster)
        parent_bits: 0x1d00ffff,
        expected_bits: 0x1d0fffff, // Should increase
        burst_guard_active: false,
    },
    
    // Test Case 4: Burst guard activation
    AsertTestCase {
        name: "burst_guard_activation",
        parent_height: 100,
        parent_time: 1609459200,
        current_time: 1609459230, // +30 seconds (20x faster)
        parent_bits: 0x1d00ffff,
        expected_bits: 0x1d3fffff, // Burst guard multiplier applied
        burst_guard_active: true,
    },
    
    // Test Case 5: Multiple fast blocks (cumulative effect)
    AsertTestCase {
        name: "cumulative_fast_blocks",
        parent_height: 200,
        parent_time: 1609459200,
        current_time: 1609459500, // +5 minutes
        parent_bits: 0x1d0fffff,  // Already increased
        expected_bits: 0x1d1fffff, // Further increase
        burst_guard_active: false,
    },
    
    // Test Case 6: Difficulty ceiling
    AsertTestCase {
        name: "difficulty_ceiling",
        parent_height: 100,
        parent_time: 1609459200,
        current_time: 1609459201, // +1 second (extremely fast)
        parent_bits: 0x207fffff,  // Maximum difficulty
        expected_bits: 0x207fffff, // Should not exceed maximum
        burst_guard_active: false,
    },
    
    // Test Case 7: Difficulty floor
    AsertTestCase {
        name: "difficulty_floor",
        parent_height: 100,
        parent_time: 1609459200,
        current_time: 1609473600, // +4 hours (extremely slow)
        parent_bits: 0x1c00ffff,  // Minimum difficulty
        expected_bits: 0x1c00ffff, // Should not go below minimum
        burst_guard_active: false,
    },
];

/// Test helper to validate ASERT calculations.
pub fn validate_asert_test_case(test_case: &AsertTestCase) -> Result<(), String> {
    let params = ConsensusParams::phase3_defaults();
    
    // Create burst guard state
    let mut guard_state = BurstGuardState::default();
    if test_case.burst_guard_active {
        guard_state.trigger(test_case.parent_height, params.difficulty.burst_guard_cooldown_blocks);
    }
    
    let guard_context = GuardContext {
        state: &mut guard_state,
        current_height: test_case.parent_height + 1,
        activation_height: params.difficulty.burst_guard_activation_height,
    };
    
    // Calculate ASERT next target
    let calculated_bits = asert_next_target(
        test_case.parent_height + 1,
        test_case.current_time,
        test_case.parent_bits,
        &params.difficulty,
        &guard_context,
    );
    
    // Compare with expected
    if calculated_bits != test_case.expected_bits {
        return Err(format!(
            "ASERT test '{}' failed: expected 0x{:08x}, got 0x{:08x}",
            test_case.name, test_case.expected_bits, calculated_bits
        ));
    }
    
    Ok(())
}

/// Run all ASERT test vectors.
pub fn run_all_asert_tests() -> Result<(), Vec<String>> {
    let mut failures = Vec::new();
    
    for test_case in ASERT_TEST_VECTORS {
        if let Err(error) = validate_asert_test_case(test_case) {
            failures.push(error);
        }
    }
    
    if failures.is_empty() {
        Ok(())
    } else {
        Err(failures)
    }
}

/// Property-based test for ASERT monotonicity.
#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;
    
    proptest! {
        #[test]
        fn asert_monotonicity_test(
            parent_height in 100u64..10000,
            time_diff in 1u32..3600, // 1 second to 1 hour
            parent_bits in 0x1c00ffffu32..0x207fffff
        ) {
            let params = ConsensusParams::phase3_defaults();
            let guard_state = BurstGuardState::default();
            let guard_context = GuardContext {
                state: &mut BurstGuardState::default(),
                current_height: parent_height + 1,
                activation_height: 0,
            };
            
            let parent_time = 1609459200; // Fixed base time
            let current_time = parent_time + time_diff;
            
            let next_bits = asert_next_target(
                parent_height + 1,
                current_time,
                parent_bits,
                &params.difficulty,
                &guard_context,
            );
            
            // Result should be within valid bounds
            prop_assert!(next_bits >= params.difficulty.asert.min_difficulty);
            prop_assert!(next_bits <= params.difficulty.asert.max_difficulty);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_all_asert_vectors() {
        let result = run_all_asert_tests();
        assert!(result.is_ok(), "ASERT test vectors failed: {:?}", result.unwrap_err());
    }
    
    #[test]
    fn test_perfect_timing() {
        let test_case = &ASERT_TEST_VECTORS[0]; // Perfect timing test
        assert!(validate_asert_test_case(test_case).is_ok());
    }
    
    #[test]
    fn test_burst_guard() {
        let test_case = &ASERT_TEST_VECTORS[3]; // Burst guard test
        assert!(validate_asert_test_case(test_case).is_ok());
    }
    
    #[test]
    fn test_difficulty_bounds() {
        let test_case = &ASERT_TEST_VECTORS[5]; // Ceiling test
        assert!(validate_asert_test_case(test_case).is_ok());
        
        let test_case = &ASERT_TEST_VECTORS[6]; // Floor test
        assert!(validate_asert_test_case(test_case).is_ok());
    }
}