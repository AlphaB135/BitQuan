//! State Partitioning - Distributes blockchain state across shards

use crate::{ShardError, PartitioningStrategy};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use blake3::Hash;

/// Manages state partitioning across shards
pub struct StatePartitioner {
    local_shard_id: u16,
    total_shards: u16,
    partitioning: PartitioningStrategy,
    local_columns: HashMap<String, StateColumn>,
    cross_shard_cache: Arc<RwLock<HashMap<[u8; 32], Vec<u8>>>>,
}

/// A column family for a specific data type
pub struct StateColumn {
    pub name: String,
    pub range: ShardRange,
    pub entries: usize,
}

/// Range of shard IDs a column covers
#[derive(Debug, Clone, Copy)]
pub struct ShardRange {
    pub start: u16,
    pub end: u16,
}

impl ShardRange {
    pub fn new(start: u16, end: u16) -> Self {
        Self { start, end }
    }

    pub fn contains(&self, shard_id: u16) -> bool {
        shard_id >= self.start && shard_id <= self.end
    }
}

impl StateColumn {
    pub fn new(name: &str, range: ShardRange) -> Self {
        Self {
            name: name.to_string(),
            range,
            entries: 0,
        }
    }

    pub fn belongs_to_this_shard(&self, shard_id: u16) -> bool {
        self.range.contains(shard_id)
    }
}

