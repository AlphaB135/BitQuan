//! Simplified integration tests for serialization.

use bitquan_types::{Transaction, NetworkId};
use bitquan_types::genesis::GENESIS_HASH_BYTES;

#[test]
fn test_transaction_serialization_roundtrip() {
    let tx = Transaction {
        version: 1,
        network: NetworkId::Mainnet,
        genesis_hash: GENESIS_HASH_BYTES,
        sig_algo: bitquan_types::SigAlgorithm::Dilithium3,
        lock_time: 0,
        inputs: vec![],
        outputs: vec![],
        witnesses: vec![],
    };
    
    let serialized = serde_json::to_string(&tx).expect("serialize");
    let deserialized: Transaction = serde_json::from_str(&serialized).expect("deserialize");
    
    assert_eq!(tx.version, deserialized.version);
    assert_eq!(tx.network, deserialized.network);
}

#[test]
fn test_network_id_values() {
    let networks = vec![
        NetworkId::Mainnet,
        NetworkId::Devnet,
        NetworkId::Testnet,
        NetworkId::Regtest,
    ];
    
    for network in networks {
        let tx = Transaction {
            version: 1,
            network,
            genesis_hash: GENESIS_HASH_BYTES,
        sig_algo: bitquan_types::SigAlgorithm::Dilithium3,
            lock_time: 0,
            inputs: vec![],
            outputs: vec![],
            witnesses: vec![],
        };
        
        let serialized = serde_json::to_string(&tx).expect("serialize");
        let deserialized: Transaction = serde_json::from_str(&serialized).expect("deserialize");
        
        assert_eq!(deserialized.network, network);
    }
}
