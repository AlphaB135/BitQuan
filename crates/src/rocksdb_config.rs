//! Optimized RocksDB configuration for BitQuan blockchain

use rocksdb::{Options, Cache, ColumnFamilyDescriptor, WriteOptions};
use std::path::Path;

/// Optimized RocksDB configuration for blockchain workloads
pub struct OptimizedRocksDBConfig {
    pub block_cache_size: usize,
    pub index_cache_size: usize,
    pub metadata_cache_size: usize,
    pub write_buffer_size: usize,
    pub max_write_buffers: usize,
    pub compression_per_cf: Vec<(&'static str, &'static str)>,
    pub background_jobs: i32,
    pub max_subcompactions: i32,
    pub bloom_bits_per_key: i32,
}

impl Default for OptimizedRocksDBConfig {
    fn default() -> Self {
        Self {
            block_cache_size: 512 * 1024 * 1024,      // 512MB
            index_cache_size: 256 * 1024 * 1024,      // 256MB
            metadata_cache_size: 128 * 1024 * 1024,    // 128MB
            write_buffer_size: 128 * 1024 * 1024,      // 128MB
            max_write_buffers: 3,
            compression_per_cf: vec![
                ("headers", "zstd"),
                ("height_index", "zstd"),
                ("tx_index", "lz4"),
                ("blocks_full", "no"),
                ("blocks_pruned", "zstd"),
                ("utxo", "zstd"),
                ("meta", "snappy"),
                ("undo", "zstd"),
            ],
            background_jobs: 8,
            max_subcompactions: 2,
            bloom_bits_per_key: 10,
        }
    }
}

impl OptimizedRocksDBConfig {
    /// Create optimized RocksDB options
    pub fn build_db_options(&self) -> Options {
        let mut opts = Options::default();

        // Enable multi-threading
        opts.set_increase_parallelism(self.background_jobs);

        // Cache configuration
        let block_cache = Cache::new_lru_cache(self.block_cache_size);
        let compressed_cache = Cache::new_lru_cache(self.block_cache_size / 4);
        let index_cache = Cache::new_lru_cache(self.index_cache_size);
        let row_cache = Cache::new_lru_cache(self.metadata_cache_size);

        opts.set_block_cache(block_cache);
        opts.set_block_cache_compressed(compressed_cache);
        opts.set_row_cache(row_cache);

        // Write buffer tuning
        opts.set_write_buffer_size(self.write_buffer_size);
        opts.set_max_write_buffer_number(self.max_write_buffers);
        opts.set_min_write_buffer_number_to_merge(2);
        opts.set_level0_file_num_compaction_trigger(4);
        opts.set_level0_slowdown_writes_trigger(12);
        opts.set_level0_stop_writes_trigger(20);

        // Compaction settings
        opts.set_max_background_jobs(self.background_jobs);
        opts.set_max_subcompactions(self.max_subcompactions);
        opts.set_compaction_style(rocksdb::DBCompactionStyle::Level);

        // Level-specific tuning
        opts.set_max_bytes_for_level_base(512 * 1024 * 1024);  // 512MB L1
        for i in 1..7 {
            opts.set_max_bytes_for_level_multiplier(i, 10);
        }

        // Block size and format tuning
        opts.set_block_size(16 * 1024);  // 16KB
        opts.set_block_size_deviation(10);
        opts.set_target_file_size_base(64 * 1024 * 1024);  // 64MB per level

        // File descriptor optimization
        opts.set_max_open_files(-1);      // Unlimited
        opts.set_use_fsync(false);       // Let OS handle

        // Statistics
        opts.enable_statistics(true);
        opts.set_stats_dump_period_sec(60);

        // Error handling
        opts.set_paranoid_checks(false); // For performance, enable in debug
        opts.set_max_total_wal_size(512 * 1024 * 1024); // 512MB WAL

        // Optimization for blockchain workload
        opts.set_allow_concurrent_memtable_write(true);
        opts.set_enable_write_thread_adaptive_yield(true);
        opts.set_bulk_load_flush(true);

        opts
    }

