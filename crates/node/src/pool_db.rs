//! Pool database layer for miners, blocks, and payouts.
//!
//! Provides SQLite-backed persistence for pool operations including:
//! - Miner reward tracking
//! - Block history
//! - Payout records

use rusqlite::{params, Connection, Result as SqlResult};
use std::sync::{Arc, Mutex};

/// Record of a persisted block.
#[derive(Debug, Clone)]
pub struct BlockRecord {
    pub hash: String,
    pub height: u64,
    pub miner_id: String,
    pub reward: u64,
    pub timestamp: u64,
}

/// Record of a payout transaction.
#[derive(Debug, Clone)]
pub struct PayoutRecord {
    pub id: String,
    pub miner_id: String,
    pub amount: u64,
    pub txid: Option<String>,
    pub created_at: u64,
}

/// Thread-safe SQLite database for pool operations.
#[derive(Clone)]
#[allow(dead_code)] // Reserved for Phase 8 pool integration
pub struct PoolDatabase {
    conn: Arc<Mutex<Connection>>,
}

impl PoolDatabase {
    /// Create or open a pool database at the given path.
    pub fn open(path: &str) -> SqlResult<Self> {
        let conn = Connection::open(path)?;
        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        db.init_schema()?;
        Ok(db)
    }

    /// Create an in-memory database for testing.
    pub fn memory() -> SqlResult<Self> {
        let conn = Connection::open_in_memory()?;
        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        db.init_schema()?;
        Ok(db)
    }

    /// Initialize database schema.
    fn init_schema(&self) -> SqlResult<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute(
            "CREATE TABLE IF NOT EXISTS miners (
                id TEXT PRIMARY KEY,
                total_reward INTEGER NOT NULL DEFAULT 0
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS blocks (
                hash TEXT PRIMARY KEY,
                height INTEGER NOT NULL,
                miner_id TEXT NOT NULL,
                reward INTEGER NOT NULL,
                timestamp INTEGER NOT NULL
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS payouts (
                id TEXT PRIMARY KEY,
                miner_id TEXT NOT NULL,
                amount INTEGER NOT NULL,
                txid TEXT,
                created_at INTEGER NOT NULL
            )",
            [],
        )?;

        // Create indexes for common queries
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_blocks_height ON blocks(height)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_blocks_miner ON blocks(miner_id)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_payouts_miner ON payouts(miner_id)",
            [],
        )?;

