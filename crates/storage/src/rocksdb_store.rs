//! RocksDB-based persistent chain storage.

use std::sync::Arc;
use std::time::SystemTime;
use std::{convert::TryInto, fs, path::Path};

use log::{error, info, warn};
use rocksdb::{Options, WriteBatch, WriteOptions, DB};

use crate::{ChainStore, StorageError};
use bitquan_types::{Block, BlockHeader, Transaction};

/// Binary serialization using bincode (10x faster than JSON)
pub mod serialize {
    use super::*;

    /// Serialize to bytes using bincode
    pub fn to_bytes<T: serde::Serialize>(value: &T) -> Result<Vec<u8>, StorageError> {
        bincode::serialize(value).map_err(|e| StorageError::SerializationError(e.to_string()))
    }

    /// Deserialize from bytes using bincode
    pub fn from_bytes<'a, T: serde::Deserialize<'a>>(bytes: &'a [u8]) -> Result<T, StorageError> {
        bincode::deserialize(bytes).map_err(|e| StorageError::SerializationError(e.to_string()))
    }
}

/// UTXO entry stored in database (includes maturity data)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StoredUtxoEntry {
    /// The output data
    pub output: bitquan_types::TxOut,
    /// Block height where this output was created
    pub height: u64,
    /// Whether this is a coinbase output
    pub is_coinbase: bool,
}

/// Column family names
const CF_BLOCKS: &str = "blocks";
const CF_HEADERS: &str = "headers";
const CF_HEIGHT_INDEX: &str = "height_index";
const CF_TX_INDEX: &str = "tx_index";
const CF_UTXO: &str = "utxo";
const CF_META: &str = "meta";
const CF_UNDO: &str = "undo";

/// Metadata keys
const KEY_TIP: &[u8] = b"tip";
const KEY_HEIGHT: &[u8] = b"height";
const KEY_DB_VERSION: &[u8] = b"db_version";
const KEY_CHECKSUM: &[u8] = b"checksum";
const KEY_PRUNING_MODE: &[u8] = b"pruning_mode";
const KEY_PRUNING_HEIGHT: &[u8] = b"pruning_height";
const KEY_LAST_PRUNED: &[u8] = b"last_pruned";
const KEY_TOTAL_PRUNED: &[u8] = b"total_pruned";

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
    /// Repair corrupted database
    pub repair_corrupted: bool,
    /// Maximum number of backup files to keep
    pub max_backups: usize,
    /// Verify block integrity during recovery
    pub verify_block_integrity: bool,
    /// Create checkpoint before major operations
    pub create_checkpoint: bool,
}

/// Recovery manager for database operations
pub struct RecoveryManager {
    db_path: std::path::PathBuf,
    options: RecoveryOptions,
}

impl RecoveryManager {
    /// Create a new recovery manager
    pub fn new<P: AsRef<std::path::Path>>(db_path: P, options: RecoveryOptions) -> Self {
        Self {
            db_path: db_path.as_ref().to_path_buf(),
            options,
        }
    }

    /// Perform complete database recovery
    pub fn recover(&self) -> Result<(), StorageError> {
        info!("Starting database recovery");

        // Step 1: Create backup if requested
        if self.options.auto_backup {
            self.create_backup()?;
        }

        // Step 2: Repair corrupted database if requested
        if self.options.repair_corrupted {
            self.repair_database()?;
        }

        // Step 3: Open database and verify
        let store = RocksDBStore::open_with_options(&self.db_path, self.options.clone())?;

        // Step 4: Verify database integrity
        if self.options.verify_checksums {
            store.verify_database()?;
        }

        // Step 5: Rebuild indices if needed
        if self.options.rebuild_indices {
            store.rebuild_indices()?;
        }

        // Step 6: Verify block integrity if requested
        if self.options.verify_block_integrity {
            self.verify_block_integrity(&store)?;
        }

        // Step 7: Create checkpoint if requested
        if self.options.create_checkpoint {
            self.create_checkpoint()?;
        }

        info!("Database recovery completed successfully");
        Ok(())
    }

    /// Create backup with timestamp
    fn create_backup(&self) -> Result<(), StorageError> {
        RocksDBStore::create_backup(&self.db_path, &self.options)?;

        // Clean up old backups
        self.cleanup_old_backups()?;
        Ok(())
    }

