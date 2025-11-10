use std::fs;
use serde_json::Value;

#[test]
fn test_genesis_verify_mainnet_ok() {
    let genesis_path = "genesis/mainnet.json";
    
    assert!(
        std::path::Path::new(genesis_path).exists(),
        "Mainnet genesis file must exist"
    );
    
    let content = fs::read_to_string(genesis_path)
        .expect("Failed to read mainnet genesis");
    
    let genesis: Value = serde_json::from_str(&content)
        .expect("Genesis must be valid JSON");
    
    // Verify required fields
    assert!(genesis.get("genesis_hash").is_some(), "genesis_hash must be present");
    assert!(genesis.get("chain_id").is_some(), "chain_id must be present");
    assert!(genesis.get("network_id").is_some(), "network_id must be present");
    assert!(genesis.get("consensus_params").is_some(), "consensus_params must be present");
    
    // Verify network_id is mainnet
    assert_eq!(
        genesis["network_id"].as_str().expect("network_id must be a string"),
        "mainnet",
        "network_id must be 'mainnet'"
    );
    
    // Verify genesis hash format (64 hex chars)
    let hash = genesis["genesis_hash"].as_str().expect("genesis_hash must be a string");
    assert_eq!(hash.len(), 64, "Genesis hash must be 64 characters");
    assert!(
        hash.chars().all(|c| c.is_ascii_hexdigit()),
        "Genesis hash must be hexadecimal"
    );
}

#[test]
fn test_genesis_verify_testnet_ok() {
    let genesis_path = "genesis/testnet.json";
    
    assert!(
        std::path::Path::new(genesis_path).exists(),
        "Testnet genesis file must exist"
    );
    
    let content = fs::read_to_string(genesis_path)
        .expect("Failed to read testnet genesis");
    
    let genesis: Value = serde_json::from_str(&content)
        .expect("Genesis must be valid JSON");
    
    // Verify network_id is testnet
    assert_eq!(
        genesis["network_id"].as_str().expect("network_id must be a string"),
        "testnet",
        "network_id must be 'testnet'"
    );
}

#[test]
fn test_genesis_consensus_params() {
    let genesis_path = "genesis/mainnet.json";
    let content = fs::read_to_string(genesis_path)
        .expect("Failed to read mainnet genesis");
    
    let genesis: Value = serde_json::from_str(&content)
        .expect("Genesis must be valid JSON");
    
    let params = &genesis["consensus_params"];
    
    // Verify critical consensus parameters
    assert!(params.get("target_block_time").is_some(), "target_block_time required");
    assert!(params.get("difficulty_adjustment_interval").is_some(), "difficulty_adjustment_interval required");
    assert!(params.get("max_block_size").is_some(), "max_block_size required");
    assert!(params.get("pow_algo").is_some(), "pow_algo required");
    
    // Verify pow_algo is sha256d for mainnet
    assert_eq!(
        params["pow_algo"].as_str().expect("pow_algo must be a string"),
        "sha256d",
        "Mainnet must use sha256d"
    );
}
