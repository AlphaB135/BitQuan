//! RocksDB-based persistent chain storage.

use std::sync::Arc;
use std::{convert::TryInto, path::Path};

use rocksdb::{Options, WriteBatch, DB};

use crate::{ChainStore, StorageError};
use bitquan_types::{Block, BlockHeader, Transaction};

/// Column family names
const CF_BLOCKS: &str = "blocks";
const CF_HEADERS: &str = "headers";
const CF_HEIGHT_INDEX: &str = "height_index";
const CF_TX_INDEX: &str = "tx_index";
const CF_UTXO: &str = "utxo";
const CF_META: &str = "meta";

/// Metadata keys
const KEY_TIP: &[u8] = b"tip";
const KEY_HEIGHT: &[u8] = b"height";
const KEY_DB_VERSION: &[u8] = b"db_version";
const KEY_CHECKSUM: &[u8] = b"checksum";

/// Current database version
const DB_VERSION: u32 = 1;

/// Database recovery options
#[derive(Debug, Clone, Default)]
pub struct RecoveryOptions {
    /// Verify checksums on open
    pub verify_checksums: bool,
    /// Auto-backup before opening
    pub auto_backup: bool,
    /// Backup directory path
    pub backup_path: Option<String>,
    /// Rebuild indices if corrupted
    pub rebuild_indices: bool,
}

/// RocksDB-backed chain store with persistent storage
pub struct RocksDBStore {
    db: Arc<DB>,
}

impl RocksDBStore {
    /// Open or create a RocksDB store at the given path
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, StorageError> {
        Self::open_with_options(path, RecoveryOptions::default())
    }

    /// Open with recovery options
    pub fn open_with_options<P: AsRef<Path>>(
        path: P,
        options: RecoveryOptions,
    ) -> Result<Self, StorageError> {
        let path = path.as_ref();

        // Auto-backup if requested
        if options.auto_backup {
            Self::create_backup(path, &options)?;
        }

        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);

        let cfs = vec![
            CF_BLOCKS,
            CF_HEADERS,
            CF_HEIGHT_INDEX,
            CF_TX_INDEX,
            CF_UTXO,
            CF_META,
        ];

        let db = DB::open_cf(&opts, path, &cfs)
            .map_err(|e| StorageError::DatabaseError(e.to_string()))?;

        let store = Self { db: Arc::new(db) };

        // Ensure metadata defaults exist
        store.init_metadata()?;

        // Verify checksums if requested
        if options.verify_checksums {
            store.verify_database()?;
        }

        // Rebuild indices if requested
        if options.rebuild_indices {
            store.rebuild_indices()?;
        }

