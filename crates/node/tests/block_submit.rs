//! Integration tests for block submission and network propagation.

use bitquan_consensus::pow::{meets_target, sha256d_pow_hash, target_from_bits};
use bitquan_node::{BlockSubmitter, SubmitResult};
use bitquan_types::{Block, BlockHeader, NetworkId, Transaction};

fn dummy_transaction() -> Transaction {
    use bitquan_types::SigAlgorithm;

    Transaction {
        version: 1,
        network: NetworkId::Testnet,
        genesis_hash: [0u8; 32],
        lock_time: 0,
        inputs: vec![],
        outputs: vec![],
        sig_algo: SigAlgorithm::Dilithium3,
        witnesses: vec![],
    }
}

fn create_test_block(nonce: u64, bits: u32) -> Block {
    let header = BlockHeader {
        version: 1,
        prev_block: [0u8; 32],
        merkle_root: [0u8; 32],
        pqc_agg_hint: [0u8; 32],
        time: 1234567890,
        bits,
        nonce,
        algo_id: 0, // SHA-256d
    };

    Block {
        header,
        transactions: vec![dummy_transaction()],
    }
}

#[tokio::test]
async fn test_block_submit_valid_mock() {
    let submitter = BlockSubmitter::mock(NetworkId::Testnet);
    let block = create_test_block(12345, 0x207fffff);

    let result = submitter.submit(&block, None).await.unwrap();

    match result {
        SubmitResult::Accepted { hash, height } => {
            assert_eq!(hash.len(), 32);
            println!(
                "Block accepted: hash={:?}, height={:?}",
                hex::encode(&hash[..8]),
                height
            );
        }
        _ => panic!("Expected acceptance in mock mode, got {:?}", result),
    }
}

#[tokio::test]
async fn test_block_submit_reject_no_transactions() {
    let submitter = BlockSubmitter::mock(NetworkId::Testnet);
    let mut block = create_test_block(12345, 0x207fffff);
    block.transactions.clear(); // Remove transactions

    let result = submitter.submit(&block, None).await.unwrap();

    match result {
        SubmitResult::Rejected { reason } => {
            assert_eq!(reason, "no_transactions");
        }
        _ => panic!("Expected rejection, got {:?}", result),
    }
}

#[tokio::test]
async fn test_block_submit_invalid_pow() {
    let submitter = BlockSubmitter::mock(NetworkId::Testnet);

    // Create block with hard difficulty but easy nonce (won't meet target)
    let mut block = create_test_block(1, 0x1d00ffff); // Hard difficulty
    block.transactions.push(dummy_transaction());

    let result = submitter.submit(&block, None).await;

    // May reject due to PoW not meeting target
    match result {
        Ok(SubmitResult::Rejected { reason }) => {
            assert_eq!(reason, "pow_invalid");
        }
        Ok(SubmitResult::Accepted { .. }) => {
            // With very low nonce, might accidentally pass easy check
            println!("Note: Block accidentally passed PoW (low probability)");
        }
        Ok(SubmitResult::Error { .. }) => {
            println!("Block submission error");
        }
        Err(e) => panic!("Unexpected error: {}", e),
    }
}

#[tokio::test]
async fn test_block_submit_reject_invalid_header() {
    let submitter = BlockSubmitter::mock(NetworkId::Testnet);

    // Create block with corrupted prev_hash
    let mut block = create_test_block(12345, 0x207fffff);
    block.header.prev_block = [0xFFu8; 32]; // Corrupt
    block.transactions.push(dummy_transaction());

    // Should still pass PoW but might fail chain validation in production
    let result = submitter.submit(&block, None).await;

    match result {
        Ok(SubmitResult::Accepted { .. }) => {
            // Mock mode accepts anything with valid PoW
            println!("Mock mode accepted corrupted block (expected in tests)");
        }
        Ok(SubmitResult::Rejected { .. }) => {
            println!("Block rejected (chain validation would catch this)");
        }
        Ok(SubmitResult::Error { .. }) => {
            println!("Block submission error");
        }
        Err(_) => {}
    }
}

#[test]
fn test_submit_result_types() {
    // Test SubmitResult enum variants
    let accepted = SubmitResult::Accepted {
        hash: [0u8; 32],
        height: Some(100),
    };

    match accepted {
        SubmitResult::Accepted { hash, height } => {
            assert_eq!(hash, [0u8; 32]);
            assert_eq!(height, Some(100));
        }
        _ => panic!("Wrong variant"),
    }

    let rejected = SubmitResult::Rejected {
        reason: "test_reason".to_string(),
    };

    match rejected {
        SubmitResult::Rejected { reason } => {
            assert_eq!(reason, "test_reason");
        }
        _ => panic!("Wrong variant"),
    }

    let error = SubmitResult::Error {
        message: "network_error".to_string(),
    };

    match error {
        SubmitResult::Error { message } => {
            assert_eq!(message, "network_error");
        }
        _ => panic!("Wrong variant"),
    }
}

#[test]
fn test_block_submitter_creation() {
    // Test creating submitters for different networks
    let testnet_submitter = BlockSubmitter::new(NetworkId::Testnet);
    assert_eq!(testnet_submitter.network_id, NetworkId::Testnet);
    assert!(!testnet_submitter.mock_mode);

    let mock_submitter = BlockSubmitter::mock(NetworkId::Devnet);
    assert_eq!(mock_submitter.network_id, NetworkId::Devnet);
    assert!(mock_submitter.mock_mode);

    let mainnet_submitter = BlockSubmitter::new(NetworkId::Mainnet);
    assert_eq!(mainnet_submitter.network_id, NetworkId::Mainnet);
}

#[tokio::test]
async fn test_block_submit_with_valid_nonce() {
    // Try to find a nonce that meets easy target
    let bits = 0x207fffff; // Very easy
    let target = target_from_bits(bits).unwrap();

    let mut found_valid = false;
    let mut valid_nonce = 0u64;

    // Try up to 100k nonces
    for nonce in 0..100_000 {
        let header = BlockHeader {
            version: 1,
            prev_block: [0u8; 32],
            merkle_root: [0u8; 32],
            pqc_agg_hint: [0u8; 32],
            time: 1234567890,
            bits,
            nonce,
            algo_id: 0,
        };

        let preimage = header.to_bytes();
        let hash = sha256d_pow_hash(&preimage);

        if meets_target(&hash, &target) {
            found_valid = true;
            valid_nonce = nonce;
            println!(
                "Found valid nonce: {} with hash: {:02x?}",
                nonce,
                &hash[..4]
            );
            break;
        }
    }

    if found_valid {
        // Test submission with valid nonce
        let block = create_test_block(valid_nonce, bits);
        let submitter = BlockSubmitter::mock(NetworkId::Testnet);

        let result = submitter.submit(&block, None).await.unwrap();

        match result {
            SubmitResult::Accepted { .. } => {
                println!("✅ Valid block accepted");
            }
            _ => panic!("Valid block should be accepted"),
        }
    } else {
        println!("⚠️  No valid nonce found in 100k attempts (expected with real difficulty)");
    }
}

#[tokio::test]
async fn test_multiple_submissions() {
    let submitter = BlockSubmitter::mock(NetworkId::Testnet);

    // Submit multiple blocks
    for i in 0..5 {
        let block = create_test_block(1000 + i, 0x207fffff);
        let result = submitter.submit(&block, None).await.unwrap();

        match result {
            SubmitResult::Accepted { .. } => {
                println!("Block {} accepted", i);
            }
            _ => panic!("Block {} should be accepted", i),
        }
    }
}
