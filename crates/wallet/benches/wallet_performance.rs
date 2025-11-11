use criterion::{black_box, criterion_group, criterion_main, Criterion};
use wallet::keystore::{
    encrypt_keystore, decrypt_keystore, optimal_parallelism, KdfProfile,
    DEFAULT_MEM_KIB, DEFAULT_TIME_COST,
};
use serde_json::json;
use std::time::Duration;

fn bench_encryption_decryption(c: &mut Criterion) {
    let secret = b"this-is-my-private-key-bytes-for-benchmarking-purposes";
    let password = "correct horse battery staple";
    let meta = Some(json!({"hint": "benchmark"}));
    
    // Test with different parallelism levels
    let parallelism_levels = vec![1, 2, 4, 8];
    
    for &parallelism in &parallelism_levels {
        let mut group = c.benchmark_group(format!("keystore_parallelism_{}", parallelism));
        group.measurement_time(Duration::from_secs(10));
        
        // Benchmark encryption
        group.bench_function("encrypt", |b| {
            b.iter(|| {
                let ks = encrypt_keystore(
                    black_box(secret),
                    black_box(password),
                    meta.clone(),
                    DEFAULT_MEM_KIB,
                    DEFAULT_TIME_COST,
                    parallelism,
                );
                black_box(ks)
            })
        });
        
        // Benchmark decryption
        let ks = encrypt_keystore(
            secret,
            password,
            meta.clone(),
            DEFAULT_MEM_KIB,
            DEFAULT_TIME_COST,
            parallelism,
        );
        
        group.bench_function("decrypt", |b| {
            b.iter(|| {
                let pt = decrypt_keystore(black_box(&ks), black_box(password)).unwrap();
                black_box(pt)
            })
        });
        
        group.finish();
    }
}

fn bench_kdf_profiles(c: &mut Criterion) {
    let secret = b"benchmark-secret-key";
    let password = "benchmark-password";
    let meta = Some(json!({"profile": "test"}));
    
    let profiles = vec![
        ("Tight", KdfProfile::Tight),
        ("Medium", KdfProfile::Medium),
        ("Light", KdfProfile::Light),
        ("Mobile", KdfProfile::Mobile),
    ];
    
    for (name, profile) in profiles {
        let (mem_kib, time_cost, parallelism) = profile.params();
        let mut group = c.benchmark_group(format!("kdf_profile_{}", name.to_lowercase()));
        group.measurement_time(Duration::from_secs(15));
        
        // Benchmark full encrypt/decrypt cycle
        group.bench_function("full_cycle", |b| {
            b.iter(|| {
                let ks = encrypt_keystore(
                    black_box(secret),
                    black_box(password),
                    meta.clone(),
                    mem_kib,
                    time_cost,
                    parallelism,
                );
                let pt = decrypt_keystore(black_box(&ks), black_box(password)).unwrap();
                black_box(pt)
            })
        });
        
        group.finish();
    }
}

fn bench_optimal_parallelism(c: &mut Criterion) {
    let secret = b"optimal-parallelism-test";
    let password = "test-password";
    let meta = Some(json!({"test": "optimal"}));
    
    let optimal = optimal_parallelism();
    let mut group = c.benchmark_group("optimal_parallelism");
    group.measurement_time(Duration::from_secs(10));
    
    // Compare optimal vs single-thread
    for &parallelism in &[1, optimal] {
        group.bench_function(format!("threads_{}", parallelism), |b| {
            b.iter(|| {
                let ks = encrypt_keystore(
                    black_box(secret),
                    black_box(password),
                    meta.clone(),
                    DEFAULT_MEM_KIB,
                    DEFAULT_TIME_COST,
                    parallelism,
                );
                let pt = decrypt_keystore(black_box(&ks), black_box(password)).unwrap();
                black_box(pt)
            })
        });
    }
    
    group.finish();
}

fn bench_buffer_pooling(c: &mut Criterion) {
    let secret = b"buffer-pooling-test";
    let password = "test-password";
    let meta = Some(json!({"test": "buffer"}));
    
    let mut group = c.benchmark_group("buffer_pooling");
    group.measurement_time(Duration::from_secs(10));
    
        // Test multiple encryptions to benefit from buffer pooling
        group.bench_function("multiple_encryptions", |b| {
            b.iter(|| {
                let mut results = Vec::new();
                for _ in 0..10 {
                    let ks = encrypt_keystore(
                        black_box(secret),
                        black_box(password),
                        meta.clone(),
                        DEFAULT_MEM_KIB,
                        DEFAULT_TIME_COST,
                        optimal_parallelism(),
                    );
                    results.push(ks);
                }
                black_box(results)
            })
        });
    
    group.finish();
}

criterion_group!(
    benches,
    bench_encryption_decryption,
    bench_kdf_profiles,
    bench_optimal_parallelism,
    bench_buffer_pooling
);
criterion_main!(benches);