impl StatePartitioner {
    /// Create a new state partitioner
    pub fn new(local_shard_id: u16, total_shards: u16) -> Self {
        let partitioning = PartitioningStrategy::Hash; // Default

        // Create columns for local shard
        let mut local_columns = HashMap::new();

        // UTXO column - handles UTXOs for this shard's addresses
        let utxo_range = calculate_shard_range(local_shard_id, total_shards);
        local_columns.insert("utxo".to_string(), StateColumn::new("utxo", utxo_range));

        // Accounts column - manages account balances
        let account_range = calculate_shard_range(local_shard_id, total_shards);
        local_columns.insert("accounts".to_string(), StateColumn::new("accounts", account_range));

        // Contracts column - handles smart contract state
        let contract_range = calculate_shard_range(local_shard_id, total_shards);
        local_columns.insert("contracts".to_string(), StateColumn::new("contracts", contract_range));

        Self {
            local_shard_id,
            total_shards,
            partitioning,
            local_columns,
            cross_shard_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get the column responsible for a given key
    pub fn get_column_for_key(&self, key: &[u8]) -> Option<&StateColumn> {
        let shard_id = self.get_shard_for_key(key);

        if shard_id == self.local_shard_id {
            // Find column in this shard
            for column in self.local_columns.values() {
                if column.range.contains(shard_id) {
                    return Some(column);
                }
            }
        }

        None
    }

    /// Get the shard ID responsible for a given key
    pub fn get_shard_for_key(&self, key: &[u8]) -> u16 {
        let hash = blake3::hash(key);
        let hash_bytes = hash.as_bytes();

        match self.partitioning {
            PartitioningStrategy::Hash => {
                let shard_value = u16::from_be_bytes([hash_bytes[0], hash_bytes[1]]);
                shard_value % self.total_shards
            }
            PartitioningStrategy::Range => {
                // Use first byte to determine shard
                (key[0] as u16) % self.total_shards
            }
            PartitioningStrategy::Consistent => {
                // Simplified consistent hashing
                let value = (hash_bytes[0] as u16) * 256 + (hash_bytes[1] as u16);
                value % self.total_shards
            }
        }
    }

    /// Check if a key belongs to this shard
    pub fn is_key_local(&self, key: &[u8]) -> bool {
        let shard_id = self.get_shard_for_key(key);
        shard_id == self.local_shard_id
    }

    /// Get column statistics
    pub fn get_column_stats(&self) -> HashMap<String, ColumnStats> {
        let mut stats = HashMap::new();

        for (name, column) in &self.local_columns {
            stats.insert(name.clone(), ColumnStats {
                name: name.clone(),
                range: column.range,
                entries: column.entries,
                is_local: column.belongs_to_this_shard(self.local_shard_id),
            });
        }

        stats
    }

    /// Update entry count for a column
    pub fn update_entry_count(&mut self, column_name: &str, delta: isize) {
        if let Some(column) = self.local_columns.get_mut(column_name) {
            // Ensure count doesn't go negative
            column.entries = column.entries.saturating_add_signed(delta);
        }
    }

    /// Get cross-shard state with caching
    pub async fn get_cross_shard_state(
        &self,
        key: &[u8],
        target_shard: u16,
    ) -> Option<Vec<u8>> {
        // Check cache first
        let cache_key = self.cache_key(key, target_shard);
        let cache = self.cross_shard_cache.read().await;

        if let Some(cached_value) = cache.get(&cache_key) {
            return Some(cached_value.clone());
        }
        drop(cache);

        // In a real implementation, this would make a network request
        // to the target shard to retrieve the state
        // For now, return None (not found)
        None
    }

    /// Store cross-shard state in cache
    pub async fn cache_cross_shard_state(
        &self,
        key: &[u8],
        target_shard: u16,
        value: Vec<u8>,
    ) {
        let cache_key = self.cache_key(key, target_shard);
        let mut cache = self.cross_shard_cache.write().await;
        cache.insert(cache_key, value);
    }

    /// Generate cache key
    fn cache_key(&self, key: &[u8], target_shard: u16) -> [u8; 32] {
        let mut hasher = blake3::Hasher::new();
        hasher.update(key);
        hasher.update(&target_shard.to_le_bytes());
        *hasher.finalize().as_bytes()
    }

    /// Clear expired cache entries
    pub async fn clear_expired_cache(&self) {
        // In a real implementation, this would remove entries older than TTL
        // For now, just clear half of the cache as a simple strategy
        let mut cache = self.cross_shard_cache.write().await;
        if cache.len() > 1000 {
            let remove_count = cache.len() / 2;
            let mut keys: Vec<[u8; 32]> = cache.keys().cloned().collect();
            keys.truncate(remove_count);
            for key in keys {
                cache.remove(&key);
            }
        }
    }
}

/// Calculate shard range for consistent distribution
pub fn calculate_shard_range(shard_id: u16, total_shards: u16) -> ShardRange {
    let start = (shard_id * 65536 / total_shards) as u16;
    let end = ((shard_id + 1) * 65536 / total_shards - 1) as u16;
    ShardRange::new(start, end)
}

/// Column statistics
#[derive(Debug, Clone)]
pub struct ColumnStats {
    pub name: String,
    pub range: ShardRange,
    pub entries: usize,
    pub is_local: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shard_range_calculation() {
        // Test with 4 shards
        let range0 = calculate_shard_range(0, 4);
        assert_eq!(range0.start, 0);
        assert_eq!(range0.end, 16383);

        let range1 = calculate_shard_range(1, 4);
        assert_eq!(range1.start, 16384);
        assert_eq!(range1.end, 32767);

        let range2 = calculate_shard_range(2, 4);
        assert_eq!(range2.start, 32768);
        assert_eq!(range2.end, 49151);

        let range3 = calculate_shard_range(3, 4);
        assert_eq!(range3.start, 49152);
        assert_eq!(range3.end, 65535);
    }

    #[test]
    fn test_shard_contains() {
        let range = ShardRange::new(100, 200);
        assert!(range.contains(150));
        assert!(!range.contains(99));
        assert!(!range.contains(201));
    }

    #[tokio::test]
    async fn test_state_partitioner() {
        let partitioner = StatePartitioner::new(0, 4);

        // Test key routing
        let key1 = b"account_123";
        let shard1 = partitioner.get_shard_for_key(key1);
        assert!(shard1 < 4);

        let key2 = b"account_456";
        let shard2 = partitioner.get_shard_for_key(key2);
        assert!(shard2 < 4);

        // Test if keys are local
        assert!(partitioner.is_key_local(key1));

        // Test column stats
        let stats = partitioner.get_column_stats();
        assert!(stats.contains_key("utxo"));
        assert!(stats.contains_key("accounts"));
        assert!(stats.contains_key("contracts"));
    }

    #[test]
    fn test_update_entry_count() {
        let mut partitioner = StatePartitioner::new(0, 4);

        let initial_count = partitioner.local_columns["utxo"].entries;
        partitioner.update_entry_count("utxo", 5);
        assert_eq!(partitioner.local_columns["utxo"].entries, initial_count + 5);

        partitioner.update_entry_count("utxo", -3);
        assert_eq!(partitioner.local_columns["utxo"].entries, initial_count + 2);
    }
}