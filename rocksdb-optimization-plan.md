# BitQuan RocksDB Performance Optimization Plan

## Executive Summary

This document analyzes BitQuan's RocksDB usage and proposes specific optimizations to improve performance, reduce memory usage, and enhance scalability. The analysis reveals several configuration gaps and optimization opportunities that can significantly improve database operations.

## Current Implementation Analysis

### 1. Current RocksDB Configuration

**Version**: rocksdb = "0.23" (LTS version, stable but not latest)

**Current Column Families**:
- `blocks` - Full block data (headers + transactions)
- `headers` - Block headers only
- `height_index` - Height to hash mapping
- `tx_index` - Transaction index
- `utxo` - UTXO set
- `meta` - Metadata
- `undo` - Undo data for rollbacks

**Current Options**:
```rust
let mut opts = Options::default();
opts.create_if_missing(true);
opts.create_missing_column_families(true);
```

**Critical Missing Configurations**:
- No cache configuration (performance bottleneck)
- No compression settings (storage inefficiency)
- No write buffer/flush settings
- No read/write threading configuration
- No background thread optimization
- No table format/block size tuning

### 2. Performance Bottlenecks Identified

#### 2.1 Cache Issues
- **No Block Cache**: Currently 0MB block cache
- **No Row Cache**: No caching for frequently accessed data
- **No Index Cache**: Metadata and index lookups uncached
- **Impact**: Every block retrieval requires disk I/O

#### 2.2 Write Performance Issues
- **Sync Writes**: All writes use `set_sync(true)` - maximum durability but minimum performance
- **Single Write Batch**: No parallelization of write operations
- **No Write Buffer Tuning**: Using RocksDB defaults (4MB-64MB range)
- **Background Compaction**: Default settings may not be optimal for blockchain workload

#### 2.3 Compression Inefficiency
- **Default Compression**: Using Snappy (fast but low compression)
- **No ZSTD/LZ4**: Better compression ratios available
- **No Compression per CF**: All column families use same compression

#### 2.4 Concurrency Problems
- **No Read Threading**: Multi-threaded reads not enabled
- **Write Contention**: Single writer bottleneck
- **Background Threads**: Default thread count may be insufficient

### 3. Column Family Analysis

#### 3.1 Current CF Usage Analysis
- **blocks**: High volume, append-only mostly, random reads
- **headers**: High frequency reads, infrequent writes
- **height_index**: Very frequent reads during sync
- **tx_index**: Read-heavy, transaction lookup
- **utxo**: Read/write intensive, UTXO management
- **meta**: Read/write, small data
- **undo**: Write-intensive, rollback data

#### 3.2 Recommended CF Specialization
Each column family should have specialized settings based on access patterns.

## Optimization Recommendations

### 1. Column Family Reorganization

#### 1.1 Recommended New CF Structure
```rust
// Hot Data - Frequently Accessed
const CF_HEADERS: &str = "headers";          // Bloom filter, high cache priority
const CF_HEIGHT_INDEX: &str = "height_index"; // Bloom filter, high cache priority
const CF_TX_INDEX: &str = "tx_index";        // Bloom filter, moderate cache

// Cold Data - Archive
const CF_BLOCKS_FULL: &str = "blocks_full";   // Low compression, no cache
const CF_BLOCKS_PRUNED: &str = "blocks_pruned"; // Medium compression
const CF_UTXO: &str = "utxo";               // Specialized compression

// Metadata
const CF_META: &str = "meta";               // Fast compression
const CF_UNDO: &str = "undo";               // High compression, archive mode
```

### 2. Cache Configuration

#### 2.1 Optimized Cache Settings
```rust
// Total cache: 1GB (adjustable based on system memory)
const BLOCK_CACHE_SIZE: usize = 512 * 1024 * 1024;  // 512MB block cache
const INDEX_FILTER_SIZE: usize = 256 * 1024 * 1024; // 256MB index cache
const METADATA_CACHE_SIZE: usize = 128 * 1024 * 1024; // 128MB metadata cache

// Block cache configuration
opts.set_block_cache(rocksdb::Cache::new_lru_cache(BLOCK_CACHE_SIZE));
opts.set_block_cache_compressed(rocksdb::Cache::new_lru_cache(BLOCK_CACHE_SIZE / 4));

// Row cache for frequently accessed small keys
opts.set_row_cache(rocksdb::Cache::new_lru_cache(256 * 1024 * 1024));
```

#### 2.2 Bloom Filters
```rust
// Enable bloom filters for point lookups
for cf_name in &[CF_HEADERS, CF_HEIGHT_INDEX, CF_TX_INDEX, CF_UTXO] {
    let cf_opts = ColumnFamilyOptions::new();
    cf_opts.set_bloom_filter(10.0, false); // 10 bits per key
    cf_opts.set_optimize_for_point_lookup(15); // 64KB index blocks
}
```

### 3. Write Optimization

#### 3.1 Write Buffer Configuration
```rust
// Write buffer tuning for blockchain workload
opts.set_write_buffer_size(128 * 1024 * 1024);  // 128MB write buffer
opts.set_max_write_buffer_number(3);
opts.set_min_write_buffer_number_to_merge(2);
opts.set_level0_file_num_compaction_trigger(4);
opts.set_level0_slowdown_writes_trigger(12);
opts.set_level0_stop_writes_trigger(20);

// Background compaction tuning
opts.set_max_background_jobs(8);
opts.set_max_subcompactions(2);
```

