//! Reward calculation and distribution engine.
//!
//! Implements Bitcoin-like halving schedule and miner reward tracking.

use bitquan_types::{Block, Result};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use crate::pool_db::{BlockRecord, PayoutRecord, PoolDatabase};

/// Initial block reward in satoshis (50 BQ).
const INITIAL_REWARD: u64 = 50_0000_0000;

/// Halving interval (blocks).
const HALVING_INTERVAL: u64 = 210_000;

/// Reward engine for calculating and distributing mining rewards.
pub struct RewardEngine {
    /// Pool database for persistence.
    db: PoolDatabase,
    /// Reward multiplier (default 1.0).
    reward_rate: f64,
    /// Block maturity for rewards (confirmations needed).
    maturity: u64,
    /// Total rewards distributed counter.
    total_distributed: Arc<AtomicU64>,
}

impl RewardEngine {
    /// Create a new reward engine.
    pub fn new(db: PoolDatabase) -> Self {
        // Load total from database
        let total = db.total_rewards().unwrap_or(0);

        Self {
            db,
            reward_rate: 1.0,
            maturity: 100,
            total_distributed: Arc::new(AtomicU64::new(total)),
        }
    }

    /// Calculate block reward based on height and fees.
    ///
    /// Implements Bitcoin-style halving:
    /// - Reward halves every 210,000 blocks
    /// - Initial reward: 50 BQ (5,000,000,000 satoshis)
    /// - Plus transaction fees
    pub fn calculate_reward(&self, block: &Block, height: u64) -> u64 {
        // Calculate base reward with halving
        let halvings = height / HALVING_INTERVAL;
        let base_reward = if halvings >= 64 {
            0 // After 64 halvings, reward is 0
        } else {
            INITIAL_REWARD >> halvings
        };

        // Calculate transaction fees
        let fees = self.calculate_fees(block);

        // Apply reward rate multiplier
        let total = base_reward + fees;
        (total as f64 * self.reward_rate) as u64
    }

    /// Calculate total transaction fees in block.
    fn calculate_fees(&self, block: &Block) -> u64 {
        let mut total_in = 0u64;
        let mut total_out = 0u64;

        for tx in &block.transactions {
            // Sum inputs (would need UTXO lookup in production)
            // For now, assume coinbase tx has no inputs
            if !tx.inputs.is_empty() {
                for _input in &tx.inputs {
                    // TODO: Look up input values from UTXO set
                    // total_in += input_value;
                }
            }

            // Sum outputs
            for output in &tx.outputs {
                total_out = total_out.saturating_add(output.value);
            }
        }

        // Fees = inputs - outputs (for non-coinbase txs)
        total_in.saturating_sub(total_out)
    }

    /// Credit reward to miner account.
    pub fn credit_miner(&mut self, miner_id: &str, amount: u64) -> Result<()> {
        self.db
            .update_miner_reward(miner_id, amount)
            .map_err(|e| bitquan_types::Error::Invalid(format!("DB error: {}", e)))?;

        self.total_distributed.fetch_add(amount, Ordering::Relaxed);
        Ok(())
    }

    /// Record a block and credit its reward.
    pub fn record_block(
        &mut self,
        block: &Block,
        block_hash: [u8; 32],
        height: u64,
        miner_id: &str,
    ) -> Result<u64> {
        // Calculate reward
        let reward = self.calculate_reward(block, height);

        // Create block record
        let record = BlockRecord {
            hash: hex::encode(block_hash),
            height,
            miner_id: miner_id.to_string(),
            reward,
            timestamp: block.header.time as u64,
        };

        // Persist block
        self.db
            .insert_block(&record)
            .map_err(|e| bitquan_types::Error::Invalid(format!("DB error: {}", e)))?;

        // Credit miner
        self.credit_miner(miner_id, reward)?;

        Ok(reward)
    }

    /// Settle pending rewards (placeholder for maturity logic).
    pub fn settle_pending_rewards(&mut self) -> Result<Vec<String>> {
        // TODO: Implement maturity check
        // Would check blocks that are now mature and mark rewards as spendable
        Ok(vec![])
    }

    /// Record a payout transaction.
    pub fn record_payout(
        &mut self,
        miner_id: &str,
        amount: u64,
        txid: Option<String>,
    ) -> Result<String> {
        let payout_id = uuid::Uuid::new_v4().to_string();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let payout = PayoutRecord {
            id: payout_id.clone(),
            miner_id: miner_id.to_string(),
            amount,
            txid,
            created_at: now,
        };

        self.db
            .insert_payout(&payout)
            .map_err(|e| bitquan_types::Error::Invalid(format!("DB error: {}", e)))?;

        Ok(payout_id)
    }

    /// Get total rewards distributed.
    pub fn total_distributed(&self) -> u64 {
        self.total_distributed.load(Ordering::Relaxed)
    }

