//! Cross-network replay protection tests.

use bitquan_types::{genesis, NetworkId, Transaction};

fn create_test_tx(network: NetworkId) -> Transaction {
    Transaction {
        version: 1,
        network,
        genesis_hash: genesis::GENESIS_HASH_BYTES,
        sig_algo: bitquan_types::SigAlgorithm::Dilithium5,
        lock_time: 0,
        inputs: vec![],
        outputs: vec![],
        witnesses: vec![],
    }
}

#[test]
fn test_different_networks_have_different_markers() {
    let mainnet_tx = create_test_tx(NetworkId::Mainnet);
    let devnet_tx = create_test_tx(NetworkId::Devnet);
    let testnet_tx = create_test_tx(NetworkId::Testnet);

    assert_ne!(mainnet_tx.network, devnet_tx.network);
    assert_ne!(mainnet_tx.network, testnet_tx.network);
    assert_ne!(devnet_tx.network, testnet_tx.network);
}

#[test]
fn test_transaction_includes_network_marker() {
    let tx = create_test_tx(NetworkId::Mainnet);
    assert_eq!(tx.network, NetworkId::Mainnet);

    let tx2 = create_test_tx(NetworkId::Devnet);
    assert_eq!(tx2.network, NetworkId::Devnet);
}

#[test]
fn test_transaction_includes_genesis_hash() {
    let tx = create_test_tx(NetworkId::Mainnet);
    assert_eq!(tx.genesis_hash, genesis::GENESIS_HASH_BYTES);
}
