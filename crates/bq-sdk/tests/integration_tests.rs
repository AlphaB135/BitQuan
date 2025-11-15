//! Integration tests for BitQuan SDK

use bq_sdk::{
    Address, AddressType, DerivationPath, Mnemonic, PQPSBT, SignatureAlgorithm,
    SimpleWallet, Wallet, WalletConfig, Network,
};
use std::collections::HashMap;

#[test]
fn test_address_generation() {
    let pubkey_hash = [0x12; 20];
    let address = Address::p2pkh(Network::Mainnet, &pubkey_hash).unwrap();
    
    assert_eq!(address.network, Network::Mainnet);
    assert_eq!(address.address_type, AddressType::P2PKH);
    assert_eq!(address.data, pubkey_hash);
    assert!(address.address.starts_with("bq1"));
}

#[test]
fn test_post_quantum_address() {
    let pubkey = [0x42; 1952];
    let address = Address::pq_p2pkh(Network::Mainnet, &pubkey).unwrap();
    
    assert_eq!(address.network, Network::Mainnet);
    assert_eq!(address.address_type, AddressType::PQPP2PKH);
    assert_eq!(address.data.len(), 20);
    assert!(address.is_post_quantum());
}

#[test]
fn test_address_validation() {
    let pubkey_hash = [0x34; 20];
    let address = Address::p2pkh(Network::Mainnet, &pubkey_hash).unwrap();
    
    // Valid for mainnet
    assert_eq!(
        Address::validate_for_network(&address.to_string(), Network::Mainnet),
        bq_sdk::address::ValidationResult::Valid
    );
    
    // Invalid for testnet
    assert_eq!(
        Address::validate_for_network(&address.to_string(), Network::Testnet),
        bq_sdk::address::ValidationResult::WrongNetwork
    );
}

#[test]
fn test_address_roundtrip() {
    let pubkey_hash = [0x56; 20];
    let original = Address::p2pkh(Network::Testnet, &pubkey_hash).unwrap();
    let parsed = Address::from_str(&original.to_string()).unwrap();
    
    assert_eq!(original, parsed);
}

#[test]
fn test_mnemonic_generation() {
    let mnemonic = Mnemonic::generate(256, true).unwrap();
    
    assert_eq!(mnemonic.words.len(), 24); // 256 bits = 24 words
    assert!(mnemonic.quantum_enhanced);
    assert_eq!(mnemonic.entropy_bits, 256);
}

#[test]
fn test_mnemonic_parsing() {
    let words = vec!["word1", "word2", "word3"];
    let mnemonic_str = words.join(" ");
    let mnemonic = Mnemonic::from_str(&mnemonic_str, false).unwrap();
    
    assert_eq!(mnemonic.words, words);
    assert!(!mnemonic.quantum_enhanced);
}

#[test]
fn test_derivation_path() {
    let path = DerivationPath::bq_standard(0, 1, 2);
    assert_eq!(path.to_string(), "m/123'/0'/0'/1/2");
    
    let parsed = DerivationPath::from_str(&path.to_string()).unwrap();
    assert_eq!(path, parsed);
}

#[test]
fn test_wallet_generation() {
    let config = WalletConfig::desktop();
    let wallet = SimpleWallet::generate(&config).unwrap();
    
    assert!(!wallet.is_locked());
    assert!(wallet.get_mnemonic().is_some());
    assert_eq!(wallet.config().network, Network::Mainnet);
}

#[test]
fn test_wallet_from_mnemonic() {
    let mnemonic = Mnemonic::generate(256, true).unwrap();
    let config = WalletConfig::mobile();
    let wallet = SimpleWallet::from_mnemonic(&mnemonic, &config).unwrap();
    
    assert!(!wallet.is_locked());
    assert!(wallet.get_mnemonic().is_some());
}

#[test]
fn test_wallet_address_generation() {
    let config = WalletConfig::desktop();
    let wallet = SimpleWallet::generate(&config).unwrap();
    
    let path = DerivationPath::default();
    let address = wallet.get_address(&path).unwrap();
    
    assert_eq!(address.network, Network::Mainnet);
    assert!(address.is_post_quantum());
}

#[test]
fn test_wallet_locking() {
    let config = WalletConfig::desktop();
    let mut wallet = SimpleWallet::generate(&config).unwrap();
    
    assert!(!wallet.is_locked());
    wallet.lock();
    assert!(wallet.is_locked());
}

#[test]
fn test_psbt_builder() {
    let psbt = PQPSBT::builder()
        .version(1)
        .locktime(0)
        .add_input("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef", 0)
        .unwrap()
        .add_output("bq1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4", 1000000)
        .unwrap()
        .build()
        .unwrap();
    
    assert_eq!(psbt.version, 0);
    assert_eq!(psbt.inputs.len(), 1);
    assert_eq!(psbt.outputs.len(), 1);
}

#[test]
fn test_psbt_serialization() {
    let psbt = PQPSBT::builder()
        .version(1)
        .add_input("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef", 0)
        .unwrap()
        .add_output("bq1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4", 1000000)
        .unwrap()
        .build()
        .unwrap();
    
    let serialized = psbt.serialize().unwrap();
    let deserialized = PQPSBT::deserialize(&serialized).unwrap();
    
    assert_eq!(psbt.version, deserialized.version);
    assert_eq!(psbt.inputs.len(), deserialized.inputs.len());
    assert_eq!(psbt.outputs.len(), deserialized.outputs.len());
}

