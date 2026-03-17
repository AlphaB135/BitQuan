//! RocksDB performance benchmark for BitQuan

use criterion::{criterion_group, criterion_main, Criterion, BatchSize};
use criterion::black_box;
use tempfile::TempDir;
use rocksdb::{DB, Options, WriteBatch, WriteOptions};
use std::sync::Arc;
use tokio::runtime::Runtime;

// Import our optimized config
mod rocksdb_config {
    pub use super::super::crates::rocksdb_config::*;
}

use rocksdb_config::*;

fn create_test_data() -> Vec<(Vec<u8>, Vec<u8>)> {
    let mut data = Vec::new();
    for i in 0..10_000 {
        let key = format!("key_{}", i).into_bytes();
        let value = format!("value_{}", i).repeat(10); // 100 bytes per value
        data.push((key, value));
    }
    data
}

fn benchmark_write_performance(c: &mut Criterion) {
    let group = c.benchmark_group("Write Performance");

    for batch_size in [1, 10, 100, 1000].iter() {
        group.bench_with_input(
            format!("batch_size_{}", batch_size),
            batch_size,
            |b, &batch| {
                b.to_async(Runtime::new().unwrap()).iter(|| async {
                    let temp_dir = TempDir::new().unwrap();
                    let config = OptimizedRocksDBConfig::default();
                    let db_opts = config.build_db_options();

                    let db = DB::open(&db_opts, temp_dir.path()).unwrap();
                    let write_opts = config.write_options(WriteDurability::Normal);

                    let data = create_test_data();
                    let mut batch = WriteBatch::default();

                    for i in 0..data.len() {
                        let j = i % batch_size as usize;
                        batch.put(&data[j].0, &data[j].1);

                        if j == batch_size as usize - 1 || i == data.len() - 1 {
                            db.write_opt(batch, &write_opts).unwrap();
                            batch = WriteBatch::default();
                        }
                    }

                    black_box(());
                });
            },
        );
    }

    group.finish();
}

fn benchmark_read_performance(c: &mut Criterion) {
    let group = c.benchmark_group("Read Performance");

    for cache_size in [0, 256, 512, 1024].iter() {
        group.bench_with_input(
            format!("cache_size_mb_{}", cache_size),
            cache_size,
            |b, &cache_size| {
                b.to_async(Runtime::new().unwrap()).iter(|| async {
                    let temp_dir = TempDir::new().unwrap();
                    let mut config = OptimizedRocksDBConfig::default();

                    if cache_size > 0 {
                        config.block_cache_size = cache_size * 1024 * 1024;
                    }

                    let db_opts = config.build_db_options();
                    let db = DB::open(&db_opts, temp_dir.path()).unwrap();

                    // Write test data first
                    let data = create_test_data();
                    let write_opts = WriteOptions::default();
                    let mut batch = WriteBatch::default();

                    for (key, value) in data.iter().take(1000) {
                        batch.put(key, value);
                    }

                    db.write_opt(batch, &write_opts).unwrap();

                    // Benchmark reads
                    for (key, _) in data.iter().take(1000) {
                        let _ = black_box(db.get(key).unwrap().unwrap());
                    }
                });
            },
        );
    }

    group.finish();
}

fn benchmark_compaction_performance(c: &mut Criterion) {
    let group = c.benchmark_group("Compaction Performance");

    for workload in ["sync", "mining"].iter() {
        group.bench_with_input(
            format!("workload_{}", workload),
            workload,
            |b, _workload| {
                b.to_async(Runtime::new().unwrap()).iter(|| async {
                    let temp_dir = TempDir::new().unwrap();
                    let mut config = OptimizedRocksDBConfig::default();

                    // Apply workload-specific tuning
                    if *workload == "sync" {
                        config.for_sync_workload();
                    } else {
                        config.for_mining_workload();
                    }

                    let db_opts = config.build_db_options();
                    let db = DB::open(&db_opts, temp_dir.path()).unwrap();

                    // Generate large dataset
                    let write_opts = WriteOptions::default();
                    let mut batch = WriteBatch::default();

                    for i in 0..100_000 {
                        let key = format!("key_{}", i).into_bytes();
                        let value = format!("value_{}", i).repeat(50);
                        batch.put(key, value);

                        if i % 1000 == 0 {
                            db.write_opt(batch, &write_opts).unwrap();
                            batch = WriteBatch::default();
                        }
                    }

                    // Force compaction
                    db.compact_range(None, None).unwrap();

                    black_box(());
                });
            },
        );
    }

    group.finish();
}

fn benchmark_cache_effectiveness(c: &mut Criterion) {
    let group = c.benchmark_group("Cache Effectiveness");

    for pattern in ["sequential", "random"].iter() {
        group.bench_with_input(
            format!("access_pattern_{}", pattern),
            pattern,
            |b, _pattern| {
                b.to_async(Runtime::new().unwrap()).iter(|| async {
                    let temp_dir = TempDir::new().unwrap();
                    let config = OptimizedRocksDBConfig::default();
                    let db_opts = config.build_db_options();
                    let db = DB::open(&db_opts, temp_dir.path()).unwrap();

                    // Write test data
                    let data = create_test_data();
                    let write_opts = WriteOptions::default();
                    let mut batch = WriteBatch::default();

                    for (key, value) in data.iter() {
                        batch.put(key, value);
                    }

                    db.write_opt(batch, &write_opts).unwrap();

                    // Benchmark access pattern
                    if *_pattern == "sequential" {
                        // Sequential access
                        for (key, _) in data.iter().take(1000) {
                            let _ = black_box(db.get(key).unwrap().unwrap());
                        }
                    } else {
                        // Random access
                        let mut indices: Vec<usize> = (0..1000).collect();
                        // Shuffle for random access
                        indices.shuffle(&mut rand::thread_rng());

                        for idx in indices {
                            let (key, _) = &data[idx];
                            let _ = black_box(db.get(key).unwrap().unwrap());
                        }
                    }

                    black_box(());
                });
            },
        );
    }

    group.finish();
}

fn benchmark_column_family_performance(c: &mut Criterion) {
    let group = c.benchmark_group("Column Family Performance");

    for cf in ["headers", "utxo", "blocks"].iter() {
        group.bench_with_input(
            format!("column_family_{}", cf),
            cf,
            |b, _cf| {
                b.to_async(Runtime::new().unwrap()).iter(|| async {
                    let temp_dir = TempDir::new().unwrap();
                    let config = OptimizedRocksDBConfig::default();
                    let db_opts = config.build_db_options();
                    let cf_descriptors = config.build_column_families();

                    let db = DB::open_cf_descriptors(&db_opts, temp_dir.path(), cf_descriptors).unwrap();

                    // Write test data to specific CF
                    let cf_handle = db.cf_handle(_cf).unwrap();
                    let write_opts = WriteOptions::default();

                    for i in 0..1000 {
                        let key = format!("key_{}", i).into_bytes();
                        let value = format!("value_{}", i).repeat(100);
                        db.put_cf_opt(&cf_handle, key, value, &write_opts).unwrap();
                    }

                    // Read from CF
                    for i in 0..1000 {
                        let key = format!("key_{}", i).into_bytes();
                        let _ = black_box(db.get_cf(&cf_handle, key).unwrap().unwrap());
                    }

                    black_box(());
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    benchmark_write_performance,
    benchmark_read_performance,
    benchmark_compaction_performance,
    benchmark_cache_effectiveness,
    benchmark_column_family_performance
);
criterion_main!(benches);