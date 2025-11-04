//! Integration tests for reward engine and chain persistence.

use bitquan_node::{BlockRecord, ChainState, MiningMetrics, PoolDatabase, RewardEngine};
use bitquan_types::{Block, BlockHeader, NetworkId, SigAlgorithm, Transaction, TxOut};

fn dummy_block(height: u64) -> Block {
    Block {
        header: BlockHeader {
            version: 1,
            prev_block: [0u8; 32],
            merkle_root: [0u8; 32],
            pqc_agg_hint: [0u8; 32],
            time: 1234567890 + height as u32,
            bits: 0x207fffff,
            nonce: height,
            algo_id: 0,
        },
        transactions: vec![Transaction {
            version: 1,
            network: NetworkId::Testnet,
            genesis_hash: [0u8; 32],
            lock_time: 0,
            inputs: vec![],
            outputs: vec![TxOut {
                value: 5000000000,
                script_pubkey: vec![],
            }],
            sig_algo: SigAlgorithm::Dilithium3,
            witnesses: vec![],
        }],
    }
}

#[test]
fn test_reward_halving_logic() {
    let db = PoolDatabase::memory().unwrap();
    let engine = RewardEngine::new(db);

    // Block 0: full reward (50 BQ)
    let block0 = dummy_block(0);
    let reward0 = engine.calculate_reward(&block0, 0);
    assert_eq!(reward0, 50_0000_0000, "Initial reward should be 50 BQ");

    // Block 210,000: first halving (25 BQ)
    let block1 = dummy_block(210_000);
    let reward1 = engine.calculate_reward(&block1, 210_000);
    assert_eq!(reward1, 25_0000_0000, "First halving should be 25 BQ");

    // Block 420,000: second halving (12.5 BQ)
    let block2 = dummy_block(420_000);
    let reward2 = engine.calculate_reward(&block2, 420_000);
    assert_eq!(reward2, 12_5000_0000, "Second halving should be 12.5 BQ");

    // Block 630,000: third halving (6.25 BQ)
    let block3 = dummy_block(630_000);
    let reward3 = engine.calculate_reward(&block3, 630_000);
    assert_eq!(reward3, 6_2500_0000, "Third halving should be 6.25 BQ");
}

#[test]
fn test_block_persistence_and_height_increment() {
    let db = PoolDatabase::memory().unwrap();
    let chain_state = ChainState::new();

    // Start at height 0
    assert_eq!(chain_state.get_height(), 0);

    // Append blocks and verify height increments
    for i in 0..10 {
        let block = dummy_block(i);
        let hash = [(i % 256) as u8; 32];
        let height = chain_state.append_block(&block, hash).unwrap();
        assert_eq!(height, i + 1);
    }

    assert_eq!(chain_state.get_height(), 10);
}

#[test]
fn test_credit_and_settle_rewards() {
    let db = PoolDatabase::memory().unwrap();
    let mut engine = RewardEngine::new(db);

    // Credit multiple rewards to same miner
    engine.credit_miner("miner1", 1000).unwrap();
    engine.credit_miner("miner1", 2000).unwrap();
    engine.credit_miner("miner1", 3000).unwrap();

    let total = engine.get_miner_reward("miner1").unwrap();
    assert_eq!(total, 6000, "Rewards should accumulate");

    // Credit different miner
    engine.credit_miner("miner2", 5000).unwrap();

    let total2 = engine.get_miner_reward("miner2").unwrap();
    assert_eq!(total2, 5000);

    // Check total distributed
    assert_eq!(engine.total_distributed(), 11000);
}

#[test]
fn test_pool_balance_metrics() {
    let db = PoolDatabase::memory().unwrap();
    let mut engine = RewardEngine::new(db);

    let block = dummy_block(0);
    let hash = [1u8; 32];

    // Record a block
    let reward = engine.record_block(&block, hash, 0, "miner1").unwrap();
    assert!(reward > 0, "Reward should be positive");

    // Get pool stats
    let stats = engine.get_pool_stats().unwrap();
    assert_eq!(stats.total_rewards, reward);
    assert_eq!(stats.miner_count, 1);
    assert_eq!(stats.block_count, 1);
    assert_eq!(stats.pool_balance, reward);
}

#[test]
fn test_miner_reward_accumulation() {
    let db = PoolDatabase::memory().unwrap();
    let mut engine = RewardEngine::new(db);

    // Mine 3 blocks for the same miner
    for i in 0..3 {
        let block = dummy_block(i);
        let hash = [(i % 256) as u8; 32];
        engine.record_block(&block, hash, i, "miner_alpha").unwrap();
    }

    let reward = engine.get_miner_reward("miner_alpha").unwrap();
    assert_eq!(reward, 50_0000_0000 * 3, "Miner should have 3x rewards");

    // Check block records
    let blocks = engine.db().get_miner_blocks("miner_alpha", 10).unwrap();
    assert_eq!(blocks.len(), 3, "Should have 3 blocks");

    // Verify blocks are in descending order by height
    assert_eq!(blocks[0].height, 2);
    assert_eq!(blocks[1].height, 1);
    assert_eq!(blocks[2].height, 0);
}