#[test]
fn test_wallet_psbt_signing() {
    let config = WalletConfig::desktop();
    let mut wallet = SimpleWallet::generate(&config).unwrap();
    
    let mut psbt = PQPSBT::builder()
        .version(1)
        .add_input("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef", 0)
        .unwrap()
        .add_output("bq1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4", 1000000)
        .unwrap()
        .build()
        .unwrap();
    
    // Sign PSBT
    wallet.sign_psbt(&mut psbt).unwrap();
    
    // Check that input has signature
    assert!(psbt.inputs[0].get_dilithium_signature().is_some());
    assert!(psbt.inputs[0].get_dilithium_public_key().is_some());
}

#[test]
fn test_signature_algorithms() {
    assert!(SignatureAlgorithm::Dilithium3.is_post_quantum());
    assert!(SignatureAlgorithm::Hybrid.is_post_quantum());
    assert!(!SignatureAlgorithm::ECDSA.is_post_quantum());
}

#[test]
fn test_wallet_configs() {
    let server_config = WalletConfig::server();
    assert!(server_config.security.memory_locking);
    assert!(!server_config.performance.enable_cache);
    
    let mobile_config = WalletConfig::mobile();
    assert!(mobile_config.security.memory_locking);
    assert!(mobile_config.performance.enable_cache);
    assert!(mobile_config.security.cache_timeout.is_some());
    
    let desktop_config = WalletConfig::desktop();
    assert!(desktop_config.security.memory_locking);
    assert!(desktop_config.performance.enable_cache);
    assert!(desktop_config.performance.max_cache_entries > mobile_config.performance.max_cache_entries);
}

#[test]
fn test_address_types() {
    assert_eq!(AddressType::P2PKH.version(), 0x00);
    assert_eq!(AddressType::PQPP2PKH.version(), 0x10);
    
    assert!(!AddressType::P2PKH.is_post_quantum());
    assert!(AddressType::PQPP2PKH.is_post_quantum());
    
    assert_eq!(AddressType::P2PKH.data_length(), 20);
    assert_eq!(AddressType::P2WSH.data_length(), 32);
}

#[test]
fn test_network_hrp() {
    assert_eq!(Network::Mainnet.hrp(), "bq");
    assert_eq!(Network::Testnet.hrp(), "tbq");
    assert_eq!(Network::Regtest.hrp(), "rbq");
    
    assert_eq!(Network::from_hrp("bq"), Some(Network::Mainnet));
    assert_eq!(Network::from_hrp("tbq"), Some(Network::Testnet));
    assert_eq!(Network::from_hrp("invalid"), None);
}

#[test]
fn test_mnemonic_to_seed() {
    let mnemonic = Mnemonic::generate(256, true).unwrap();
    let seed1 = mnemonic.to_seed("").unwrap();
    let seed2 = mnemonic.to_seed("").unwrap();
    
    assert_eq!(seed1, seed2); // Should be deterministic
    assert_ne!(seed1, [0u8; 64]); // Should not be all zeros
}

#[test]
fn test_comprehensive_wallet_flow() {
    // Generate mnemonic
    let mnemonic = Mnemonic::generate(256, true).unwrap();
    
    // Create wallet
    let config = WalletConfig::desktop();
    let mut wallet = SimpleWallet::from_mnemonic(&mnemonic, &config).unwrap();
    
    // Generate addresses
    let path1 = DerivationPath::bq_standard(0, 0, 0);
    let path2 = DerivationPath::bq_standard(0, 0, 1);
    
    let address1 = wallet.get_address(&path1).unwrap();
    let address2 = wallet.get_address(&path2).unwrap();
    
    assert_ne!(address1, address2);
    assert!(address1.is_post_quantum());
    assert!(address2.is_post_quantum());
    
    // Create and sign transaction
    let mut psbt = PQPSBT::builder()
        .version(1)
        .add_input("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef", 0)
        .unwrap()
        .add_output(&address2.to_string(), 500000)
        .unwrap()
        .build()
        .unwrap();
    
    wallet.sign_psbt(&mut psbt).unwrap();
    
    // Verify signature was added
    assert!(psbt.inputs[0].get_dilithium_signature().is_some());
    
    // Lock wallet
    wallet.lock();
    assert!(wallet.is_locked());
}

#[test]
fn test_error_handling() {
    // Test invalid address
    let result = Address::from_str("invalid_address");
    assert!(result.is_err());
    
    // Test invalid derivation path
    let result = DerivationPath::from_str("invalid/path");
    assert!(result.is_err());
    
    // Test invalid mnemonic
    let result = Mnemonic::from_str("", false);
    assert!(result.is_err());
}

#[test]
fn test_serialization_roundtrip() {
    let config = WalletConfig::mobile();
    let serialized = serde_json::to_string(&config).unwrap();
    let deserialized: WalletConfig = serde_json::from_str(&serialized).unwrap();
    
    assert_eq!(config.network, deserialized.network);
    assert_eq!(config.signature_algorithms, deserialized.signature_algorithms);
    assert_eq!(config.security.quantum_entropy, deserialized.security.quantum_entropy);
}