    /// Create column family descriptors with optimized options
    pub fn build_column_families(&self) -> Vec<ColumnFamilyDescriptor> {
        let mut cf_descriptors = Vec::new();

        // Base column family options
        let base_opts = self.build_cf_base_options();

        // Define all column families with their specific options
        let cf_configs = vec![
            ("headers", true, true),                // Read-heavy, needs bloom filter
            ("height_index", true, true),          // Read-heavy, needs bloom filter
            ("tx_index", true, true),              // Read-heavy, needs bloom filter
            ("blocks_full", false, false),         // Append-only, no optimization
            ("blocks_pruned", false, false),       // Archive, compressed
            ("utxo", true, true),                   // Read/write, needs bloom filter
            ("meta", true, false),                 // Metadata, small data
            ("undo", false, false),                // Write-intensive, archive
        ];

        for (name, needs_bloom, needs_optimize) in cf_configs {
            let mut cf_opts = base_opts.clone();

            if needs_bloom {
                cf_opts.set_bloom_filter(self.bloom_bits_per_key, false);
                cf_opts.set_optimize_for_point_lookup(15);
            }

            if needs_optimize {
                cf_opts.set_level0_file_num_compaction_trigger(4);
                cf_opts.set_target_file_size_base(64 * 1024 * 1024);
            }

            // Set compression
            let compression = self.compression_per_cf
                .iter()
                .find(|(cf, _)| *cf == name)
                .map(|(_, comp)| comp)
                .unwrap_or("snappy");

            match *compression {
                "zstd" => cf_opts.set_compression_type(rocksdb::DBCompressionType::Zstd),
                "lz4" => cf_opts.set_compression_type(rocksdb::DBCompressionType::Lz4),
                "no" => cf_opts.set_compression_type(rocksdb::DBCompressionType::None),
                "snappy" => cf_opts.set_compression_type(rocksdb::DBCompressionType::Snappy),
                _ => cf_opts.set_compression_type(rocksdb::DBCompressionType::Snappy),
            }

            cf_descriptors.push(ColumnFamilyDescriptor::new(name, cf_opts));
        }

        cf_descriptors
    }

    /// Build base column family options
    fn build_cf_base_options(&self) -> Options {
        let mut opts = Options::default();

        // Inherit DB-level settings
        opts.set_write_buffer_size(self.write_buffer_size);
        opts.set_max_write_buffer_number(self.max_write_buffers);
        opts.set_min_write_buffer_number_to_merge(2);

        opts
    }

    /// Get optimized write options for different scenarios
    pub fn write_options(&self, durability: WriteDurability) -> WriteOptions {
        let mut opts = WriteOptions::default();

        match durability {
            WriteDurability::Low => {
                opts.set_sync(false);
                opts.set_wal_bytes_per_sync(1024 * 1024); // 1MB
            }
            WriteDurability::Normal => {
                opts.set_sync(true);
                opts.set_wal_bytes_per_sync(0);
            }
            WriteDurability::High => {
                opts.set_sync(true);
                opts.set_wal_bytes_per_sync(0);
            }
        }

        opts
    }

    /// Get recommended cache sizes based on system memory
    pub fn auto_tune_cache(&mut self, total_memory_mb: usize) {
        let available_memory = total_memory_mb * 1024 * 1024;

        // Allocate 50% of available memory for caching
        let cache_budget = (available_memory * 50) / 100;

        // Block cache gets 60% of cache budget
        self.block_cache_size = (cache_budget * 60) / 100;

        // Index cache gets 25% of cache budget
        self.index_cache_size = (cache_budget * 25) / 100;

        // Row cache gets 15% of cache budget
        self.metadata_cache_size = (cache_budget * 15) / 100;
    }

    /// Performance tuning for specific workloads
    pub fn for_sync_workload(&mut self) {
        // Larger write buffers for sync
        self.write_buffer_size = 256 * 1024 * 1024; // 256MB
        self.max_write_buffers = 5;

        // More background threads
        self.background_jobs = 12;

        // Less aggressive compaction during sync
        opts.set_level0_slowdown_writes_trigger(8);
        opts.set_level0_stop_writes_trigger(16);
    }

    /// Performance tuning for mining workloads
    pub fn for_mining_workload(&mut self) {
        // Smaller write buffers for frequent small writes
        self.write_buffer_size = 64 * 1024 * 1024; // 64MB

        // Fewer background threads to save CPU
        self.background_jobs = 4;

        // More aggressive compaction
        opts.set_level0_file_num_compaction_trigger(2);
    }

    /// Get cache hit rate metrics
    pub fn get_cache_stats(&self, db: &rocksdb::DB) -> CacheStats {
        CacheStats {
            block_cache_hit: db.get_statistics()
                .get("block.cache.hit.count")
                .unwrap_or(0),
            block_cache_miss: db.get_statistics()
                .get("block.cache.miss.count")
                .unwrap_or(0),
            index_cache_hit: db.get_statistics()
                .get("index.cache.hit.count")
                .unwrap_or(0),
            index_cache_miss: db.get_statistics()
                .get("index.cache.miss.count")
                .unwrap_or(0),
        }
    }
}

/// Write durability levels
#[derive(Debug, Clone, Copy)]
pub enum WriteDurability {
    /// Low durability - fastest, possible data loss on crash
    Low,
    /// Normal durability - good balance
    Normal,
    /// High durability - sync all writes
    High,
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub block_cache_hit: u64,
    pub block_cache_miss: u64,
    pub index_cache_hit: u64,
    pub index_cache_miss: u64,
}