#[test]
fn test_multiple_miners() {
    let db = PoolDatabase::memory().unwrap();
    let mut engine = RewardEngine::new(db);

    // Mine blocks with different miners
    engine
        .record_block(&dummy_block(0), [1u8; 32], 0, "alice")
        .unwrap();
    engine
        .record_block(&dummy_block(1), [2u8; 32], 1, "bob")
        .unwrap();
    engine
        .record_block(&dummy_block(2), [3u8; 32], 2, "alice")
        .unwrap();

    let alice_reward = engine.get_miner_reward("alice").unwrap();
    let bob_reward = engine.get_miner_reward("bob").unwrap();

    assert_eq!(
        alice_reward,
        50_0000_0000 * 2,
        "Alice should have 2x rewards"
    );
    assert_eq!(bob_reward, 50_0000_0000, "Bob should have 1x reward");

    let stats = engine.get_pool_stats().unwrap();
    assert_eq!(stats.miner_count, 2, "Should have 2 miners");
}

#[test]
fn test_payout_recording() {
    let db = PoolDatabase::memory().unwrap();
    let mut engine = RewardEngine::new(db);

    // Record a payout
    let payout_id = engine
        .record_payout("miner1", 1000000, Some("tx123".to_string()))
        .unwrap();
    assert!(!payout_id.is_empty(), "Payout ID should be generated");

    // List payouts
    let payouts = engine.list_payouts(10).unwrap();
    assert_eq!(payouts.len(), 1);
    assert_eq!(payouts[0].miner_id, "miner1");
    assert_eq!(payouts[0].amount, 1000000);
    assert_eq!(payouts[0].txid, Some("tx123".to_string()));
}

#[test]
fn test_database_persistence() {
    // Use a temporary file for this test
    let temp_path = format!("/tmp/bitquan_test_{}.db", std::process::id());

    // Scope 1: Create and populate database
    {
        let db = PoolDatabase::open(&temp_path).unwrap();
        let mut engine = RewardEngine::new(db);

        engine
            .record_block(&dummy_block(0), [1u8; 32], 0, "miner1")
            .unwrap();
        engine
            .record_block(&dummy_block(1), [2u8; 32], 1, "miner1")
            .unwrap();
    }

    // Scope 2: Reopen database and verify data persists
    {
        let db = PoolDatabase::open(&temp_path).unwrap();
        let engine = RewardEngine::new(db);

        let reward = engine.get_miner_reward("miner1").unwrap();
        assert_eq!(reward, 50_0000_0000 * 2, "Rewards should persist");

        let stats = engine.get_pool_stats().unwrap();
        assert_eq!(stats.block_count, 2);
    }

    // Cleanup
    let _ = std::fs::remove_file(&temp_path);
}

#[test]
fn test_metrics_integration() {
    use bitquan_consensus::pow::PowAlgo;

    let db = PoolDatabase::memory().unwrap();
    let mut engine = RewardEngine::new(db);
    let metrics = MiningMetrics::new(&[PowAlgo::Sha256d]);

    // Record block and update metrics
    let block = dummy_block(100);
    let hash = [1u8; 32];
    let reward = engine.record_block(&block, hash, 100, "miner1").unwrap();

    metrics.record_block_persisted();
    metrics.set_total_rewards(engine.total_distributed());
    metrics.set_pool_balance(engine.total_distributed());
    metrics.set_reward_per_block(reward);

    // Verify metrics
    assert_eq!(metrics.get_blocks_persisted(), 1);
    assert_eq!(metrics.get_total_rewards(), reward);
    assert_eq!(metrics.get_pool_balance(), reward);
}

#[test]
fn test_edge_cases() {
    let db = PoolDatabase::memory().unwrap();
    let mut engine = RewardEngine::new(db);

    // Test with empty miner ID
    let result = engine.credit_miner("", 1000);
    assert!(result.is_ok(), "Empty miner ID should be handled");

    // Test with zero reward
    let result = engine.credit_miner("miner1", 0);
    assert!(result.is_ok(), "Zero reward should be handled");

    // Test getting non-existent miner
    let reward = engine.get_miner_reward("nonexistent").unwrap();
    assert_eq!(reward, 0, "Non-existent miner should have 0 reward");
}