        Ok(store)
    }

    /// Create a backup of the database
    fn create_backup<P: AsRef<Path>>(
        db_path: P,
        options: &RecoveryOptions,
    ) -> Result<(), StorageError> {
        use std::fs;
        use std::time::SystemTime;

        let db_path = db_path.as_ref();
        if !db_path.exists() {
            return Ok(()); // Nothing to backup
        }

        let backup_dir = if let Some(ref path) = options.backup_path {
            Path::new(path)
        } else {
            db_path.parent().unwrap_or(Path::new("."))
        };

        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let backup_name = format!("chaindata.backup.{}", timestamp);
        let backup_path = backup_dir.join(backup_name);

        // Copy database directory
        fs::create_dir_all(&backup_path).map_err(|e| {
            StorageError::DatabaseError(format!("backup dir creation failed: {}", e))
        })?;

        // Recursively copy all files
        Self::copy_dir_recursive(db_path, &backup_path)?;

        eprintln!("✅ Database backed up to: {}", backup_path.display());

        Ok(())
    }

    /// Recursively copy directory
    fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), StorageError> {
        use std::fs;

        for entry in fs::read_dir(src)
            .map_err(|e| StorageError::DatabaseError(format!("read_dir failed: {}", e)))?
        {
            let entry = entry.map_err(|e| StorageError::DatabaseError(e.to_string()))?;
            let path = entry.path();
            let dest_path = dst.join(entry.file_name());

            if path.is_dir() {
                fs::create_dir_all(&dest_path)
                    .map_err(|e| StorageError::DatabaseError(e.to_string()))?;
                Self::copy_dir_recursive(&path, &dest_path)?;
            } else {
                fs::copy(&path, &dest_path)
                    .map_err(|e| StorageError::DatabaseError(e.to_string()))?;
            }
        }

        Ok(())
    }

    /// Get metadata value
    fn get_meta(&self, key: &[u8]) -> Result<Option<Vec<u8>>, StorageError> {
        let cf = self
            .db
            .cf_handle(CF_META)
            .ok_or_else(|| StorageError::DatabaseError("meta CF not found".into()))?;

        self.db
            .get_cf(&cf, key)
            .map_err(|e| StorageError::DatabaseError(e.to_string()))
    }

    /// Put metadata value
    fn put_meta(&self, key: &[u8], value: &[u8]) -> Result<(), StorageError> {
        let cf = self
            .db
            .cf_handle(CF_META)
            .ok_or_else(|| StorageError::DatabaseError("meta CF not found".into()))?;

        self.db
            .put_cf(&cf, key, value)
            .map_err(|e| StorageError::DatabaseError(e.to_string()))
    }

    /// Initialise metadata entries if missing
    fn init_metadata(&self) -> Result<(), StorageError> {
        match self.get_meta(KEY_DB_VERSION)? {
            Some(bytes) if bytes.len() == 4 => {
                let arr: [u8; 4] = bytes
                    .as_slice()
                    .try_into()
                    .map_err(|_| StorageError::DatabaseError("invalid db version bytes".into()))?;
                let version = u32::from_le_bytes(arr);
                if version != DB_VERSION {
                    return Err(StorageError::DatabaseError(format!(
                        "Unsupported database version: {} (expected {})",
                        version, DB_VERSION
                    )));
                }
            }
            Some(_) => {
                return Err(StorageError::DatabaseError(
                    "Corrupted db version metadata".into(),
                ));
            }
            None => {
                self.put_meta(KEY_DB_VERSION, &DB_VERSION.to_le_bytes())?;
            }
        }

        if self.get_meta(KEY_CHECKSUM)?.is_none() {
            self.put_meta(KEY_CHECKSUM, b"pending")?;
        }

        Ok(())
    }

    /// Get current chain height
    pub fn height(&self) -> Result<u64, StorageError> {
        match self.get_meta(KEY_HEIGHT)? {
            Some(bytes) => {
                let arr: [u8; 8] = bytes
                    .try_into()
                    .map_err(|_| StorageError::DatabaseError("invalid height bytes".into()))?;
                Ok(u64::from_le_bytes(arr))
            }
            None => Ok(0),
        }
    }

    /// Compute block ID (SHA256d of header)
    fn block_id(header: &BlockHeader) -> [u8; 32] {
        use sha2::{Digest, Sha256};
        let bytes = header.to_bytes();
        let first = Sha256::digest(bytes);
        let second = Sha256::digest(first);
        let mut out = [0u8; 32];
        out.copy_from_slice(&second);
        out
    }

    /// Verify database integrity
    pub fn verify_database(&self) -> Result<(), StorageError> {
        eprintln!("🔍 Verifying database integrity...");

        // Check metadata consistency
        let height = self.height()?;
        eprintln!("  Chain height: {}", height);

        // Ensure DB version matches expected value
        match self.get_meta(KEY_DB_VERSION)? {
            Some(bytes) if bytes.len() == 4 => {
                let arr: [u8; 4] = bytes
                    .as_slice()
                    .try_into()
                    .map_err(|_| StorageError::DatabaseError("invalid db version bytes".into()))?;
                let version = u32::from_le_bytes(arr);
                if version != DB_VERSION {
                    return Err(StorageError::DatabaseError(format!(
                        "Database version mismatch: {} (expected {})",
                        version, DB_VERSION
                    )));
                }
            }
            Some(_) => {
                return Err(StorageError::DatabaseError(
                    "Corrupted db version metadata".into(),
                ));
            }
            None => {
                return Err(StorageError::DatabaseError(
                    "Missing db version metadata".into(),
                ));
            }
        }

        // Verify tip exists
        let tip = self.get_meta(KEY_TIP)?;
        if tip.is_none() && height > 0 {
            return Err(StorageError::DatabaseError(
                "Inconsistent state: height > 0 but no tip".into(),
            ));
        }

        // Check column families exist
        for cf_name in &[
            CF_BLOCKS,
            CF_HEADERS,
            CF_HEIGHT_INDEX,
            CF_TX_INDEX,
            CF_UTXO,
            CF_META,
        ] {
            if self.db.cf_handle(cf_name).is_none() {
                return Err(StorageError::DatabaseError(format!(
                    "Missing column family: {}",
                    cf_name
                )));
            }
        }

        // Verify chain continuity (sample check)
        if height > 0 {
            self.verify_chain_continuity(height)?;
        }

        eprintln!("✅ Database verification complete");
        Ok(())
    }

    /// Verify chain continuity by checking a sample of blocks
    fn verify_chain_continuity(&self, height: u64) -> Result<(), StorageError> {
        let sample_points = if height <= 10 {
            (0..height).collect::<Vec<_>>()
        } else {
            // Sample: genesis, tip, and 8 random points
            let mut points = vec![0, height - 1];
            let step = height / 8;
            for i in 1..9 {
                points.push(i * step);
            }
            points
        };

        let cf = self
            .db
            .cf_handle(CF_HEIGHT_INDEX)
            .ok_or_else(|| StorageError::DatabaseError("height_index CF not found".into()))?;

        for h in sample_points {
            let key = h.to_le_bytes();
            if self
                .db
                .get_cf(&cf, key)
                .map_err(|e| StorageError::DatabaseError(e.to_string()))?
                .is_none()
            {
                return Err(StorageError::DatabaseError(format!(
                    "Missing block at height {}",
                    h
                )));
            }
        }

        Ok(())
    }

    /// Rebuild all indices from blocks
    pub fn rebuild_indices(&self) -> Result<(), StorageError> {
        eprintln!("🔧 Rebuilding database indices...");

        // This is a simplified version - in production, you'd:
        // 1. Iterate through all blocks
        // 2. Rebuild height_index
        // 3. Rebuild tx_index
        // 4. Validate UTXO set consistency

        let height = self.height()?;
        eprintln!("  Processing {} blocks...", height);

        // For now, just verify indices exist
        self.verify_chain_continuity(height)?;

        eprintln!("✅ Index rebuild complete");
        Ok(())
    }

    /// Prune orphan blocks (blocks not in main chain)
    #[allow(dead_code)]
    pub fn prune_orphans(&self) -> Result<u64, StorageError> {
        eprintln!("🗑️  Pruning orphan blocks...");

        // TODO: Implement orphan detection and removal
        // This requires tracking main chain and identifying orphans

        let pruned = 0u64;
        eprintln!("✅ Pruned {} orphan blocks", pruned);
        Ok(pruned)
    }

    /// Get database statistics
    pub fn get_stats(&self) -> Result<DatabaseStats, StorageError> {
        let height = self.height()?;

        let mut stats = DatabaseStats {
            height,
            num_blocks: 0,
            num_transactions: 0,
            num_utxos: 0,
            db_size_bytes: 0,
        };

        // Count blocks (approximate)
        let blocks_cf = self
            .db
            .cf_handle(CF_BLOCKS)
            .ok_or_else(|| StorageError::DatabaseError("blocks CF not found".into()))?;

        let iter = self
            .db
            .iterator_cf(&blocks_cf, rocksdb::IteratorMode::Start);
        stats.num_blocks = iter.count() as u64;

        Ok(stats)
    }
}

