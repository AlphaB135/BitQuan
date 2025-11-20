//! Address management for BitQuan with Bech32m encoding
//!
//! Supports all BitQuan address types including post-quantum addresses.

use crate::{Result, SDKError};
use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;

/// Supported networks
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum Network {
    /// Main network
    #[default]
    Mainnet,
    /// Test network
    Testnet,
    /// Regression test network
    Regtest,
}

impl Network {
    /// Get the human-readable part for Bech32m encoding
    pub fn hrp(&self) -> &'static str {
        match self {
            Network::Mainnet => "bq",
            Network::Testnet => "tbq",
            Network::Regtest => "rbq",
        }
    }
    
    /// Get network from human-readable part
    pub fn from_hrp(hrp: &str) -> Option<Self> {
        match hrp {
            "bq" => Some(Network::Mainnet),
            "tbq" => Some(Network::Testnet),
            "rbq" => Some(Network::Regtest),
            _ => None,
        }
    }
}

/// Address types supported by BitQuan
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AddressType {
    /// Pay-to-Public-Key-Hash (legacy)
    P2PKH = 0x00,
    /// Pay-to-Script-Hash (legacy)
    P2SH = 0x01,
    /// Pay-to-Witness-Public-Key-Hash (native SegWit)
    P2WPKH = 0x02,
    /// Pay-to-Witness-Script-Hash (native SegWit)
    P2WSH = 0x03,
    /// Post-Quantum Pay-to-Public-Key-Hash
    PQP2PKH = 0x10,
    /// Post-Quantum Pay-to-Witness-Script-Hash
    PQP2WSH = 0x11,
}

impl AddressType {
    /// Get the version byte for this address type
    pub fn version(self) -> u8 {
        self as u8
    }
    
    /// Check if this is a post-quantum address type
    pub fn is_post_quantum(self) -> bool {
        matches!(self, AddressType::PQP2PKH | AddressType::PQP2WSH)
    }
    
    /// Get the expected data length for this address type
    pub fn data_length(self) -> usize {
        match self {
            AddressType::P2PKH | AddressType::P2SH | 
            AddressType::P2WPKH | AddressType::PQP2PKH => 20,
            AddressType::P2WSH | AddressType::PQP2WSH => 32,
        }
    }
}

/// Address validation result
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationResult {
    /// Address is valid
    Valid,
    /// Invalid format
    InvalidFormat(String),
    /// Wrong network
    WrongNetwork,
    /// Invalid version
    InvalidVersion,
    /// Invalid checksum
    InvalidChecksum,
    /// Invalid data length
    InvalidLength,
}

/// Address errors
#[derive(Debug, Error)]
pub enum AddressError {
    /// Invalid address format
    #[error("Invalid address format: {0}")]
    InvalidFormat(String),
    
    /// Invalid checksum
    #[error("Invalid checksum")]
    InvalidChecksum,
    
    /// Wrong network
    #[error("Address is for wrong network")]
    WrongNetwork,
    
    /// Invalid version
    #[error("Invalid address version: {0}")]
    InvalidVersion(u8),
    
    /// Invalid data length
    #[error("Invalid data length: {0}")]
    InvalidLength(usize),
    
    /// Bech32m encoding error
    #[error("Bech32m encoding error: {0}")]
    Bech32mError(String),
}

/// BitQuan address with Bech32m encoding
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Address {
    /// Network this address belongs to
    pub network: Network,
    /// Address type
    pub address_type: AddressType,
    /// Address data (hash or script)
    pub data: Vec<u8>,
    /// Full address string
    pub address: String,
}

impl Address {
    /// Create a new address from components
    pub fn new(network: Network, address_type: AddressType, data: Vec<u8>) -> Result<Self> {
        if data.len() != address_type.data_length() {
            return Err(SDKError::Address(AddressError::InvalidLength(data.len())));
        }
        
        let address = bech32m_encode(network.hrp(), address_type.version(), &data)?;
        
        Ok(Self {
            network,
            address_type,
            data,
            address,
        })
    }
    
    /// Create a P2PKH address from a public key hash
    pub fn p2pkh(network: Network, pubkey_hash: &[u8; 20]) -> Result<Self> {
        Self::new(network, AddressType::P2PKH, pubkey_hash.to_vec())
    }
    
    /// Create a post-quantum P2PKH address from Dilithium public key
    pub fn pq_p2pkh(network: Network, dilithium_pubkey: &[u8; 1952]) -> Result<Self> {
        // Hash the Dilithium public key
        use sha2::{Digest, Sha256};
        let hash = Sha256::digest(dilithium_pubkey);
        
        // RIPEMD-160 of SHA-256 hash
        use ripemd::Ripemd160;
        let mut hasher = Ripemd160::new();
        hasher.update(hash);
        let pubkey_hash = hasher.finalize();
        
        Self::new(network, AddressType::PQP2PKH, pubkey_hash.to_vec())
    }
    
    /// Create a P2WPKH address from a public key hash
    pub fn p2wpkh(network: Network, pubkey_hash: &[u8; 20]) -> Result<Self> {
        Self::new(network, AddressType::P2WPKH, pubkey_hash.to_vec())
    }
    
