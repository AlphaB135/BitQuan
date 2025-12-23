#![no_main]

use bitquan_consensus::pow::{
    EthashConfig, EthashEngine, PowEngine, RandomXConfig, RandomXEngine, RandomXMode, Sha256dEngine,
};
use bitquan_types::BlockHeader;
use libfuzzer_sys::fuzz_target;

// Fuzz PoW engines for VM caching, DoS protection, and algorithm switching
fuzz_target!(|data: &[u8]| {
    // Ensure minimum data for block header
    if data.len() < 117 {
        return;
    }

    // Create block header from fuzz data
    let mut header_bytes = [0u8; 117];
    let copy_len = data.len().min(117);
    header_bytes[..copy_len].copy_from_slice(&data[..copy_len]);

    // Parse header fields safely
    let version = i32::from_le_bytes([
        header_bytes[0],
        header_bytes[1],
        header_bytes[2],
        header_bytes[3],
    ]);
    let mut prev_block = [0u8; 32];
    prev_block.copy_from_slice(&header_bytes[4..36]);
    let mut merkle_root = [0u8; 32];
    merkle_root.copy_from_slice(&header_bytes[36..68]);
    let mut pqc_agg_hint = [0u8; 32];
    pqc_agg_hint.copy_from_slice(&header_bytes[68..100]);
    let time = u32::from_le_bytes([
        header_bytes[100],
        header_bytes[101],
        header_bytes[102],
        header_bytes[103],
    ]);
    let bits = u32::from_le_bytes([
        header_bytes[104],
        header_bytes[105],
        header_bytes[106],
        header_bytes[107],
    ]);
    let nonce = u64::from_le_bytes([
        header_bytes[108],
        header_bytes[109],
        header_bytes[110],
        header_bytes[111],
        header_bytes[112],
        header_bytes[113],
        header_bytes[114],
        header_bytes[115],
    ]);
    let algo_id = header_bytes[116];

    let header = BlockHeader {
        version,
        prev_block,
        merkle_root,
        pqc_agg_hint,
        time,
        bits,
        nonce,
        algo_id,
    };

    // Test SHA-256d engine (always available)
    let sha256d_engine = Sha256dEngine;
    let _ = sha256d_engine.verify(&header);
    let _ = sha256d_engine.pow_hash(&header);

    // Test RandomX engine with caching
    let randomx_config = RandomXConfig {
        mode: RandomXMode::Fast,
        seed: [0u8; 32], // Use fixed seed for reproducible caching
    };
    let randomx_engine = RandomXEngine::new(randomx_config.clone());

    // Test multiple calls to exercise caching
    for _ in 0..3 {
        let _ = randomx_engine.verify(&header);
        let _ = randomx_engine.pow_hash(&header);
    }

    // Test Ethash engine with caching
    let ethash_config = EthashConfig {
        cache_size: 1024, // Fixed size for reproducible caching
        dag_size: 2048,   // Add DAG size for Ethash
    };
    let ethash_engine = EthashEngine::new(ethash_config.clone());

    // Test multiple calls to exercise caching
    for _ in 0..3 {
        let _ = ethash_engine.verify(&header);
        let _ = ethash_engine.pow_hash(&header);
    }

    // Test algorithm switching stress
    if data.len() > 200 {
        // Create multiple engines with different seeds
        for i in 0..5 {
            let mut seed = [0u8; 32];
            let seed_start = (i * 32) % (data.len() - 32);
            seed.copy_from_slice(&data[seed_start..seed_start + 32]);

            let rx_config = RandomXConfig {
                mode: if i % 2 == 0 {
                    RandomXMode::Fast
                } else {
                    RandomXMode::Full
                },
                seed,
            };

            let rx_engine = RandomXEngine::new(rx_config);
            let _ = rx_engine.verify(&header);
            let _ = rx_engine.pow_hash(&header);
        }
    }

    // Test extreme values that might cause cache issues
    if data.len() > 300 {
        // Test with maximum nonce values
        let mut extreme_header = header.clone();
        extreme_header.nonce = u64::MAX;

        let rx_engine = RandomXEngine::new(randomx_config.clone());
        let _ = rx_engine.verify(&extreme_header);
        let _ = rx_engine.pow_hash(&extreme_header);

        let ethash_engine = EthashEngine::new(ethash_config.clone());
        let _ = ethash_engine.verify(&extreme_header);
        let _ = ethash_engine.pow_hash(&extreme_header);
    }

    // Test concurrent access patterns (simulate multiple threads)
    if data.len() > 400 {
        use std::sync::Arc;
        use std::thread;

        let header_arc = Arc::new(header);
        let rx_config_arc = Arc::new(randomx_config.clone());

        let handles: Vec<_> = (0..3)
            .map(|_| {
                let header_clone = Arc::clone(&header_arc);
                let config_clone = Arc::clone(&rx_config_arc);

                thread::spawn(move || {
                    let engine = RandomXEngine::new((*config_clone).clone());
                    for _ in 0..2 {
                        let _ = engine.verify(&*header_clone);
                        let _ = engine.pow_hash(&*header_clone);
                    }
                })
            })
            .collect();

        // Wait for all threads to complete
        for handle in handles {
            let _ = handle.join();
        }
    }
});