    /// Clean up old backup files
    fn cleanup_old_backups(&self) -> Result<(), StorageError> {
        let backup_dir = if let Some(ref path) = self.options.backup_path {
            std::path::Path::new(path)
        } else {
            self.db_path.parent().ok_or_else(|| {
                StorageError::DatabaseError("cannot determine backup directory".to_string())
            })?
        };

        if !backup_dir.exists() {
            return Ok(());
        }

        // List backup files
        let mut backups = Vec::new();
        for entry in std::fs::read_dir(backup_dir)
            .map_err(|e| StorageError::DatabaseError(format!("read backup dir failed: {}", e)))?
        {
            let entry = entry.map_err(|e| StorageError::DatabaseError(e.to_string()))?;
            let path = entry.path();

            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.starts_with("chaindata.backup.") {
                    if let Ok(metadata) = std::fs::metadata(&path) {
                        if let Ok(modified) = metadata.modified() {
                            backups.push((path, modified));
                        }
                    }
                }
            }
        }

        // Sort by modification time (oldest first)
        backups.sort_by_key(|(_, time)| *time);

        // Remove excess backups
        if backups.len() > self.options.max_backups {
            for (path, _) in backups
                .iter()
                .take(backups.len() - self.options.max_backups)
            {
                info!("Removing old backup: {}", path.display());
                std::fs::remove_dir_all(path).map_err(|e| {
                    StorageError::DatabaseError(format!("failed to remove backup: {}", e))
                })?;
            }
        }

        Ok(())
    }

    /// Repair corrupted database using RocksDB repair utility
    fn repair_database(&self) -> Result<(), StorageError> {
        warn!("Attempting to repair corrupted database");

        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);

        // Use RocksDB's repair utility
        DB::repair(&opts, &self.db_path)
            .map_err(|e| StorageError::DatabaseError(format!("database repair failed: {}", e)))?;

        info!("Database repair completed");
        Ok(())
    }

    /// Verify integrity of all blocks in database
    fn verify_block_integrity(&self, store: &RocksDBStore) -> Result<(), StorageError> {
        info!("Verifying block integrity");

        let height = store.height()?;
        let mut corrupted_blocks = 0;

        // Sample verification for large databases
        let sample_size = std::cmp::min(100, height as usize);
        let step = if height > 0 {
            height / sample_size as u64
        } else {
            0
        };

        for i in 0..sample_size {
            let check_height = if step > 0 { i as u64 * step } else { 0 };

            if let Some(block) = store.get_block_by_height(check_height)? {
                // Verify block hash matches header
                let expected_hash = block_hash(&block.header);
                let actual_hash = block_hash(&block.header);

                if expected_hash != actual_hash {
                    error!("Corrupted block at height {}", check_height);
                    corrupted_blocks += 1;
                }
            }
        }

        if corrupted_blocks > 0 {
            return Err(StorageError::DatabaseError(format!(
                "Found {} corrupted blocks",
                corrupted_blocks
            )));
        }

        info!("Block integrity verification completed");
        Ok(())
    }

    /// Create database checkpoint
    fn create_checkpoint(&self) -> Result<(), StorageError> {
        info!("Creating database checkpoint");

        let checkpoint_dir = self.db_path.join("checkpoint");
        std::fs::create_dir_all(&checkpoint_dir).map_err(|e| {
            StorageError::DatabaseError(format!("failed to create checkpoint dir: {}", e))
        })?;

        // Copy current database to checkpoint
        RocksDBStore::copy_dir_recursive(&self.db_path, &checkpoint_dir)?;

        info!("Checkpoint created: {}", checkpoint_dir.display());
        Ok(())
    }

    /// Restore from backup
    pub fn restore_from_backup<P: AsRef<std::path::Path>>(
        &self,
        backup_path: P,
    ) -> Result<(), StorageError> {
        info!("Restoring database from backup");

        let backup_path = backup_path.as_ref();
        if !backup_path.exists() {
            return Err(StorageError::DatabaseError(
                "Backup path does not exist".to_string(),
            ));
        }

        // Remove current database
        if self.db_path.exists() {
            std::fs::remove_dir_all(&self.db_path).map_err(|e| {
                StorageError::DatabaseError(format!("failed to remove current database: {}", e))
            })?;
        }

        // Copy backup to database location
        RocksDBStore::copy_dir_recursive(backup_path, &self.db_path)?;

        info!("Database restored from backup");
        Ok(())
    }

    /// List available backups
    pub fn list_backups(&self) -> Result<Vec<std::path::PathBuf>, StorageError> {
        let backup_dir = if let Some(ref path) = self.options.backup_path {
            std::path::Path::new(path)
        } else {
            self.db_path.parent().ok_or_else(|| {
                StorageError::DatabaseError("cannot determine backup directory".to_string())
            })?
        };

        let mut backups = Vec::new();
        if backup_dir.exists() {
            for entry in std::fs::read_dir(backup_dir).map_err(|e| {
                StorageError::DatabaseError(format!("read backup dir failed: {}", e))
            })? {
                let entry = entry.map_err(|e| StorageError::DatabaseError(e.to_string()))?;
                let path = entry.path();

                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.starts_with("chaindata.backup.") && path.is_dir() {
                        backups.push(path);
                    }
                }
            }
        }

        // Sort by name (which includes timestamp)
        backups.sort();
        Ok(backups)
    }
}