    /// Parse address from string
    pub fn parse(address: &str) -> Result<Self> {
        let (hrp, version, data) = bech32m_decode(address)?;
        
        let network = Network::from_hrp(&hrp)
            .ok_or(SDKError::Address(AddressError::WrongNetwork))?;
        
        let address_type = AddressType::iter()
            .find(|t| t.version() == version)
            .ok_or(SDKError::Address(AddressError::InvalidVersion(version)))?;
        
        Ok(Self {
            network,
            address_type,
            data,
            address: address.to_string(),
        })
    }
    
    /// Validate address for specific network
    pub fn validate_for_network(address: &str, expected_network: Network) -> ValidationResult {
        match Self::parse(address) {
            Ok(addr) => {
                if addr.network != expected_network {
                    ValidationResult::WrongNetwork
                } else {
                    ValidationResult::Valid
                }
            }
            Err(SDKError::Address(AddressError::InvalidFormat(e))) => {
                ValidationResult::InvalidFormat(e)
            }
            Err(SDKError::Address(AddressError::InvalidChecksum)) => {
                ValidationResult::InvalidChecksum
            }
            Err(SDKError::Address(AddressError::InvalidVersion(_v))) => {
                ValidationResult::InvalidVersion
            }
            Err(SDKError::Address(AddressError::InvalidLength(_))) => {
                ValidationResult::InvalidLength
            }
            Err(_) => ValidationResult::InvalidFormat("Unknown error".to_string()),
        }
    }
    
    /// Get the public key hash for P2PKH/P2WPKH addresses
    pub fn pubkey_hash(&self) -> Option<[u8; 20]> {
        if self.data.len() == 20 {
            let mut hash = [0u8; 20];
            hash.copy_from_slice(&self.data);
            Some(hash)
        } else {
            None
        }
    }
    
    /// Get the script hash for P2WSH addresses
    pub fn script_hash(&self) -> Option<[u8; 32]> {
        if self.data.len() == 32 {
            let mut hash = [0u8; 32];
            hash.copy_from_slice(&self.data);
            Some(hash)
        } else {
            None
        }
    }
    
    /// Check if this is a post-quantum address
    pub fn is_post_quantum(&self) -> bool {
        self.address_type.is_post_quantum()
    }
}

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.address)
    }
}

impl std::str::FromStr for Address {
    type Err = SDKError;
    
    fn from_str(s: &str) -> Result<Self> {
        Self::parse(s)
    }
}

// Bech32m encoding/decoding functions

/// Encode data using Bech32m
fn bech32m_encode(hrp: &str, version: u8, data: &[u8]) -> Result<String> {
    let mut converted_data = vec![version];
    converted_data.extend_from_slice(data);
    
    bech32::encode::<bech32::Bech32m>(bech32::Hrp::parse(hrp).unwrap(), &converted_data)
        .map_err(|e| SDKError::Address(AddressError::Bech32mError(e.to_string())))
}

/// Decode Bech32m encoded string
fn bech32m_decode(s: &str) -> Result<(String, u8, Vec<u8>)> {
    let (hrp, data) = bech32::decode(s)
        .map_err(|e| SDKError::Address(AddressError::Bech32mError(e.to_string())))?;
    
    if data.is_empty() {
        return Err(SDKError::Address(AddressError::InvalidFormat("No data".to_string())));
    }
    
    let version = data[0];
    let payload = data[1..].to_vec();
    
    Ok((hrp.to_string(), version, payload))
}

impl AddressType {
    /// Iterate over all address types
    pub fn iter() -> impl Iterator<Item = Self> {
        [
            Self::P2PKH,
            Self::P2SH,
            Self::P2WPKH,
            Self::P2WSH,
            Self::PQP2PKH,
            Self::PQP2WSH,
        ].iter().copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_address_p2pkh() {
        let pubkey_hash = [0x12; 20];
        let address = Address::p2pkh(Network::Mainnet, &pubkey_hash).unwrap();
        
        assert_eq!(address.network, Network::Mainnet);
        assert_eq!(address.address_type, AddressType::P2PKH);
        assert_eq!(address.data, pubkey_hash);
        assert!(address.address.starts_with("bq1"));
    }
    
    #[test]
    fn test_address_pq_p2pkh() {
        let pubkey = [0x42; 1952];
        let address = Address::pq_p2pkh(Network::Mainnet, &pubkey).unwrap();
        
        assert_eq!(address.network, Network::Mainnet);
        assert_eq!(address.address_type, AddressType::PQP2PKH);
        assert_eq!(address.data.len(), 20);
        assert!(address.is_post_quantum());
    }
    
    #[test]
    fn test_address_validation() {
        let pubkey_hash = [0x12; 20];
        let address = Address::p2pkh(Network::Mainnet, &pubkey_hash).unwrap();
        
        // Valid for mainnet
        assert_eq!(
            Address::validate_for_network(&address.to_string(), Network::Mainnet),
            ValidationResult::Valid
        );
        
        // Invalid for testnet
        assert_eq!(
            Address::validate_for_network(&address.to_string(), Network::Testnet),
            ValidationResult::WrongNetwork
        );
    }
    
    #[test]
    fn test_address_roundtrip() {
        let pubkey_hash = [0x34; 20];
        let original = Address::p2pkh(Network::Testnet, &pubkey_hash).unwrap();
        let parsed = Address::parse(&original.to_string()).unwrap();
        
        assert_eq!(original, parsed);
    }
}