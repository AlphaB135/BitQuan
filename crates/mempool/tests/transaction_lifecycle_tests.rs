//! Integration tests for mempool transaction lifecycle.

use bitquan_mempool::Mempool;
use bitquan_types::genesis::GENESIS_HASH_BYTES;
use bitquan_types::{NetworkId, SignaturePayload, Transaction, TxIn, TxOut, Witness};

fn create_test_transaction(nonce: u8) -> Transaction {
    Transaction {
        version: 1,
        network: NetworkId::Devnet,
        genesis_hash: GENESIS_HASH_BYTES,
        sig_algo: bitquan_types::SigAlgorithm::Dilithium3,
        inputs: vec![TxIn {
            prev_txid: GENESIS_HASH_BYTES,
            prev_vout: nonce as u32,
            sequence: 0xFFFFFFFF,
            script_sig: vec![],
        }],
        outputs: vec![TxOut {
            value: 1000,
            script_pubkey: vec![0xAA; 32],
        }],
        witnesses: vec![Witness {
            signatures: vec![SignaturePayload {
                signer_index: 0,
                signature: vec![0xBB; 3293],
                public_key: vec![0xCC; 1952],
                aux: None,
            }],
        }],
        lock_time: 0,
    }
}

#[test]
fn test_transaction_add_success() {
    let mut mempool = Mempool::new().expect("mempool creation");
    let tx = create_test_transaction(1);

    // Add transaction
    let result = mempool.insert(tx.clone(), 5000);
    assert!(result.is_ok(), "insert should succeed");

    // Verify mempool is not empty
    assert!(!mempool.is_empty(), "mempool should not be empty");
    assert_eq!(mempool.len(), 1, "mempool should have 1 transaction");
}

#[test]
fn test_mempool_size_tracking() {
    let mut mempool = Mempool::new().expect("mempool creation");
    let initial_size = mempool.size_bytes();

    let tx = create_test_transaction(2);
    mempool.insert(tx, 3000).expect("insert");

    let after_size = mempool.size_bytes();
    assert!(
        after_size > initial_size,
        "size should increase after insertion"
    );
}

#[test]
fn test_multiple_transactions() {
    let mut mempool = Mempool::new().expect("mempool creation");

    // Add multiple transactions with different inputs
    let tx1 = create_test_transaction(3);
    let tx2 = create_test_transaction(4);
    let tx3 = create_test_transaction(5);

    mempool.insert(tx1, 5000).expect("insert tx1");
    mempool.insert(tx2, 6000).expect("insert tx2");
    mempool.insert(tx3, 4000).expect("insert tx3");

    assert_eq!(mempool.len(), 3, "should have 3 transactions");
}

#[test]
fn test_mempool_fee_rate_policy() {
    let mut mempool = Mempool::new().expect("mempool creation");

    let tx1 = create_test_transaction(10);
    let tx2 = create_test_transaction(11);
    let tx3 = create_test_transaction(12);

    // Insert with different fees (high enough to pass minimum fee rate)
    mempool.insert(tx1, 10_000).expect("insert tx1");
    mempool.insert(tx2, 50_000).expect("insert tx2");
    mempool.insert(tx3, 20_000).expect("insert tx3");

    // Verify all were accepted
    assert_eq!(mempool.len(), 3, "all transactions should be accepted");

    // Check policy
    let min_fee = mempool.min_fee_rate();
    assert!(min_fee > 0, "minimum fee rate should be positive");
}

#[test]
fn test_mempool_size_limit() {
    use bitquan_consensus::MempoolPolicy;

    let policy = MempoolPolicy::standard();
    // Create small mempool with 10KB limit (enough for ~2 Dilithium transactions)
    let mut mempool = Mempool::with_limits(policy, 10_000).expect("mempool creation");

    let tx = create_test_transaction(20);

    // First transaction should fit
    mempool
        .insert(tx.clone(), 1000)
        .expect("first insert should succeed");

    // Additional transactions may be rejected if size limit exceeded
    let mut inserted_count = 1;
    for i in 21..30 {
        let tx_i = create_test_transaction(i);
        if mempool.insert(tx_i, 1000).is_ok() {
            inserted_count += 1;
        }
    }

    // Should have some limit enforcement (10KB allows ~2 transactions)
    assert!(
        inserted_count < 9,
        "size limit should prevent all insertions"
    );
}

#[test]
fn test_mempool_empty_state() {
    let mempool = Mempool::new().expect("mempool creation");

    assert!(mempool.is_empty(), "new mempool should be empty");
    assert_eq!(mempool.len(), 0, "new mempool should have zero length");
    assert_eq!(mempool.size_bytes(), 0, "new mempool should have zero size");
}

#[test]
fn test_low_fee_rejection() {
    let mut mempool = Mempool::new().expect("mempool creation");

    let tx = create_test_transaction(40);

    // Try to insert with very low fee (should be rejected by policy)
    let result = mempool.insert(tx, 1);

    // Expect rejection due to minimum fee rate policy
    assert!(result.is_err(), "very low fee should be rejected");
}
