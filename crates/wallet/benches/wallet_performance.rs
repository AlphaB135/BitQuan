use criterion::{black_box, criterion_group, criterion_main, Criterion};
use wallet::keystore::{
    encrypt_keystore, decrypt_keystore, optimal_parallelism, KdfProfile,
    encrypt_keystore_adaptive, encrypt_keystore_with_profile, HardwareProfile,
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

fn bench_adaptive_vs_fixed(c: &mut Criterion) {
    let secret = b"adaptive-vs-fixed-test";
    let password = "test-password";
    let meta = Some(json!({"benchmark": "adaptive"}));
    
    let mut group = c.benchmark_group("adaptive_vs_fixed");
    group.measurement_time(Duration::from_secs(15));
    
    // Benchmark adaptive encryption
    group.bench_function("adaptive_encryption", |b| {
        b.iter(|| {
            let ks = encrypt_keystore_adaptive(
                black_box(secret),
                black_box(password),
                meta.clone(),
            );
            black_box(ks)
        })
    });
    
    // Benchmark fixed Tight profile
    group.bench_function("fixed_tight_encryption", |b| {
        b.iter(|| {
            let ks = encrypt_keystore_with_profile(
                black_box(secret),
                black_box(password),
                meta.clone(),
                KdfProfile::Tight,
            );
            black_box(ks)
        })
    });
    
    // Benchmark fixed Mobile profile
    group.bench_function("fixed_mobile_encryption", |b| {
        b.iter(|| {
            let ks = encrypt_keystore_with_profile(
                black_box(secret),
                black_box(password),
                meta.clone(),
                KdfProfile::Mobile,
            );
            black_box(ks)
        })
    });
    
    // Benchmark decryption performance
    let ks_adaptive = encrypt_keystore_adaptive(secret, password, meta.clone());
    let ks_tight = encrypt_keystore_with_profile(secret, password, meta.clone(), KdfProfile::Tight);
    let ks_mobile = encrypt_keystore_with_profile(secret, password, meta.clone(), KdfProfile::Mobile);
    
    group.bench_function("adaptive_decryption", |b| {
        b.iter(|| {
            let pt = decrypt_keystore(black_box(&ks_adaptive), black_box(password)).unwrap();
            black_box(pt)
        })
    });
    
    group.bench_function("fixed_tight_decryption", |b| {
        b.iter(|| {
            let pt = decrypt_keystore(black_box(&ks_tight), black_box(password)).unwrap();
            black_box(pt)
        })
    });
    
    group.bench_function("fixed_mobile_decryption", |b| {
        b.iter(|| {
            let pt = decrypt_keystore(black_box(&ks_mobile), black_box(password)).unwrap();
            black_box(pt)
        })
    });
    
    group.finish();
}

fn bench_hardware_profiles(c: &mut Criterion) {
    let secret = b"hardware-profile-test";
    let password = "test-password";
    let meta = Some(json!({"hardware": "test"}));
    
    let profiles = vec![
        ("HighEndDesktop", HardwareProfile::HighEndDesktop),
        ("MidRangeLaptop", HardwareProfile::MidRangeLaptop),
        ("LowEndDevice", HardwareProfile::LowEndDevice),
        ("MobileDevice", HardwareProfile::MobileDevice),
    ];
    
    for (name, hw_profile) in profiles {
        let mut group = c.benchmark_group(format!("hardware_profile_{}", name.to_lowercase()));
        group.measurement_time(Duration::from_secs(10));
        
        let (mem_kib, time_cost, parallelism) = KdfProfile::adaptive_params_with_hardware(hw_profile);
        
        group.bench_function("encrypt_decrypt_cycle", |b| {
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

criterion_group!(
    benches,
    bench_encryption_decryption,
    bench_kdf_profiles,
    bench_optimal_parallelism,
    bench_buffer_pooling,
    bench_adaptive_vs_fixed,
    bench_hardware_profiles
);
criterion_main!(benches);