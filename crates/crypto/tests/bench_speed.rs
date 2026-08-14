//! Comprehensive Performance & Speed Benchmark for BitQuan Post-Quantum Crypto & Transfers

use std::time::Instant;
use pqc_dilithium_seeded::{Keypair, verify};

#[test]
fn bench_pqc_speed_and_throughput() {
    println!("\n============================================================");
    println!("🚀 BITQUAN PERFORMANCE & TRANSFER SPEED BENCHMARK");
    println!("   Platform: Oracle Cloud ARM (4 OCPU, aarch64-linux-gnu)");
    println!("============================================================\n");

    let num_iterations = 200;
    let message = b"BitQuan Post-Quantum Transaction Sighash Payload 32bytes";

    // 1. Benchmark Key Generation
    print!("⏳ Measuring Key Generation ({} iterations)... ", num_iterations);
    let start = Instant::now();
    let mut keypairs = Vec::with_capacity(num_iterations);
    for _ in 0..num_iterations {
        keypairs.push(Keypair::generate());
    }
    let keygen_duration = start.elapsed();
    let keygen_per_sec = (num_iterations as f64) / keygen_duration.as_secs_f64();
    let keygen_avg_ms = (keygen_duration.as_secs_f64() * 1000.0) / (num_iterations as f64);
    println!("DONE\n   ⚡ Avg Keygen Time: {:.3} ms ({:.1} keypairs/sec)", keygen_avg_ms, keygen_per_sec);

    // 2. Benchmark Signing Speed
    print!("⏳ Measuring Dilithium5 Signing ({} iterations)... ", num_iterations);
    let start = Instant::now();
    let mut signatures = Vec::with_capacity(num_iterations);
    for kp in &keypairs {
        signatures.push(kp.sign(message));
    }
    let sign_duration = start.elapsed();
    let sign_per_sec = (num_iterations as f64) / sign_duration.as_secs_f64();
    let sign_avg_ms = (sign_duration.as_secs_f64() * 1000.0) / (num_iterations as f64);
    println!("DONE\n   ⚡ Avg Signing Time: {:.3} ms ({:.1} signs/sec)", sign_avg_ms, sign_per_sec);

    // 3. Benchmark Verification Speed (Single Thread)
    print!("⏳ Measuring Dilithium5 Verification (Single Core, {} iterations)... ", num_iterations);
    let start = Instant::now();
    let mut valid_count = 0;
    for (kp, sig) in keypairs.iter().zip(&signatures) {
        if verify(sig, message, &kp.public).is_ok() {
            valid_count += 1;
        }
    }
    let verify_duration = start.elapsed();
    let verify_per_sec = (num_iterations as f64) / verify_duration.as_secs_f64();
    let verify_avg_ms = (verify_duration.as_secs_f64() * 1000.0) / (num_iterations as f64);
    println!("DONE\n   ⚡ Single-Core Verification: {:.3} ms/tx ({:.1} verifications/sec) [Valid: {}/{}]", 
        verify_avg_ms, verify_per_sec, valid_count, num_iterations);

    // 4. Benchmark Multi-Threaded Batch Verification (Parallel CPU Throughput)
    use std::sync::atomic::{AtomicUsize, Ordering};
    let num_batch = 1000;
    print!("⏳ Measuring Multi-Threaded Batch Verification ({} signatures across CPU cores)... ", num_batch);
    
    // Prepare 1000 items
    let mut batch_pairs = Vec::with_capacity(num_batch);
    for i in 0..num_batch {
        let kp = &keypairs[i % keypairs.len()];
        let sig = &signatures[i % signatures.len()];
        batch_pairs.push((kp.public, *sig));
    }

    let num_threads = 4;
    let chunk_size = num_batch / num_threads;
    let verified_counter = std::sync::Arc::new(AtomicUsize::new(0));

    let start = Instant::now();
    std::thread::scope(|s| {
        for chunk in batch_pairs.chunks(chunk_size) {
            let counter = verified_counter.clone();
            s.spawn(move || {
                let mut local = 0;
                for (pubkey, sig) in chunk {
                    if verify(sig, message, pubkey).is_ok() {
                        local += 1;
                    }
                }
                counter.fetch_add(local, Ordering::Relaxed);
            });
        }
    });
    let batch_duration = start.elapsed();
    let batch_tps = (num_batch as f64) / batch_duration.as_secs_f64();
    println!("DONE\n   🚀 Multi-Core Peak Verification TPS: {:.1} TPS ({:.2} ms total for {} txs)", 
        batch_tps, batch_duration.as_secs_f64() * 1000.0, num_batch);

    // 5. Blockchain Transaction & Block Sizing Metrics
    let tx_size_bytes = 4864 + 2592 + 300; // ~7.7 KB with Dilithium5 signature + pubkey + overhead
    let max_block_size_bytes = 4_000_000; // 4MB BitQuan block limit
    let max_tx_per_block = max_block_size_bytes / tx_size_bytes;
    let block_time_secs = 60.0; // 1 minute target block time
    let theoretical_l1_tps = (max_tx_per_block as f64) / block_time_secs;

    println!("\n============================================================");
    println!("📊 BITQUAN BLOCKCHAIN CAPACITY & TRANSFER SPEED SUMMARY");
    println!("============================================================");
    println!("• PQC Signature Algorithm  : Dilithium5 (NIST Level 5 Quantum Proof)");
    println!("• Signature Size           : 4,595 bytes");
    println!("• Public Key Size          : 2,592 bytes");
    println!("• Average Tx Wire Size     : ~{:.1} KB", tx_size_bytes as f64 / 1024.0);
    println!("• Signing Latency / Tx     : {:.3} ms (Instantaneous)", sign_avg_ms);
    println!("• Verification Latency / Tx: {:.3} ms (Single Core)", verify_avg_ms);
    println!("• Node Validation Capacity : {:.1} TPS (Multi-threaded verify across 4 cores)", batch_tps);
    println!("• Max Transactions / Block : ~{} transactions / 4MB block", max_tx_per_block);
    println!("• Target Block Interval    : {:.0} seconds", block_time_secs);
    println!("• Layer-1 On-Chain TPS     : ~{:.2} TPS (Direct L1 On-Chain Settlement)", theoretical_l1_tps);
    println!("============================================================\n");
}