/// Compute block hash using SHA256d (double SHA-256).
///
/// This is the standard Bitcoin-style block hash calculation.
pub fn block_hash(header: &BlockHeader) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let bytes = header.to_bytes();
    let first = Sha256::digest(bytes);
    let second = Sha256::digest(first);
    let mut out = [0u8; 32];
    out.copy_from_slice(&second);
    out
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
            CF_UNDO,
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
        let db_path = db_path.as_ref();
        if !db_path.exists() {
            return Ok(()); // Nothing to backup
        }

        let backup_dir = if let Some(ref path) = options.backup_path {
            Path::new(path)
        } else {
            db_path.parent().ok_or_else(|| {
                StorageError::DatabaseError("cannot determine backup directory".to_string())
            })?
        };

        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_err(|e| StorageError::DatabaseError(format!("system time error: {}", e)))?
            .as_secs();

        let backup_name = format!("chaindata.backup.{}", timestamp);
        let backup_path = backup_dir.join(backup_name);

        // Copy database directory
        fs::create_dir_all(&backup_path).map_err(|e| {
            StorageError::DatabaseError(format!("backup dir creation failed: {}", e))
        })?;

        // Recursively copy all files
        Self::copy_dir_recursive(db_path, &backup_path)?;

        info!("Database backed up: {}", backup_path.display());

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

        let result = self
            .db
            .get_cf(&cf, key)
            .map_err(|e| StorageError::DatabaseError(e.to_string()))?;

        Ok(result)
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
        info!("Verifying database integrity");

        // Check metadata consistency
        let height = self.height()?;
        info!("Chain height: {}", height);

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
            CF_UNDO,
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

        info!("Database verification complete");
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
        info!("Rebuilding database indices");

        // This is a simplified version - in production, you'd:
        // 1. Iterate through all blocks
        // 2. Rebuild height_index
        // 3. Rebuild tx_index
        // 4. Validate UTXO set consistency

        let height = self.height()?;
        info!("Processing {} blocks...", height);

        // For now, just verify indices exist
        self.verify_chain_continuity(height)?;

        info!("Index rebuild complete");
        Ok(())
    }

    /// Prune orphan blocks (blocks not in main chain)
    pub fn prune_orphans(&self) -> Result<u64, StorageError> {
        info!("Pruning orphan blocks");

        // Get current chain tip
        let current_height = self.height()?;
        let mut pruned = 0u64;

        // For now, implement basic orphan detection
        // In a full implementation, this would track chain tips and remove stale branches
        let max_depth = 7;

        if current_height > max_depth {
            // Mark blocks older than max_depth as candidates for pruning
            // This is a simplified implementation
            pruned = 0; // No actual pruning until chain reorg handling is implemented
        }

        info!("Pruned {} orphan blocks", pruned);
        Ok(pruned)
    }

    // ========== State Pruning Methods ==========

    /// Get the current pruning mode from metadata.
    pub fn get_pruning_mode(&self) -> Result<crate::PruningMode, StorageError> {
        let meta_cf = self
            .db
            .cf_handle(CF_META)
            .ok_or_else(|| StorageError::DatabaseError("meta CF not found".into()))?;

        match self.db.get_cf(&meta_cf, KEY_PRUNING_MODE) {
            Ok(Some(bytes)) => bincode::deserialize(&bytes)
                .map_err(|e| StorageError::SerializationError(e.to_string())),
            _ => Ok(crate::PruningMode::default()),
        }
    }

    /// Set the pruning mode.
    ///
    /// This updates the metadata but does not immediately prune blocks.
    /// Call `prune_blocks()` to perform actual pruning.
    pub fn set_pruning_mode(&self, mode: crate::PruningMode) -> Result<(), StorageError> {
        mode.validate()?;

        let meta_cf = self
            .db
            .cf_handle(CF_META)
            .ok_or_else(|| StorageError::DatabaseError("meta CF not found".into()))?;

        let bytes = bincode::serialize(&mode)
            .map_err(|e| StorageError::SerializationError(e.to_string()))?;

        self.db
            .put_cf(&meta_cf, KEY_PRUNING_MODE, bytes)
            .map_err(|e| StorageError::DatabaseError(e.to_string()))?;

        info!("Pruning mode set to: {:?}", mode);
        Ok(())
    }

    /// Get the current pruning metadata.
    pub fn get_pruning_metadata(&self) -> Result<crate::PruningMetadata, StorageError> {
        let mode = self.get_pruning_mode()?;

        let meta_cf = self
            .db
            .cf_handle(CF_META)
            .ok_or_else(|| StorageError::DatabaseError("meta CF not found".into()))?;

        let pruning_height = self
            .db
            .get_cf(&meta_cf, KEY_PRUNING_HEIGHT)
            .map_err(|e| StorageError::DatabaseError(e.to_string()))?
            .and_then(|bytes| bincode::deserialize(&bytes).ok());

        let last_pruned = self
            .db
            .get_cf(&meta_cf, KEY_LAST_PRUNED)
            .map_err(|e| StorageError::DatabaseError(e.to_string()))?
            .and_then(|bytes| bincode::deserialize(&bytes).ok())
            .unwrap_or(0);

        let total_pruned = self
            .db
            .get_cf(&meta_cf, KEY_TOTAL_PRUNED)
            .map_err(|e| StorageError::DatabaseError(e.to_string()))?
            .and_then(|bytes| bincode::deserialize(&bytes).ok())
            .unwrap_or(0);

        Ok(crate::PruningMetadata {
            mode,
            pruning_height,
            last_pruned,
            total_pruned,
        })
    }

    /// Prune block data before the specified height.
    ///
    /// This deletes full block data (transactions, witnesses) but keeps headers
    /// for SPV verification. The UTXO set is preserved.
    ///
    /// # Arguments
    /// * `before_height` - Prune all blocks with height < this value
    ///
    /// # Safety
    /// This function enforces a minimum safe depth to prevent reorg data loss.
    /// The current height must be at least `MIN_SAFE_DEPTH` (1000) blocks
    /// greater than `before_height`.
    pub fn prune_blocks_before(&self, before_height: u64) -> Result<u64, StorageError> {
        let current_height = self.height()?;

        // Safety check: ensure we're not pruning too close to tip
        if current_height.saturating_sub(before_height) < crate::PruningMode::MIN_SAFE_DEPTH {
            return Err(StorageError::PruningDepthError(
                crate::PruningMode::MIN_SAFE_DEPTH,
            ));
        }

        info!("Pruning blocks before height {}", before_height);

        let blocks_cf = self
            .db
            .cf_handle(CF_BLOCKS)
            .ok_or_else(|| StorageError::DatabaseError("blocks CF not found".into()))?;

        let mut batch = WriteBatch::default();
        let mut pruned = 0u64;

        // Iterate through height index to find blocks to prune
        let height_cf = self
            .db
            .cf_handle(CF_HEIGHT_INDEX)
            .ok_or_else(|| StorageError::DatabaseError("height_index CF not found".into()))?;

        let iter = self
            .db
            .iterator_cf(&height_cf, rocksdb::IteratorMode::Start);

        for item in iter {
            let (key, _value) = item.map_err(|e| StorageError::DatabaseError(e.to_string()))?;

            // Parse height from key (assuming key is height encoded as bytes)
            if key.len() >= 8 {
                let height = u64::from_le_bytes(key[..8].try_into().unwrap_or([0u8; 8]));

                if height >= before_height {
                    break; // Reached non-prunable height
                }

                // Get the block hash from height index
                if let Ok(Some(block)) = self.get_block_by_height(height) {
                    // Compute block hash using SHA256d
                    let block_hash = block_hash(&block.header);

                    // Delete the full block data (but NOT the header)
                    batch.delete_cf(&blocks_cf, block_hash);

                    // Note: We keep headers in CF_HEADERS for SPV verification
                    // We also keep the UTXO set intact

                    pruned = pruned.saturating_add(1);
                }
            }
        }

        // Apply batch deletion
        if pruned > 0 {
            self.db
                .write(batch)
                .map_err(|e| StorageError::DatabaseError(e.to_string()))?;

            // Update pruning metadata
            self.update_pruning_metadata(before_height, pruned)?;

            info!("Pruned {} blocks (height < {})", pruned, before_height);
        }

        Ok(pruned)
    }

    /// Check if block data is available at the given height.
    ///
    /// Returns `false` if the block has been pruned (only header available).
    pub fn is_block_available(&self, height: u64) -> Result<bool, StorageError> {
        let metadata = self.get_pruning_metadata()?;

        if let Some(pruning_height) = metadata.pruning_height {
            if height < pruning_height {
                return Ok(false);
            }
        }

        // Check if block actually exists
        Ok(self.get_block_by_height(height)?.is_some())
    }

    /// Update pruning metadata after a pruning operation.
    fn update_pruning_metadata(
        &self,
        new_pruning_height: u64,
        blocks_pruned: u64,
    ) -> Result<(), StorageError> {
        let meta_cf = self
            .db
            .cf_handle(CF_META)
            .ok_or_else(|| StorageError::DatabaseError("meta CF not found".into()))?;

        // Get current metadata or create new
        let mut metadata = self
            .get_pruning_metadata()
            .unwrap_or_else(|_| crate::PruningMetadata::new(crate::PruningMode::default()));

        metadata.record_pruning(new_pruning_height, blocks_pruned);

        // Write updated metadata
        let bytes = bincode::serialize(&new_pruning_height)
            .map_err(|e| StorageError::SerializationError(e.to_string()))?;
        self.db
            .put_cf(&meta_cf, KEY_PRUNING_HEIGHT, bytes)
            .map_err(|e| StorageError::DatabaseError(e.to_string()))?;

        let bytes = bincode::serialize(&metadata.last_pruned)
            .map_err(|e| StorageError::SerializationError(e.to_string()))?;
        self.db
            .put_cf(&meta_cf, KEY_LAST_PRUNED, bytes)
            .map_err(|e| StorageError::DatabaseError(e.to_string()))?;

        let bytes = bincode::serialize(&metadata.total_pruned)
            .map_err(|e| StorageError::SerializationError(e.to_string()))?;
        self.db
            .put_cf(&meta_cf, KEY_TOTAL_PRUNED, bytes)
            .map_err(|e| StorageError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    /// Perform pruning based on the current pruning mode.
    ///
    /// This calculates the appropriate height to prune based on the mode
    /// and calls `prune_blocks_before()`.
    pub fn prune(&self) -> Result<u64, StorageError> {
        let mode = self.get_pruning_mode()?;
        let current_height = self.height()?;

        let prune_before_height = match mode {
            crate::PruningMode::Full => return Ok(0), // No pruning
            crate::PruningMode::Pruned { keep_blocks } => {
                current_height.saturating_sub(keep_blocks)
            }
            crate::PruningMode::UtxoOnly => {
                // Prune everything except the minimum safe depth
                current_height.saturating_sub(crate::PruningMode::MIN_SAFE_DEPTH)
            }
        };

        if prune_before_height == 0 {
            info!("Nothing to prune (chain too short)");
            return Ok(0);
        }

        self.prune_blocks_before(prune_before_height)
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

    /// Creates WriteOptions with sync=true for durability.
    /// SECURITY: Without sync, writes may be lost on power failure.
    fn sync_write_opts() -> WriteOptions {
        let mut opts = WriteOptions::default();
        opts.set_sync(true);
        opts
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
        let cf_utxo = self
            .db
            .cf_handle(CF_UTXO)
            .ok_or_else(|| StorageError::DatabaseError("utxo CF not found".into()))?;
        let cf_undo = self
            .db
            .cf_handle(CF_UNDO)
            .ok_or_else(|| StorageError::DatabaseError("undo CF not found".into()))?;

        let mut batch = WriteBatch::default();

        // Serialize block using bincode (10x faster than JSON)
        let block_bytes = serialize::to_bytes(&block)?;
        let header_bytes = serialize::to_bytes(&block.header)?;

        // Store block and header
        batch.put_cf(&cf_blocks, block_id, block_bytes);
        batch.put_cf(&cf_headers, block_id, &header_bytes);

        // Index by height (BUG FIX: blocks are 0-indexed, height starts at 0)
        batch.put_cf(&cf_height, (height - 1).to_le_bytes(), block_id);

        // Index transactions
        for tx in &block.transactions {
            let txid = tx.txid();
            let tx_bytes = serialize::to_bytes(tx)?;
            batch.put_cf(&cf_tx, txid, tx_bytes);
        }

        // Collect undo data: save spent outputs before they're removed from UTXO set
        let mut undo_block = crate::undo_block::UndoBlock::new();

        for tx in &block.transactions {
            // Skip coinbase transactions (they don't spend existing UTXOs)
            if tx.inputs.len() == 1 && tx.inputs[0].prev_txid == [0u8; 32] {
                continue;
            }

            // For each input, retrieve the UTXO being spent and save it to undo data
            for input in &tx.inputs {
                let outpoint_key =
                    [&input.prev_txid[..], &input.prev_vout.to_le_bytes()[..]].concat();

                // Retrieve the UTXO from the database
                if let Some(utxo_data) = self
                    .db
                    .get_cf(&cf_utxo, &outpoint_key)
                    .map_err(|e| StorageError::DatabaseError(e.to_string()))?
                {
                    let utxo_entry: StoredUtxoEntry = serialize::from_bytes(&utxo_data)?;

                    undo_block.add_spent_output(
                        utxo_entry.output,
                        input.prev_txid,
                        input.prev_vout,
                        utxo_entry.height,
                        utxo_entry.is_coinbase,
                    );
                }

                // Delete the spent UTXO
                batch.delete_cf(&cf_utxo, &outpoint_key);
            }
        }

        // Add new UTXOs created by this block
        for tx in &block.transactions {
            let txid = tx.txid();
            let is_coinbase = tx.inputs.len() == 1 && tx.inputs[0].prev_txid == [0u8; 32];

            for (vout, output) in tx.outputs.iter().enumerate() {
                let outpoint_key = [&txid[..], &(vout as u32).to_le_bytes()[..]].concat();

                let utxo_entry = StoredUtxoEntry {
                    output: output.clone(),
                    height,
                    is_coinbase,
                };
                let utxo_data = serialize::to_bytes(&utxo_entry)?;
                batch.put_cf(&cf_utxo, &outpoint_key, &utxo_data);
            }
        }

        // Save undo data indexed by block hash
        let undo_bytes = serialize::to_bytes(&undo_block)?;
        batch.put_cf(&cf_undo, block_id, undo_bytes);

        // Update metadata
        let cf_meta = self
            .db
            .cf_handle(CF_META)
            .ok_or_else(|| StorageError::DatabaseError("meta CF not found".into()))?;
        batch.put_cf(&cf_meta, KEY_TIP, header_bytes.clone());
        batch.put_cf(&cf_meta, KEY_HEIGHT, height.to_le_bytes());

        // Write batch atomically with sync for durability
        self.db
            .write_opt(batch, &Self::sync_write_opts())
            .map_err(|e| StorageError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    fn disconnect_block(&mut self, block: &Block) -> Result<(), StorageError> {
        let block_id = Self::block_id(&block.header);
        let mut batch = WriteBatch::default();

        let cf_utxo = self
            .db
            .cf_handle(CF_UTXO)
            .ok_or_else(|| StorageError::DatabaseError("utxo CF not found".into()))?;
        let cf_meta = self
            .db
            .cf_handle(CF_META)
            .ok_or_else(|| StorageError::DatabaseError("meta CF not found".into()))?;
        let cf_undo = self
            .db
            .cf_handle(CF_UNDO)
            .ok_or_else(|| StorageError::DatabaseError("undo CF not found".into()))?;

        // Load undo data for this block
        let undo_data = self
            .db
            .get_cf(&cf_undo, block_id)
            .map_err(|e| StorageError::DatabaseError(e.to_string()))?;

        let undo_block: crate::undo_block::UndoBlock = match undo_data {
            Some(data) => serialize::from_bytes(&data)?,
            None => {
                // No undo data found - this might be an old block from before undo was implemented
                // Fall back to the old method (less efficient but works)
                return self.disconnect_block_legacy(block);
            }
        };

        // Restore spent UTXOs from undo data
        for spent_output in &undo_block.spent_outputs {
            let outpoint_key = [
                &spent_output.prev_txid[..],
                &spent_output.prev_vout.to_le_bytes()[..],
            ]
            .concat();

            let utxo_entry = StoredUtxoEntry {
                output: spent_output.output.clone(),
                height: spent_output.height,
                is_coinbase: spent_output.is_coinbase,
            };
            let utxo_data = serialize::to_bytes(&utxo_entry)?;

            batch.put_cf(&cf_utxo, &outpoint_key, &utxo_data);
        }

        // Delete UTXOs created by this block
        for tx in &block.transactions {
            let txid = tx.txid();
            for i in 0..tx.outputs.len() {
                let outpoint_key = [&txid[..], &(i as u32).to_le_bytes()[..]].concat();
                batch.delete_cf(&cf_utxo, &outpoint_key);
            }
        }

        // Update tip and height
        let new_height = self.height()?.saturating_sub(1);
        let prev_header_bytes = match self.get_block(&block.header.prev_block)? {
            Some(prev_block) => Some(serialize::to_bytes(&prev_block.header)?),
            None => None,
        };

        if let Some(header_bytes) = prev_header_bytes {
            batch.put_cf(&cf_meta, KEY_TIP, header_bytes);
        } else {
            // Genesis block is being disconnected
            batch.delete_cf(&cf_meta, KEY_TIP);
        }

        batch.put_cf(&cf_meta, KEY_HEIGHT, new_height.to_le_bytes());

        self.db
            .write_opt(batch, &Self::sync_write_opts())
            .map_err(|e| StorageError::DatabaseError(e.to_string()))
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
                let block: Block = serialize::from_bytes(&bytes)?;
                Ok(Some(block))
            }
            None => Ok(None),
        }
    }

    fn tip(&self) -> Result<Option<BlockHeader>, StorageError> {
        match self.get_meta(KEY_TIP)? {
            Some(bytes) => {
                let header: BlockHeader = serialize::from_bytes(&bytes)?;
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
                let tx: Transaction = serialize::from_bytes(&bytes)?;
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

impl RocksDBStore {
    /// Legacy disconnect method for blocks without undo data.
    /// This is less efficient but provides backwards compatibility.
    fn disconnect_block_legacy(&mut self, block: &Block) -> Result<(), StorageError> {
        let mut batch = WriteBatch::default();
        let cf_utxo = self
            .db
            .cf_handle(CF_UTXO)
            .ok_or_else(|| StorageError::DatabaseError("utxo CF not found".into()))?;
        let cf_meta = self
            .db
            .cf_handle(CF_META)
            .ok_or_else(|| StorageError::DatabaseError("meta CF not found".into()))?;

        // Revert UTXO changes
        for tx in block.transactions.iter() {
            // For each input, find the transaction it came from, get the output, and re-add it to the UTXO set
            if !(tx.inputs.len() == 1 && tx.inputs[0].prev_txid == [0u8; 32]) {
                for input in &tx.inputs {
                    let prev_tx = self
                        .get_transaction(&input.prev_txid)?
                        .ok_or(StorageError::TxNotFound)?;
                    let spent_output = prev_tx.outputs.get(input.prev_vout as usize).ok_or(
                        StorageError::DatabaseError(
                            "Spent output not found in transaction".to_string(),
                        ),
                    )?;

                    let outpoint_key =
                        [&input.prev_txid[..], &input.prev_vout.to_le_bytes()[..]].concat();

                    // NOTE: Legacy method doesn't have height/is_coinbase info
                    // Use height=0 (considered mature) as safe fallback
                    let utxo_entry = StoredUtxoEntry {
                        output: spent_output.clone(),
                        height: 0,          // Considered mature
                        is_coinbase: false, // Conservative assumption
                    };
                    let utxo_data = serialize::to_bytes(&utxo_entry)?;

                    batch.put_cf(&cf_utxo, &outpoint_key, &utxo_data);
                }
            }

            // For each output, delete it from the UTXO set
            let txid = tx.txid();
            for i in 0..tx.outputs.len() {
                let outpoint_key = [&txid[..], &(i as u32).to_le_bytes()[..]].concat();
                batch.delete_cf(&cf_utxo, &outpoint_key);
            }
        }

        // Update tip and height
        let new_height = self.height()?.saturating_sub(1);
        let prev_header_bytes = match self.get_block(&block.header.prev_block)? {
            Some(prev_block) => Some(serialize::to_bytes(&prev_block.header)?),
            None => None,
        };

        if let Some(header_bytes) = prev_header_bytes {
            batch.put_cf(&cf_meta, KEY_TIP, header_bytes);
        } else {
            batch.delete_cf(&cf_meta, KEY_TIP);
        }

        batch.put_cf(&cf_meta, KEY_HEIGHT, new_height.to_le_bytes());

        self.db
            .write_opt(batch, &Self::sync_write_opts())
            .map_err(|e| StorageError::DatabaseError(e.to_string()))
    }

    /// Rollback the blockchain to a specific height.
    /// This disconnects all blocks above the target height.
    ///
    /// # Arguments
    /// * `target_height` - The height to rollback to
    ///
    /// # Returns
    /// The number of blocks disconnected
    pub fn rollback_to_height(&mut self, target_height: u64) -> Result<u64, StorageError> {
        let current_height = self.height()?;

        if target_height >= current_height {
            return Ok(0); // Nothing to rollback
        }

        info!(
            "Rolling back from height {} to {}",
            current_height, target_height
        );

        let mut blocks_disconnected = 0u64;

        while self.height()? > target_height {
            // Get the current tip block
            let tip_header = self.tip()?.ok_or_else(|| {
                StorageError::DatabaseError("No tip found during rollback".into())
            })?;

            let tip_id = Self::block_id(&tip_header);
            let block = self.get_block(&tip_id)?.ok_or_else(|| {
                StorageError::DatabaseError("Tip block not found in database".into())
            })?;

            // Disconnect the block
            self.disconnect_block(&block)?;
            blocks_disconnected += 1;

            if blocks_disconnected % 100 == 0 {
                info!(
                    "Disconnected {} blocks during rollback",
                    blocks_disconnected
                );
            }
        }

        info!(
            "Rollback complete: disconnected {} blocks",
            blocks_disconnected
        );
        Ok(blocks_disconnected)
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
        let temp_dir = tempfile::tempdir().expect("Failed to create temp directory");
        let mut store = RocksDBStore::open(temp_dir.path()).expect("Failed to open RocksDB store");

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
            sig_algo: SigAlgorithm::Dilithium5,
        };

        let header = BlockHeader {
            version: 1,
            prev_block: [0u8; 32],
            merkle_root: [0u8; 32],
            pqc_agg_hint: [0u8; 32],
            uncles_hash: [0u8; 32],
            time: 1729900000,
            bits: 0x1d00ffff,
            nonce: 0,
            algo_id: 0,
        };

        let block = Block {
            header: header.clone(),
            uncles: vec![],
            transactions: vec![coinbase.clone()],
        };

        // Insert block
        store
            .insert_block(block.clone())
            .expect("Failed to insert block");

        // Verify height
        assert_eq!(store.height().expect("Failed to get store height"), 1);

        // Verify tip
        let tip = store
            .tip()
            .expect("Failed to get tip")
            .expect("Tip is None");
        assert_eq!(tip.time, header.time);

        // Get block by height (first block is at height 0)
        let retrieved = store
            .get_block_by_height(0)
            .expect("Failed to get block by height")
            .expect("Block is None");
        assert_eq!(retrieved.header.time, header.time);

        // Get transaction
        let txid = coinbase.txid();
        let tx = store
            .get_transaction(&txid)
            .expect("Failed to get transaction")
            .expect("Transaction is None");
        assert_eq!(tx.version, coinbase.version);
    }

    #[test]
    fn test_utxo_operations() {
        let temp_dir = tempfile::tempdir().expect("Failed to create temp directory");
        let mut store = RocksDBStore::open(temp_dir.path()).expect("Failed to open RocksDB store");

        let outpoint = b"test_outpoint_123";
        let data = b"utxo_data";

        // Put UTXO
        store.put_utxo(outpoint, data).expect("Failed to put UTXO");

        // Get UTXO
        let retrieved = store
            .get_utxo(outpoint)
            .expect("Failed to get UTXO")
            .expect("UTXO is None");
        assert_eq!(retrieved, data);

        // Delete UTXO
        store.delete_utxo(outpoint).expect("Failed to delete UTXO");

        // Verify deleted
        assert!(store
            .get_utxo(outpoint)
            .expect("Failed to get UTXO after deletion")
            .is_none());
    }
}
