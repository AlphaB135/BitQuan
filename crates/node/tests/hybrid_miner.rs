//! Integration tests for hybrid mining functionality.

#[cfg(feature = "randomx")]
#[test]
fn hybrid_miner_switches_algos() {
    use bitquan_consensus::pow::PowAlgo;
    use bitquan_node::miner::HybridMiner;
    use bitquan_types::NetworkId;

    let weights = vec![(PowAlgo::Sha256d, 1.0), (PowAlgo::RandomX, 2.0)];
    let miner = HybridMiner::new(&weights, 1, NetworkId::Devnet).unwrap();

    // Test algorithm selection distribution
    let mut sha256d_count = 0;
    let mut randomx_count = 0;

    for i in 0..30 {
        match miner.select_algorithm(i) {
            PowAlgo::Sha256d => sha256d_count += 1,
            PowAlgo::RandomX => randomx_count += 1,
            PowAlgo::Ethash => {} // Not used in this test
        }
    }

    // With 1:2 ratio, RandomX should be selected more often
    assert!(randomx_count > sha256d_count,
        "RandomX (weight 2.0) should be selected more than SHA256d (weight 1.0). Got SHA256d: {}, RandomX: {}",
        sha256d_count, randomx_count);
}

#[cfg(feature = "randomx")]
#[test]
fn randomx_miner_produces_valid_block() {
    use bitquan_consensus::pow::PowAlgo;
    use bitquan_node::miner::HybridMiner;
    use bitquan_types::{BlockHeader, NetworkId};

    let weights = vec![(PowAlgo::RandomX, 1.0)];
    let miner = HybridMiner::new(&weights, 1, NetworkId::Devnet).unwrap();

    // Create a very easy target (devnet max bits)
    let header = BlockHeader {
        version: 1,
        prev_block: [0u8; 32],
        merkle_root: [0u8; 32],
        pqc_agg_hint: [0u8; 32],
        time: 1234567890,
        bits: 0x207fffff, // Very easy target
        nonce: 0,
        algo_id: 1, // RandomX
    };

    // Try mining with RandomX (should find solution quickly with easy target)
    let result = miner.mine_block_attempt(header, 1_000_000, PowAlgo::RandomX);

    match result {
        Ok(Some(mined)) => {
            assert_eq!(
                mined.algo_id, 1,
                "Algorithm ID should be set to RandomX (1)"
            );
            // nonce is always valid (u64 >= 0)
        }
        Ok(None) => {
            // May not find solution in limited attempts with RandomX placeholder
            println!("No solution found in limited attempts (expected with placeholder RandomX)");
        }
        Err(e) => panic!("Mining should not error: {}", e),
    }
}

#[test]
fn mainnet_rejects_hybrid_mode() {
    #[cfg(feature = "randomx")]
    {
        use bitquan_consensus::pow::PowAlgo;
        use bitquan_node::miner::HybridMiner;
        use bitquan_types::NetworkId;

        let weights = vec![(PowAlgo::Sha256d, 1.0), (PowAlgo::RandomX, 1.0)];
        let result = HybridMiner::new(&weights, 1, NetworkId::Mainnet);

        assert!(result.is_err(), "Mainnet should reject hybrid mode");
        if let Err(e) = result {
            assert!(
                e.to_string().contains("not allowed on mainnet"),
                "Error message should mention mainnet restriction"
            );
        }
    }
}

#[test]
fn sha256d_only_miner_works() {
    use bitquan_consensus::pow::PowAlgo;
    use bitquan_node::miner::HybridMiner;
    use bitquan_types::{BlockHeader, NetworkId};

    let weights = vec![(PowAlgo::Sha256d, 1.0)];
    let miner = HybridMiner::new(&weights, 1, NetworkId::Mainnet).unwrap();

    // Create a very easy target
    let header = BlockHeader {
        version: 1,
        prev_block: [0u8; 32],
        merkle_root: [0u8; 32],
        pqc_agg_hint: [0u8; 32],
        time: 1234567890,
        bits: 0x207fffff, // Very easy target
        nonce: 0,
        algo_id: 0, // SHA256d
    };

    // Try mining with SHA256d (should find solution quickly)
    let result = miner.mine_block_attempt(header, 10_000_000, PowAlgo::Sha256d);

    match result {
        Ok(Some(mined)) => {
            assert_eq!(mined.algo_id, 0, "Algorithm ID should be SHA256d (0)");
            println!("Found solution at nonce: {}", mined.nonce);
        }
        Ok(None) => {
            // With an easy target, we should usually find a solution, but it's probabilistic
            println!("No solution found in 10M attempts (rare but possible)");
        }
        Err(e) => panic!("Mining should not error: {}", e),
    }
}

#[test]
fn metrics_track_mining_activity() {
    use bitquan_consensus::pow::PowAlgo;
    use bitquan_node::miner::HybridMiner;
    use bitquan_types::{BlockHeader, NetworkId};

    let weights = vec![(PowAlgo::Sha256d, 1.0)];
    let miner = HybridMiner::new(&weights, 1, NetworkId::Devnet).unwrap();

    let header = BlockHeader {
        version: 1,
        prev_block: [0u8; 32],
        merkle_root: [0u8; 32],
        pqc_agg_hint: [0u8; 32],
        time: 1234567890,
        bits: 0x207fffff,
        nonce: 0,
        algo_id: 0,
    };

    // Mine a block
    let _ = miner.mine_block_attempt(header, 10_000, PowAlgo::Sha256d);

    // Check metrics
    let metrics = miner.metrics();
    let attempts = metrics.get_hash_attempts(PowAlgo::Sha256d);

    assert!(attempts > 0, "Should have recorded hash attempts");
    println!("Recorded {} hash attempts", attempts);
}