        Ok(())
    }

    /// Insert a new block record.
    pub fn insert_block(&self, block: &BlockRecord) -> SqlResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO blocks (hash, height, miner_id, reward, timestamp)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                &block.hash,
                block.height,
                &block.miner_id,
                block.reward,
                block.timestamp
            ],
        )?;
        Ok(())
    }

    /// Get a block by hash.
    pub fn get_block(&self, hash: &str) -> SqlResult<Option<BlockRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT hash, height, miner_id, reward, timestamp FROM blocks WHERE hash = ?1",
        )?;

        let mut rows = stmt.query(params![hash])?;
        if let Some(row) = rows.next()? {
            Ok(Some(BlockRecord {
                hash: row.get(0)?,
                height: row.get(1)?,
                miner_id: row.get(2)?,
                reward: row.get(3)?,
                timestamp: row.get(4)?,
            }))
        } else {
            Ok(None)
        }
    }

    /// Get the latest block (highest height).
    pub fn get_latest_block(&self) -> SqlResult<Option<BlockRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT hash, height, miner_id, reward, timestamp 
             FROM blocks 
             ORDER BY height DESC 
             LIMIT 1",
        )?;

        let mut rows = stmt.query([])?;
        if let Some(row) = rows.next()? {
            Ok(Some(BlockRecord {
                hash: row.get(0)?,
                height: row.get(1)?,
                miner_id: row.get(2)?,
                reward: row.get(3)?,
                timestamp: row.get(4)?,
            }))
        } else {
            Ok(None)
        }
    }

    /// Update miner's total reward (adds to existing amount).
    pub fn update_miner_reward(&self, miner_id: &str, amount: u64) -> SqlResult<()> {
        let conn = self.conn.lock().unwrap();

        // Insert or update miner
        conn.execute(
            "INSERT INTO miners (id, total_reward) VALUES (?1, ?2)
             ON CONFLICT(id) DO UPDATE SET total_reward = total_reward + ?2",
            params![miner_id, amount],
        )?;
        Ok(())
    }

    /// Get miner's total reward.
    pub fn get_miner_reward(&self, miner_id: &str) -> SqlResult<u64> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT total_reward FROM miners WHERE id = ?1")?;

        let mut rows = stmt.query(params![miner_id])?;
        if let Some(row) = rows.next()? {
            Ok(row.get(0)?)
        } else {
            Ok(0)
        }
    }

    /// Get blocks mined by a specific miner.
    pub fn get_miner_blocks(&self, miner_id: &str, limit: usize) -> SqlResult<Vec<BlockRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT hash, height, miner_id, reward, timestamp 
             FROM blocks 
             WHERE miner_id = ?1 
             ORDER BY height DESC 
             LIMIT ?2",
        )?;

        let rows = stmt.query_map(params![miner_id, limit], |row| {
            Ok(BlockRecord {
                hash: row.get(0)?,
                height: row.get(1)?,
                miner_id: row.get(2)?,
                reward: row.get(3)?,
                timestamp: row.get(4)?,
            })
        })?;

        rows.collect()
    }

    /// Insert a payout record.
    pub fn insert_payout(&self, payout: &PayoutRecord) -> SqlResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO payouts (id, miner_id, amount, txid, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                &payout.id,
                &payout.miner_id,
                payout.amount,
                &payout.txid,
                payout.created_at
            ],
        )?;
        Ok(())
    }

    /// List recent payouts.
    pub fn list_payouts(&self, limit: usize) -> SqlResult<Vec<PayoutRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, miner_id, amount, txid, created_at 
             FROM payouts 
             ORDER BY created_at DESC 
             LIMIT ?1",
        )?;

        let rows = stmt.query_map(params![limit], |row| {
            Ok(PayoutRecord {
                id: row.get(0)?,
                miner_id: row.get(1)?,
                amount: row.get(2)?,
                txid: row.get(3)?,
                created_at: row.get(4)?,
            })
        })?;

        rows.collect()
    }

    /// Get total rewards distributed.
    pub fn total_rewards(&self) -> SqlResult<u64> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT COALESCE(SUM(total_reward), 0) FROM miners")?;

        let total: i64 = stmt.query_row([], |row| row.get(0))?;
        Ok(total as u64)
    }

    /// Get total number of miners.
    pub fn miner_count(&self) -> SqlResult<u64> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT COUNT(*) FROM miners WHERE total_reward > 0")?;

        let count: i64 = stmt.query_row([], |row| row.get(0))?;
        Ok(count as u64)
    }

    /// Get total number of blocks.
    pub fn block_count(&self) -> SqlResult<u64> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT COUNT(*) FROM blocks")?;

        let count: i64 = stmt.query_row([], |row| row.get(0))?;
        Ok(count as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_creation() {
        let db = PoolDatabase::memory().unwrap();
        assert_eq!(db.total_rewards().unwrap(), 0);
    }

    #[test]
    fn test_block_insertion_and_retrieval() {
        let db = PoolDatabase::memory().unwrap();

        let block = BlockRecord {
            hash: "abc123".to_string(),
            height: 100,
            miner_id: "miner1".to_string(),
            reward: 5000000000,
            timestamp: 1234567890,
        };

        db.insert_block(&block).unwrap();

        let retrieved = db.get_block("abc123").unwrap().unwrap();
        assert_eq!(retrieved.height, 100);
        assert_eq!(retrieved.miner_id, "miner1");
    }

    #[test]
    fn test_miner_reward_accumulation() {
        let db = PoolDatabase::memory().unwrap();

        db.update_miner_reward("miner1", 1000).unwrap();
        db.update_miner_reward("miner1", 2000).unwrap();

        let total = db.get_miner_reward("miner1").unwrap();
        assert_eq!(total, 3000);
    }

    #[test]
    fn test_total_rewards() {
        let db = PoolDatabase::memory().unwrap();

        db.update_miner_reward("miner1", 1000).unwrap();
        db.update_miner_reward("miner2", 2000).unwrap();

        let total = db.total_rewards().unwrap();
        assert_eq!(total, 3000);
    }
}
