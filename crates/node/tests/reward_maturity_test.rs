//! Integration tests for reward maturity system.
//!
//! Tests the 100-block maturity requirement for mining rewards,
//! balance tracking, and settlement logic.

use bitquan_node::pool_db::{BlockRecord, PoolDatabase};
use bitquan_node::reward_engine::RewardEngine;

/// Helper to create a test block record.
fn create_test_block(height: u64, miner_id: &str, reward: u64) -> BlockRecord {
    BlockRecord {
        hash: format!("block_{}", height),
        height,
        miner_id: miner_id.to_string(),
        reward,
        timestamp: 1234567890 + height,
        spendable: false,
    }
}

#[test]
fn test_reward_becomes_spendable_after_100_blocks() {
    // Create in-memory database
    let db = PoolDatabase::memory().expect("Failed to create database");
    let mut engine = RewardEngine::new(db);

    // Mine block at height 0
    let block = create_test_block(0, "miner1", 50_0000_0000);
    engine
        .process_block(&block, "miner1")
        .expect("Failed to process block");

    // At height 99, reward should still be pending
    let settled = engine
        .settle_pending_rewards(99)
        .expect("Failed to settle at height 99");
    assert_eq!(settled.len(), 0, "No rewards should be settled at height 99");

    // At height 100, reward should become spendable
    let settled = engine
        .settle_pending_rewards(100)
        .expect("Failed to settle at height 100");
    assert_eq!(
        settled.len(),
        1,
        "One reward should be settled at height 100"
    );
    assert_eq!(settled[0].height, 0);
    assert_eq!(settled[0].miner_id, "miner1");
}

#[test]
fn test_balance_tracking_total_spendable_pending() {
    let db = PoolDatabase::memory().expect("Failed to create database");
    let mut engine = RewardEngine::new(db);

    // Mine 10 blocks for miner1
    for height in 0..10 {
        let block = create_test_block(height, "miner1", 50_0000_0000);
        engine
            .process_block(&block, "miner1")
            .expect("Failed to process block");
    }

    // At height 50, no blocks are mature yet
    engine
        .settle_pending_rewards(50)
        .expect("Failed to settle at height 50");

    let balance = engine
        .get_balance_info("miner1")
        .expect("Failed to get balance");

    assert_eq!(balance.total, 500_0000_0000, "Total should be 10 blocks");
    assert_eq!(balance.spendable, 0, "No blocks mature at height 50");
    assert_eq!(
        balance.pending, 500_0000_0000,
        "All rewards pending at height 50"
    );

    // At height 110, first block (height 0) should be mature
    engine
        .settle_pending_rewards(110)
        .expect("Failed to settle at height 110");

    let balance = engine
        .get_balance_info("miner1")
        .expect("Failed to get balance");

    assert_eq!(balance.total, 500_0000_0000, "Total unchanged");
    assert_eq!(balance.spendable, 50_0000_0000, "One block mature");
    assert_eq!(balance.pending, 450_0000_0000, "Nine blocks still pending");
}

#[test]
fn test_multiple_miners_independent_balances() {
    let db = PoolDatabase::memory().expect("Failed to create database");
    let mut engine = RewardEngine::new(db);

    // Miner1 mines blocks 0-4
    for height in 0..5 {
        let block = create_test_block(height, "miner1", 50_0000_0000);
        engine
            .process_block(&block, "miner1")
            .expect("Failed to process block");
    }

    // Miner2 mines blocks 5-9
    for height in 5..10 {
        let block = create_test_block(height, "miner2", 50_0000_0000);
        engine
            .process_block(&block, "miner2")
            .expect("Failed to process block");
    }

    // Settle at height 110
    engine
        .settle_pending_rewards(110)
        .expect("Failed to settle");

    // Miner1 should have 5 mature blocks
    let balance1 = engine
        .get_balance_info("miner1")
        .expect("Failed to get miner1 balance");
    assert_eq!(balance1.total, 250_0000_0000);
    assert_eq!(balance1.spendable, 250_0000_0000);
    assert_eq!(balance1.pending, 0);

    // Miner2 should have 5 blocks, 0 mature (heights 5-9 need height 105-109)
    let balance2 = engine
        .get_balance_info("miner2")
        .expect("Failed to get miner2 balance");
    assert_eq!(balance2.total, 250_0000_0000);
    assert_eq!(balance2.spendable, 50_0000_0000); // Only block 5 mature at 105
    assert_eq!(balance2.pending, 200_0000_0000);
}

