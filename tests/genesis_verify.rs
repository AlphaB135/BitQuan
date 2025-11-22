//! Integration test: Genesis block validation
//!
//! Validates mainnet and testnet genesis configurations

use serde_json::Value;
use std::fs;

#[test]
fn test_mainnet_genesis_exists() {
    let genesis_path = "genesis/mainnet.json";
    assert!(
        std::path::Path::new(genesis_path).exists(),
        "Mainnet genesis file must exist"
    );
}

#[test]
fn test_testnet_genesis_exists() {
    let genesis_path = "genesis/testnet.json";
    assert!(
        std::path::Path::new(genesis_path).exists(),
        "Testnet genesis file must exist"
    );
}

#[test]
fn test_mainnet_genesis_valid_json() {
    let content = fs::read_to_string("genesis/mainnet.json")
        .expect("Failed to read mainnet genesis");

    let genesis: Value = serde_json::from_str(&content)
        .expect("Mainnet genesis must be valid JSON");

    // Verify required fields
    assert!(genesis["chain_id"].is_string(), "chain_id must be present");
    assert!(genesis["network_id"].is_string(), "network_id must be present");
    assert!(genesis["genesis_hash"].is_string(), "genesis_hash must be present");
    assert!(genesis["genesis_timestamp"].is_number(), "genesis_timestamp must be present");

    // Verify mainnet specific values
    assert_eq!(
        genesis["network_id"].as_str().expect("network_id must be a string"),
        "mainnet",
        "network_id must be 'mainnet'"
    );
}

#[test]
fn test_testnet_genesis_valid_json() {
    let content = fs::read_to_string("genesis/testnet.json")
        .expect("Failed to read testnet genesis");

    let genesis: Value = serde_json::from_str(&content)
        .expect("Testnet genesis must be valid JSON");

    assert!(genesis["chain_id"].is_string(), "chain_id must be present");
    assert_eq!(
        genesis["network_id"].as_str().expect("network_id must be a string"),
        "testnet",
        "network_id must be 'testnet'"
    );
}

#[test]
fn test_mainnet_genesis_hash_format() {
    let content = fs::read_to_string("genesis/mainnet.json")
        .expect("Failed to read mainnet genesis");
    let genesis: Value = serde_json::from_str(&content).expect("Failed to parse mainnet genesis JSON");

    let hash = genesis["genesis_hash"].as_str().expect("genesis_hash must be a string");

    // Genesis hash must be 64 hex characters (32 bytes)
    assert_eq!(hash.len(), 64, "Genesis hash must be 64 hex characters");
    assert!(
        hash.chars().all(|c| c.is_ascii_hexdigit()),
        "Genesis hash must contain only hex characters"
    );

    // Bitcoin-style genesis hashes start with multiple zeros
    assert!(
        hash.starts_with("00000"),
        "Genesis hash should start with leading zeros for PoW"
    );
}

#[test]
fn test_mainnet_consensus_params() {
    let content = fs::read_to_string("genesis/mainnet.json")
        .expect("Failed to read mainnet genesis");
    let genesis: Value = serde_json::from_str(&content).expect("Failed to parse mainnet genesis JSON");

    let params = &genesis["consensus_params"];

    assert!(params["target_block_time"].is_number());
    assert!(params["max_block_size"].is_number());
    assert!(params["initial_subsidy"].is_number());
    assert!(params["subsidy_halving_interval"].is_number());

    // Mainnet should use SHA-256d
    assert_eq!(
        params["pow_algo"].as_str().expect("pow_algo must be a string"),
        "sha256d",
        "Mainnet must use SHA-256d PoW"
    );
}

#[test]
fn test_mainnet_pqc_signature_present() {
    let content = fs::read_to_string("genesis/mainnet.json")
        .expect("Failed to read mainnet genesis");
    let genesis: Value = serde_json::from_str(&content).expect("Failed to parse mainnet genesis JSON");

    let pqc_sig = &genesis["pqc_signature"];

    assert!(pqc_sig["algorithm"].is_string(), "PQC algorithm must be specified");
    assert!(pqc_sig["public_key"].is_string(), "PQC public key must be present");
    assert!(pqc_sig["signature"].is_string(), "PQC signature must be present");

    assert_eq!(
        pqc_sig["algorithm"].as_str().expect("algorithm must be a string"),
        "dilithium3",
        "Must use Dilithium3 for genesis signing"
    );
}

#[test]
fn test_mainnet_dns_seeds() {
    let content = fs::read_to_string("genesis/mainnet.json")
        .expect("Failed to read mainnet genesis");
    let genesis: Value = serde_json::from_str(&content).unwrap();

    let seeds = genesis["dns_seeds"].as_array().expect("dns_seeds must be an array");

    assert!(
        seeds.len() >= 3,
        "Mainnet must have at least 3 DNS seeds for redundancy"
    );

    for seed in seeds {
        let seed_str = seed.as_str().expect("seed must be a string");
        assert!(
            seed_str.contains(".bitquan.network"),
            "DNS seeds should use official domain"
        );
    }
}

