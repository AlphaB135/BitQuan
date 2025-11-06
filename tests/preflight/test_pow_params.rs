use std::fs;
use serde_json::Value;

#[test]
fn test_pow_param_matrix_mainnet() {
    let genesis_path = "genesis/mainnet.json";
    let content = fs::read_to_string(genesis_path)
        .expect("Failed to read mainnet genesis");
    
    let genesis: Value = serde_json::from_str(&content)
        .expect("Genesis must be valid JSON");
    
    let pow_algo = genesis["consensus_params"]["pow_algo"]
        .as_str()
        .expect("pow_algo must be present");
    
    // Mainnet must be locked to sha256d
    assert_eq!(
        pow_algo,
        "sha256d",
        "Mainnet must use SHA-256d only, hybrid forbidden"
    );
}

#[test]
fn test_pow_param_matrix_testnet() {
    let genesis_path = "genesis/testnet.json";
    let content = fs::read_to_string(genesis_path)
        .expect("Failed to read testnet genesis");
    
    let genesis: Value = serde_json::from_str(&content)
        .expect("Genesis must be valid JSON");
    
    let pow_algo = genesis["consensus_params"]["pow_algo"]
        .as_str()
        .expect("pow_algo must be present");
    
    // Testnet can use sha256d or hybrid
    assert!(
        pow_algo == "sha256d" || pow_algo == "hybrid",
        "Testnet must use sha256d or hybrid, got: {}",
        pow_algo
    );
}

#[test]
fn test_pow_difficulty_params() {
    let genesis_path = "genesis/mainnet.json";
    let content = fs::read_to_string(genesis_path)
        .expect("Failed to read mainnet genesis");
    
    let genesis: Value = serde_json::from_str(&content)
        .expect("Genesis must be valid JSON");
    
    let params = &genesis["consensus_params"];
    
    // Verify difficulty adjustment parameters
    assert!(
        params.get("difficulty_adjustment_interval").is_some(),
        "difficulty_adjustment_interval required"
    );
    
    assert!(
        params.get("min_difficulty_bits").is_some(),
        "min_difficulty_bits required"
    );
    
    assert!(
        params.get("max_difficulty_bits").is_some(),
        "max_difficulty_bits required"
    );
    
    let min_bits = params["min_difficulty_bits"].as_u64().unwrap();
    let max_bits = params["max_difficulty_bits"].as_u64().unwrap();
    
    assert!(
        max_bits >= min_bits,
        "max_difficulty_bits must be >= min_difficulty_bits"
    );
}

#[test]
fn test_pow_target_block_time() {
    let genesis_path = "genesis/mainnet.json";
    let content = fs::read_to_string(genesis_path)
        .expect("Failed to read mainnet genesis");
    
    let genesis: Value = serde_json::from_str(&content)
        .expect("Genesis must be valid JSON");
    
    let target_block_time = genesis["consensus_params"]["target_block_time"]
        .as_u64()
        .expect("target_block_time must be u64");
    
    // Typical Bitcoin-like target is 600 seconds (10 minutes)
    assert!(
        target_block_time > 0,
        "target_block_time must be positive"
    );
    
    assert!(
        target_block_time <= 3600,
        "target_block_time should be reasonable (≤ 1 hour)"
    );
}

#[test]
fn test_pow_subsidy_params() {
    let genesis_path = "genesis/mainnet.json";
    let content = fs::read_to_string(genesis_path)
        .expect("Failed to read mainnet genesis");
    
    let genesis: Value = serde_json::from_str(&content)
        .expect("Genesis must be valid JSON");
    
    let params = &genesis["consensus_params"];
    
    assert!(
        params.get("initial_subsidy").is_some(),
        "initial_subsidy required"
    );
    
    assert!(
        params.get("subsidy_halving_interval").is_some(),
        "subsidy_halving_interval required"
    );
    
    let initial_subsidy = params["initial_subsidy"].as_u64().unwrap();
    let halving_interval = params["subsidy_halving_interval"].as_u64().unwrap();
    
    assert!(initial_subsidy > 0, "initial_subsidy must be positive");
    assert!(halving_interval > 0, "subsidy_halving_interval must be positive");
}