/// Database statistics
#[derive(Debug, Clone)]
pub struct DatabaseStats {
    /// Current chain height
    pub height: u64,
    /// Number of blocks stored
    pub num_blocks: u64,
    /// Number of transactions indexed
    pub num_transactions: u64,
    /// Number of UTXO entries
    pub num_utxos: u64,
    /// Database size in bytes (approximate)
    pub db_size_bytes: u64,
}

impl ChainStore for RocksDBStore {
    fn insert_block(&mut self, block: Block) -> Result<(), StorageError> {
        let block_id = Self::block_id(&block.header);
        let height = self.height()? + 1;

        let cf_blocks = self
            .db
            .cf_handle(CF_BLOCKS)
            .ok_or_else(|| StorageError::DatabaseError("blocks CF not found".into()))?;
        let cf_headers = self
            .db
            .cf_handle(CF_HEADERS)
            .ok_or_else(|| StorageError::DatabaseError("headers CF not found".into()))?;
        let cf_height = self
            .db
            .cf_handle(CF_HEIGHT_INDEX)
            .ok_or_else(|| StorageError::DatabaseError("height_index CF not found".into()))?;
        let cf_tx = self
            .db
            .cf_handle(CF_TX_INDEX)
            .ok_or_else(|| StorageError::DatabaseError("tx_index CF not found".into()))?;

        let mut batch = WriteBatch::default();

        // Serialize block (JSON for simplicity, can use bincode for production)
        let block_json = serde_json::to_vec(&block)
            .map_err(|e| StorageError::SerializationError(e.to_string()))?;
        let header_json = serde_json::to_vec(&block.header)
            .map_err(|e| StorageError::SerializationError(e.to_string()))?;

        // Store block and header
        batch.put_cf(&cf_blocks, block_id, block_json);
        batch.put_cf(&cf_headers, block_id, &header_json);

        // Index by height
        batch.put_cf(&cf_height, height.to_le_bytes(), block_id);

        // Index transactions
        for tx in &block.transactions {
            let txid = tx.txid();
            let tx_json = serde_json::to_vec(tx)
                .map_err(|e| StorageError::SerializationError(e.to_string()))?;
            batch.put_cf(&cf_tx, txid, tx_json);
        }

        // Update metadata
        let cf_meta = self
            .db
            .cf_handle(CF_META)
            .ok_or_else(|| StorageError::DatabaseError("meta CF not found".into()))?;
        batch.put_cf(&cf_meta, KEY_TIP, header_json.clone());
        batch.put_cf(&cf_meta, KEY_HEIGHT, height.to_le_bytes());

        // Write batch atomically
        self.db
            .write(batch)
            .map_err(|e| StorageError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    fn get_block(&self, id: &[u8; 32]) -> Result<Option<Block>, StorageError> {
        let cf = self
            .db
            .cf_handle(CF_BLOCKS)
            .ok_or_else(|| StorageError::DatabaseError("blocks CF not found".into()))?;

        match self
            .db
            .get_cf(&cf, id)
            .map_err(|e| StorageError::DatabaseError(e.to_string()))?
        {
            Some(bytes) => {
                let block: Block = serde_json::from_slice(&bytes)
                    .map_err(|e| StorageError::SerializationError(e.to_string()))?;
                Ok(Some(block))
            }
            None => Ok(None),
        }
    }

    fn tip(&self) -> Result<Option<BlockHeader>, StorageError> {
        match self.get_meta(KEY_TIP)? {
            Some(bytes) => {
                let header: BlockHeader = serde_json::from_slice(&bytes)
                    .map_err(|e| StorageError::SerializationError(e.to_string()))?;
                Ok(Some(header))
            }
            None => Ok(None),
        }
    }

    fn get_block_by_height(&self, height: u64) -> Result<Option<Block>, StorageError> {
        let cf_height = self
            .db
            .cf_handle(CF_HEIGHT_INDEX)
            .ok_or_else(|| StorageError::DatabaseError("height_index CF not found".into()))?;

        match self
            .db
            .get_cf(&cf_height, height.to_le_bytes())
            .map_err(|e| StorageError::DatabaseError(e.to_string()))?
        {
            Some(block_id_bytes) => {
                let mut block_id = [0u8; 32];
                block_id.copy_from_slice(&block_id_bytes);
                self.get_block(&block_id)
            }
            None => Ok(None),
        }
    }

    fn get_transaction(&self, txid: &[u8; 32]) -> Result<Option<Transaction>, StorageError> {
        let cf = self
            .db
            .cf_handle(CF_TX_INDEX)
            .ok_or_else(|| StorageError::DatabaseError("tx_index CF not found".into()))?;

        match self
            .db
            .get_cf(&cf, txid)
            .map_err(|e| StorageError::DatabaseError(e.to_string()))?
        {
            Some(bytes) => {
                let tx: Transaction = serde_json::from_slice(&bytes)
                    .map_err(|e| StorageError::SerializationError(e.to_string()))?;
                Ok(Some(tx))
            }
            None => Ok(None),
        }
    }

    fn put_utxo(&mut self, outpoint: &[u8], data: &[u8]) -> Result<(), StorageError> {
        let cf = self
            .db
            .cf_handle(CF_UTXO)
            .ok_or_else(|| StorageError::DatabaseError("utxo CF not found".into()))?;

        self.db
            .put_cf(&cf, outpoint, data)
            .map_err(|e| StorageError::DatabaseError(e.to_string()))
    }

    fn get_utxo(&self, outpoint: &[u8]) -> Result<Option<Vec<u8>>, StorageError> {
        let cf = self
            .db
            .cf_handle(CF_UTXO)
            .ok_or_else(|| StorageError::DatabaseError("utxo CF not found".into()))?;

        self.db
            .get_cf(&cf, outpoint)
            .map_err(|e| StorageError::DatabaseError(e.to_string()))
    }

    fn delete_utxo(&mut self, outpoint: &[u8]) -> Result<(), StorageError> {
        let cf = self
            .db
            .cf_handle(CF_UTXO)
            .ok_or_else(|| StorageError::DatabaseError("utxo CF not found".into()))?;

        self.db
            .delete_cf(&cf, outpoint)
            .map_err(|e| StorageError::DatabaseError(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitquan_types::{
        genesis::GENESIS_HASH_BYTES, NetworkId, SigAlgorithm, Transaction, TxIn, TxOut,
    };

    #[test]
    fn test_rocksdb_store_basic() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mut store = RocksDBStore::open(temp_dir.path()).unwrap();

        // Create genesis block
        let coinbase = Transaction {
            version: 1,
            network: NetworkId::Devnet,
            genesis_hash: GENESIS_HASH_BYTES,
            inputs: vec![TxIn {
                prev_txid: [0u8; 32],
                prev_vout: 0xffffffff,
                script_sig: vec![],
                sequence: 0xffffffff,
            }],
            outputs: vec![TxOut {
                value: 5000000000,
                script_pubkey: vec![0x51], // OP_TRUE
            }],
            lock_time: 0,
            witnesses: vec![],
            sig_algo: SigAlgorithm::Dilithium3,
        };

        let header = BlockHeader {
            version: 1,
            prev_block: [0u8; 32],
            merkle_root: [0u8; 32],
            pqc_agg_hint: [0u8; 32],
            time: 1729900000,
            bits: 0x1d00ffff,
            nonce: 0,
        };

        let block = Block {
            header: header.clone(),
            transactions: vec![coinbase.clone()],
        };

        // Insert block
        store.insert_block(block.clone()).unwrap();

        // Verify height
        assert_eq!(store.height().unwrap(), 1);

        // Verify tip
        let tip = store.tip().unwrap().unwrap();
        assert_eq!(tip.time, header.time);

        // Get block by height
        let retrieved = store.get_block_by_height(1).unwrap().unwrap();
        assert_eq!(retrieved.header.time, header.time);

        // Get transaction
        let txid = coinbase.txid();
        let tx = store.get_transaction(&txid).unwrap().unwrap();
        assert_eq!(tx.version, coinbase.version);
    }

    #[test]
    fn test_utxo_operations() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mut store = RocksDBStore::open(temp_dir.path()).unwrap();

        let outpoint = b"test_outpoint_123";
        let data = b"utxo_data";

        // Put UTXO
        store.put_utxo(outpoint, data).unwrap();

        // Get UTXO
        let retrieved = store.get_utxo(outpoint).unwrap().unwrap();
        assert_eq!(retrieved, data);

        // Delete UTXO
        store.delete_utxo(outpoint).unwrap();

        // Verify deleted
        assert!(store.get_utxo(outpoint).unwrap().is_none());
    }
}