#[test]
fn test_settlement_at_exact_maturity_height() {
    let db = PoolDatabase::memory().expect("Failed to create database");
    let mut engine = RewardEngine::new(db);

    // Mine block at height 50
    let block = create_test_block(50, "miner1", 50_0000_0000);
    engine
        .process_block(&block, "miner1")
        .expect("Failed to process block");

    // At height 149, not mature
    let settled = engine
        .settle_pending_rewards(149)
        .expect("Failed to settle at 149");
    assert_eq!(settled.len(), 0);

    // At height 150, exactly mature (50 + 100 = 150)
    let settled = engine
        .settle_pending_rewards(150)
        .expect("Failed to settle at 150");
    assert_eq!(settled.len(), 1);
    assert_eq!(settled[0].height, 50);
}

#[test]
fn test_edge_case_height_zero() {
    let db = PoolDatabase::memory().expect("Failed to create database");
    let mut engine = RewardEngine::new(db);

    // Genesis block at height 0
    let block = create_test_block(0, "genesis", 50_0000_0000);
    engine
        .process_block(&block, "genesis")
        .expect("Failed to process genesis");

    // Should mature at height 100
    let settled = engine
        .settle_pending_rewards(100)
        .expect("Failed to settle");
    assert_eq!(settled.len(), 1);
    assert_eq!(settled[0].height, 0);
}

#[test]
fn test_progressive_settlement() {
    let db = PoolDatabase::memory().expect("Failed to create database");
    let mut engine = RewardEngine::new(db);

    // Mine blocks 0-9
    for height in 0..10 {
        let block = create_test_block(height, "miner1", 50_0000_0000);
        engine
            .process_block(&block, "miner1")
            .expect("Failed to process block");
    }

    // Settle progressively
    for current_height in 100..=109 {
        let settled = engine
            .settle_pending_rewards(current_height)
            .expect("Failed to settle");

        let expected_settled = (current_height - 99) as usize;
        assert_eq!(
            settled.len(),
            expected_settled,
            "At height {}, {} blocks should be settled",
            current_height,
            expected_settled
        );
    }

    // All should be spendable now
    let balance = engine
        .get_balance_info("miner1")
        .expect("Failed to get balance");
    assert_eq!(balance.spendable, 500_0000_0000);
    assert_eq!(balance.pending, 0);
}

#[test]
fn test_no_rewards_for_unknown_miner() {
    let db = PoolDatabase::memory().expect("Failed to create database");
    let engine = RewardEngine::new(db);

    let balance = engine
        .get_balance_info("unknown_miner")
        .expect("Failed to get balance");

    assert_eq!(balance.total, 0);
    assert_eq!(balance.spendable, 0);
    assert_eq!(balance.pending, 0);
}

#[test]
fn test_settlement_idempotent() {
    let db = PoolDatabase::memory().expect("Failed to create database");
    let mut engine = RewardEngine::new(db);

    let block = create_test_block(0, "miner1", 50_0000_0000);
    engine
        .process_block(&block, "miner1")
        .expect("Failed to process block");

    // Settle multiple times at same height
    for _ in 0..3 {
        let settled = engine
            .settle_pending_rewards(100)
            .expect("Failed to settle");
        assert_eq!(settled.len(), 1, "Should settle same block each time");
    }

    // Balance should still be correct
    let balance = engine
        .get_balance_info("miner1")
        .expect("Failed to get balance");
    assert_eq!(balance.spendable, 50_0000_0000);
}