#### 3.2 Compression Settings
```rust
// Different compression per CF
const CF_OPTS: &[(&str, &str)] = &[
    (CF_HEADERS, "zstd"),           // Fast compression for headers
    (CF_HEIGHT_INDEX, "zstd"),      // Fast compression for index
    (CF_TX_INDEX, "lz4"),           // Good speed/compression ratio
    (CF_BLOCKS_FULL, "no"),        // No compression for full blocks
    (CF_BLOCKS_PRUNED, "zstd"),    // Medium compression for pruned
    (CF_UTXO, "zstd"),             // Good compression for UTXO
    (CF_META, "snappy"),           // Fast for small metadata
    (CF_UNDO, "zstd"),             // High compression for archive
];
```

### 4. Read Optimization

#### 4.1 Read Thread Configuration
```rust
// Multi-threaded reads
opts.set_increase_parallelism(8);  // 8 read threads
opts.set_max_open_files(-1);      // Unlimited file descriptors
opts.set_use_fsync(false);        // Let OS handle fsync
opts.set_stats_dump_period_sec(60); // Stats every minute
```

#### 4.2 Table Format Optimization
```rust
// Block size tuning
opts.set_block_size(16 * 1024);  // 16KB block size
opts.set_block_size_deviation(10);
opts.set_target_file_size_base(64 * 1024 * 1024);  // 64MB per level

// Level-specific tuning
opts.set_max_bytes_for_level_base(512 * 1024 * 1024); // 512MB L1
for i in 1..7 {
    opts.set_max_bytes_for_level_multiplier(i as i32, 10);
}
```

### 5. Concurrent Access Patterns

#### 5.1 Reader-Writer Optimization
```rust
// Enable concurrent writes
opts.set_allow_concurrent_memtable_write(true);
opts.set_enable_write_thread_adaptive_yield(true);

// Bulk load optimization for sync operations
opts.set_bulk_load_flush(true);
```

### 6. Specific Query Optimizations

#### 6.1 Batch Operations
```rust
// Use write batches for atomic operations
pub fn batch_insert_blocks(&self, blocks: &[Block]) -> Result<(), StorageError> {
    let mut batch = WriteBatch::default();
    // Batch multiple blocks together
    for block in blocks {
        // Batch insertion logic
    }
    self.db.write(batch)
}
```

#### 6.2 Range Query Optimization
```rust
// Use iterators efficiently
pub fn get_blocks_range(&self, start: u64, end: u64) -> Result<Vec<Block>, StorageError> {
    let cf = self.db.cf_handle(CF_HEIGHT_INDEX)?;
    let mut iterator = self.db.iterator_cf(&cf, rocksdb::IteratorMode::Start);

    let mut blocks = Vec::new();
    for (key, _) in iterator.by_ref().take((end - start + 1) as usize) {
        // Range logic
    }
    Ok(blocks)
}
```

### 7. Configuration Files

#### 7.1 Production Configuration
```toml
# rocksdb-config.toml
[block_cache]
size_mb = 512
compressed_cache_mb = 128

[write_buffer]
size_mb = 128
max_buffers = 3
min_buffers_to_merge = 2

[compression]
headers = "zstd"
height_index = "zstd"
tx_index = "lz4"
blocks_full = "no"
blocks_pruned = "zstd"
utxo = "zstd"
meta = "snappy"
undo = "zstd"

[background_threads]
max = 8
subcompactions = 2

[level_compaction]
level0_trigger = 4
level0_slowdown = 12
level0_stop = 20
level1_base_mb = 512
multiplier = 10
```

### 8. Monitoring and Metrics

#### 8.1 Performance Metrics
```rust
// Enable RocksDB statistics
opts.enable_statistics(true);

// Critical metrics to monitor:
// - block_cache_hit_count
// - block_cache_miss_count
// - compression_ratio
// - bg_error_count
// - stall_count
// - bytes_written
// - bytes_read
```

### 9. Performance Expectations

#### 9.1 Expected Improvements
- **Block Retrieval**: 10-50x faster (with caching)
- **Write Throughput**: 2-5x improvement
- **Memory Usage**: 30-50% reduction (with better compression)
- **Disk Space**: 20-40% savings
- **Sync Time**: 3-10x faster for IBD

#### 9.2 Hardware Requirements
- **Minimum**: 2GB RAM, SSD storage
- **Recommended**: 8GB+ RAM, NVMe SSD
- **Production**: 16GB+ RAM, enterprise SSD

### 10. Implementation Plan

#### Phase 1: Core Configuration
1. Add cache configuration
2. Implement compression settings
3. Add write buffer tuning

#### Phase 2: Column Family Optimization
1. Split blocks into full/pruned
2. Add bloom filters
3. Optimize level compaction

#### Phase 3: Advanced Features
1. Implement read threading
2. Add metrics collection
3. Create configuration management

#### Phase 4: Testing & Validation
1. Performance benchmarking
2. Load testing
3. Memory usage validation

### 11. Migration Considerations

#### 11.1 Backward Compatibility
- Maintain existing column families
- Add new CFs alongside
- Gradual migration path

#### 11.2 Data Integrity
- Backup before migration
- Verify data integrity
- Monitor error rates

### 12. Alternative Approaches

#### 12.1 Tiered Storage
- Hot data in RAM disk
- Warm data on SSD
- Cold data on HDD

#### 12.2 Columnar Format
- Consider Parquet for analytical queries
- Keep RocksDB for transactional operations

## Conclusion

The proposed optimizations can significantly improve BitQuan's database performance while maintaining data integrity. The key improvements come from:

1. **Proper caching** - Eliminating repeated disk I/O
2. **Compression tuning** - Reducing storage and I/O
3. **Write optimization** - Improving blockchain sync speed
4. **Concurrent access** - Better utilizing multi-core CPUs

These changes should be implemented incrementally with thorough testing at each phase.