impl CacheStats {
    /// Calculate block cache hit rate
    pub fn block_cache_hit_rate(&self) -> f64 {
        let total = self.block_cache_hit + self.block_cache_miss;
        if total == 0 {
            0.0
        } else {
            self.block_cache_hit as f64 / total as f64 * 100.0
        }
    }

    /// Calculate index cache hit rate
    pub fn index_cache_hit_rate(&self) -> f64 {
        let total = self.index_cache_hit + self.index_cache_miss;
        if total == 0 {
            0.0
        } else {
            self.index_cache_hit as f64 / total as f64 * 100.0
        }
    }

    /// Get total hit rate
    pub fn total_hit_rate(&self) -> f64 {
        let total_hits = self.block_cache_hit + self.index_cache_hit;
        let total_misses = self.block_cache_miss + self.index_cache_miss;
        let total = total_hits + total_misses;

        if total == 0 {
            0.0
        } else {
            total_hits as f64 / total as f64 * 100.0
        }
    }
}

/// Performance monitoring utilities
pub struct PerformanceMonitor {
    pub config: OptimizedRocksDBConfig,
    pub db: Option<rocksdb::DB>,
}

impl PerformanceMonitor {
    pub fn new(config: OptimizedRocksDBConfig) -> Self {
        Self {
            config,
            db: None,
        }
    }

    /// Set database reference for monitoring
    pub fn set_db(&mut self, db: rocksdb::DB) {
        self.db = Some(db);
    }

    /// Get comprehensive performance metrics
    pub fn get_metrics(&self) -> PerformanceMetrics {
        if let Some(ref db) = self.db {
            let stats = db.get_statistics();

            PerformanceMetrics {
                block_cache_hit_rate: self.config.get_cache_stats(db).block_cache_hit_rate(),
                index_cache_hit_rate: self.config.get_cache_stats(db).index_cache_hit_rate(),
                total_hit_rate: self.config.get_cache_stats(db).total_hit_rate(),
                bytes_written: stats.get("rocksdb.bytes-written").unwrap_or(0),
                bytes_read: stats.get("rocksdb.bytes-read").unwrap_or(0),
                compaction_count: stats.get("rocksdb.num-files-at-level0").unwrap_or(0),
                write_stall_count: stats.get("rocksdb Stall count").unwrap_or(0),
                memtable_flush: stats.get("rocksdb.num-flushes").unwrap_or(0),
                bg_error_count: stats.get("rocksdb.background-errors").unwrap_or(0),
            }
        } else {
            PerformanceMetrics::default()
        }
    }

    /// Print performance report
    pub fn print_report(&self) {
        let metrics = self.get_metrics();

        println!("=== RocksDB Performance Report ===");
        println!("Block Cache Hit Rate: {:.2}%", metrics.block_cache_hit_rate);
        println!("Index Cache Hit Rate: {:.2}%", metrics.index_cache_hit_rate);
        println!("Total Cache Hit Rate: {:.2}%", metrics.total_hit_rate);
        println!("Bytes Written: {}", metrics.bytes_written);
        println!("Bytes Read: {}", metrics.bytes_read);
        println!("Compaction Count: {}", metrics.compaction_count);
        println!("Write Stall Count: {}", metrics.write_stall_count);
        println!("Memtable Flush Count: {}", metrics.memtable_flush);
        println!("Background Errors: {}", metrics.bg_error_count);
        println!("================================");
    }
}

/// Comprehensive performance metrics
#[derive(Debug, Clone, Default)]
pub struct PerformanceMetrics {
    pub block_cache_hit_rate: f64,
    pub index_cache_hit_rate: f64,
    pub total_hit_rate: f64,
    pub bytes_written: u64,
    pub bytes_read: u64,
    pub compaction_count: u64,
    pub write_stall_count: u64,
    pub memtable_flush: u64,
    pub bg_error_count: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = OptimizedRocksDBConfig::default();
        assert_eq!(config.block_cache_size, 512 * 1024 * 1024);
        assert_eq!(config.write_buffer_size, 128 * 1024 * 1024);
        assert_eq!(config.background_jobs, 8);
    }

    #[test]
    fn test_cache_hit_rate_calculation() {
        let mut stats = CacheStats {
            block_cache_hit: 900,
            block_cache_miss: 100,
            index_cache_hit: 800,
            index_cache_miss: 200,
        };

        assert_eq!(stats.block_cache_hit_rate(), 90.0);
        assert_eq!(stats.index_cache_hit_rate(), 80.0);
        assert_eq!(stats.total_hit_rate(), 85.0);
    }

    #[test]
    fn test_auto_tune_cache() {
        let mut config = OptimizedRocksDBConfig::default();
        config.auto_tune_cache(16384); // 16GB RAM

        // Should have adjusted cache sizes
        assert!(config.block_cache_size > 512 * 1024 * 1024);
    }
}