#[test]
fn test_mainnet_bootstrap_peers() {
    let content = fs::read_to_string("genesis/mainnet.json")
        .expect("Failed to read mainnet genesis");
    let genesis: Value = serde_json::from_str(&content).expect("Failed to parse mainnet genesis JSON");

    let peers = genesis["bootstrap_peers"].as_array().expect("bootstrap_peers must be an array");

    assert!(
        peers.len() >= 2,
        "Mainnet must have at least 2 bootstrap peers"
    );

    for peer in peers {
        let peer_str = peer.as_str().expect("peer must be a string");
        assert!(
            peer_str.contains(':'),
            "Bootstrap peer must include port: {}",
            peer_str
        );
    }
}

#[test]
fn test_mainnet_min_client_version() {
    let content = fs::read_to_string("genesis/mainnet.json")
        .expect("Failed to read mainnet genesis");
    let genesis: Value = serde_json::from_str(&content).expect("Failed to parse mainnet genesis JSON");

    let version = genesis["min_client_version"].as_str().expect("min_client_version must be a string");

    // Version should be in semver format
    let parts: Vec<&str> = version.split('.').collect();
    assert_eq!(parts.len(), 3, "Version must be in semver format (X.Y.Z)");

    // For mainnet launch, should be >= 1.0.0
    let major: u32 = parts[0].parse().expect("Failed to parse major version");
    assert!(major >= 1, "Mainnet requires version >= 1.0.0");
}

#[test]
fn test_testnet_allows_premine() {
    let content = fs::read_to_string("genesis/testnet.json")
        .expect("Failed to read testnet genesis");
    let genesis: Value = serde_json::from_str(&content).expect("Failed to parse testnet genesis JSON");

    // Testnet may have premine for development
    if let Some(premine) = genesis.get("premine") {
        assert!(premine["total_amount"].is_number());
    }
}

#[test]
fn test_mainnet_no_premine() {
    let content = fs::read_to_string("genesis/mainnet.json")
        .expect("Failed to read mainnet genesis");
    let genesis: Value = serde_json::from_str(&content).expect("Failed to parse mainnet genesis JSON");

    let premine = &genesis["premine"];
    let total = premine["total_amount"].as_u64().expect("total_amount must be a number");

    assert_eq!(
        total, 0,
        "Mainnet must not have premine (fair launch)"
    );
}

#[test]
fn test_genesis_block_structure() {
    let content = fs::read_to_string("genesis/mainnet.json")
        .expect("Failed to read mainnet genesis");
    let genesis: Value = serde_json::from_str(&content).expect("Failed to parse mainnet genesis JSON");

    let block = &genesis["genesis_block"];

    assert_eq!(block["version"].as_u64().unwrap(), 1);
    assert_eq!(block["height"].as_u64().unwrap(), 0);
    assert!(block["timestamp"].is_number());
    assert!(block["merkle_root"].is_string());
    assert!(block["bits"].is_number());
    assert!(block["nonce"].is_number());
    assert!(block["transactions"].is_array());

    // Genesis block must have exactly one coinbase transaction
    let txs = block["transactions"].as_array().unwrap();
    assert_eq!(txs.len(), 1, "Genesis block must have exactly one transaction");
}

#[test]
fn test_genesis_coinbase_transaction() {
    let content = fs::read_to_string("genesis/mainnet.json")
        .expect("Failed to read mainnet genesis");
    let genesis: Value = serde_json::from_str(&content).unwrap();

    let tx = &genesis["genesis_block"]["transactions"][0];

    assert!(tx["inputs"].is_array());
    assert!(tx["outputs"].is_array());

    let inputs = tx["inputs"].as_array().unwrap();
    assert_eq!(inputs.len(), 1, "Coinbase must have exactly one input");

    // Coinbase input has special coinbase field
    assert!(inputs[0]["coinbase"].is_string());

    let outputs = tx["outputs"].as_array().unwrap();
    assert!(!outputs.is_empty(), "Genesis coinbase must have outputs");
}

#[test]
fn test_mainnet_testnet_different_hashes() {
    let mainnet = fs::read_to_string("genesis/mainnet.json").unwrap();
    let testnet = fs::read_to_string("genesis/testnet.json").unwrap();

    let mainnet_json: Value = serde_json::from_str(&mainnet).unwrap();
    let testnet_json: Value = serde_json::from_str(&testnet).unwrap();

    let mainnet_hash = mainnet_json["genesis_hash"].as_str().unwrap();
    let testnet_hash = testnet_json["genesis_hash"].as_str().unwrap();

    assert_ne!(
        mainnet_hash, testnet_hash,
        "Mainnet and testnet must have different genesis hashes"
    );
}