    /// Get miner's total reward.
    pub fn get_miner_reward(&self, miner_id: &str) -> Result<u64> {
        self.db
            .get_miner_reward(miner_id)
            .map_err(|e| bitquan_types::Error::Invalid(format!("DB error: {}", e)))
    }

    /// Get recent payouts.
    pub fn list_payouts(&self, limit: usize) -> Result<Vec<PayoutRecord>> {
        self.db
            .list_payouts(limit)
            .map_err(|e| bitquan_types::Error::Invalid(format!("DB error: {}", e)))
    }

    /// Get pool statistics.
    pub fn get_pool_stats(&self) -> Result<PoolStats> {
        Ok(PoolStats {
            total_rewards: self.total_distributed(),
            miner_count: self
                .db
                .miner_count()
                .map_err(|e| bitquan_types::Error::Invalid(format!("DB error: {}", e)))?,
            block_count: self
                .db
                .block_count()
                .map_err(|e| bitquan_types::Error::Invalid(format!("DB error: {}", e)))?,
            pool_balance: self.calculate_pool_balance()?,
        })
    }

    /// Calculate pool balance (rewards - payouts).
    fn calculate_pool_balance(&self) -> Result<u64> {
        let total_rewards = self.total_distributed();
        // TODO: Calculate total payouts
        // For now, pool balance equals total rewards
        Ok(total_rewards)
    }

    /// Set reward rate multiplier.
    pub fn set_reward_rate(&mut self, rate: f64) {
        self.reward_rate = rate;
    }

    /// Get reference to database.
    pub fn db(&self) -> &PoolDatabase {
        &self.db
    }
}

/// Pool statistics snapshot.
#[derive(Debug, Clone, serde::Serialize)]
pub struct PoolStats {
    pub total_rewards: u64,
    pub miner_count: u64,
    pub block_count: u64,
    pub pool_balance: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitquan_types::{BlockHeader, NetworkId, SigAlgorithm, Transaction, TxOut};

    fn dummy_block(height: u64) -> Block {
        Block {
            header: BlockHeader {
                version: 1,
                prev_block: [0u8; 32],
                merkle_root: [0u8; 32],
                pqc_agg_hint: [0u8; 32],
                time: 1234567890,
                bits: 0x207fffff,
                nonce: 0,
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

        // Block 0: full reward
        let block0 = dummy_block(0);
        let reward0 = engine.calculate_reward(&block0, 0);
        assert_eq!(reward0, INITIAL_REWARD);

        // Block 210,000: first halving
        let block1 = dummy_block(210_000);
        let reward1 = engine.calculate_reward(&block1, 210_000);
        assert_eq!(reward1, INITIAL_REWARD / 2);

        // Block 420,000: second halving
        let block2 = dummy_block(420_000);
        let reward2 = engine.calculate_reward(&block2, 420_000);
        assert_eq!(reward2, INITIAL_REWARD / 4);
    }

    #[test]
    fn test_credit_and_settle_rewards() {
        let db = PoolDatabase::memory().unwrap();
        let mut engine = RewardEngine::new(db);

        engine.credit_miner("miner1", 1000).unwrap();
        engine.credit_miner("miner1", 2000).unwrap();

        let total = engine.get_miner_reward("miner1").unwrap();
        assert_eq!(total, 3000);

        assert_eq!(engine.total_distributed(), 3000);
    }

    #[test]
    fn test_record_block() {
        let db = PoolDatabase::memory().unwrap();
        let mut engine = RewardEngine::new(db);

        let block = dummy_block(100);
        let hash = [1u8; 32];

        let reward = engine.record_block(&block, hash, 100, "miner1").unwrap();
        assert!(reward > 0);

        let miner_reward = engine.get_miner_reward("miner1").unwrap();
        assert_eq!(miner_reward, reward);
    }

    #[test]
    fn test_pool_balance_metrics() {
        let db = PoolDatabase::memory().unwrap();
        let mut engine = RewardEngine::new(db);

        let block = dummy_block(0);
        let hash = [1u8; 32];

        engine.record_block(&block, hash, 0, "miner1").unwrap();

        let stats = engine.get_pool_stats().unwrap();
        assert!(stats.total_rewards > 0);
        assert_eq!(stats.miner_count, 1);
        assert_eq!(stats.block_count, 1);
    }

    #[test]
    fn test_record_payout() {
        let db = PoolDatabase::memory().unwrap();
        let mut engine = RewardEngine::new(db);

        let payout_id = engine
            .record_payout("miner1", 1000, Some("tx123".to_string()))
            .unwrap();
        assert!(!payout_id.is_empty());

        let payouts = engine.list_payouts(10).unwrap();
        assert_eq!(payouts.len(), 1);
        assert_eq!(payouts[0].amount, 1000);
    }
}
