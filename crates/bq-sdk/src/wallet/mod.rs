//! Wallet implementation with post-quantum security

use crate::{
    address::{Address, Network},
    crypto::DilithiumKeyPair,
    psbt::PQPSBT,
    Result, SDKError,
};
use serde::{Deserialize, Serialize};
use sha2::Digest;
use std::collections::HashMap;
use thiserror::Error;
use zeroize::Zeroize;

/// Wallet errors
#[derive(Debug, Error)]
pub enum WalletError {
    /// Invalid mnemonic
    #[error("Invalid mnemonic: {0}")]
    InvalidMnemonic(String),

    /// Invalid derivation path
    #[error("Invalid derivation path: {0}")]
    InvalidDerivationPath(String),

    /// Key generation failed
    #[error("Key generation failed: {0}")]
    KeyGenerationFailed(String),

    /// Signing failed
    #[error("Signing failed: {0}")]
    SigningFailed(String),

    /// Address generation failed
    #[error("Address generation failed: {0}")]
    AddressGenerationFailed(String),

    /// Wallet locked
    #[error("Wallet is locked")]
    WalletLocked,

    /// Invalid password
    #[error("Invalid password")]
    InvalidPassword,
}

/// Signature algorithms supported by the wallet
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum SignatureAlgorithm {
    /// ECDSA (secp256k1)
    ECDSA,
    /// Dilithium5 (post-quantum)
    #[default]
    Dilithium5,
    /// Hybrid (both ECDSA and Dilithium)
    Hybrid,
}

impl SignatureAlgorithm {
    /// Check if the algorithm is post-quantum secure
    pub fn is_post_quantum(&self) -> bool {
        matches!(
            self,
            SignatureAlgorithm::Dilithium5 | SignatureAlgorithm::Hybrid
        )
    }
}

/// Derivation path for HD wallets
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DerivationPath {
    /// Path components
    pub path: Vec<u32>,
    /// Whether components are hardened
    pub hardened: Vec<bool>,
}

impl DerivationPath {
    /// Create new derivation path
    pub fn new() -> Self {
        Self {
            path: vec![],
            hardened: vec![],
        }
    }

    /// Add component to path
    pub fn push(mut self, index: u32, hardened: bool) -> Self {
        self.path.push(index);
        self.hardened.push(hardened);
        self
    }

    /// Get BIP32 standard path for account
    pub fn bip44_standard(account: u32, change: u32, address_index: u32) -> Self {
        Self::new()
            .push(44, true) // purpose
            .push(0, true) // coin_type (Bitcoin)
            .push(account, true) // account
            .push(change, false) // change
            .push(address_index, false) // address_index
    }

    /// Get BIP84 standard path (native SegWit)
    pub fn bip84_standard(account: u32, change: u32, address_index: u32) -> Self {
        Self::new()
            .push(84, true) // purpose
            .push(0, true) // coin_type (Bitcoin)
            .push(account, true) // account
            .push(change, false) // change
            .push(address_index, false) // address_index
    }

    /// Get BitQuan post-quantum path
    pub fn bq_standard(account: u32, change: u32, address_index: u32) -> Self {
        Self::new()
            .push(123, true) // BitQuan purpose
            .push(0, true) // coin_type
            .push(account, true) // account
            .push(change, false) // change
            .push(address_index, false) // address_index
    }

    /// Convert to string representation
    pub fn as_string(&self) -> String {
        if self.path.is_empty() {
            return "m".to_string();
        }

        let mut result = "m".to_string();
        for (index, hardened) in self.path.iter().zip(self.hardened.iter()) {
            result.push('/');
            result.push_str(&index.to_string());
            if *hardened {
                result.push('\'');
            }
        }
        result
    }

    /// Parse from string representation
    pub fn parse(path: &str) -> Result<Self> {
        if !path.starts_with('m') {
            return Err(SDKError::Wallet(WalletError::InvalidDerivationPath(
                "Path must start with 'm'".to_string(),
            )));
        }

        let parts: Vec<&str> = path.split('/').collect();
        let mut derivation = Self::new();

        for part in parts.iter().skip(1) {
            if part.is_empty() {
                continue;
            }

            let hardened = part.ends_with('\'');
            let index_str = if hardened {
                &part[..part.len() - 1]
            } else {
                part
            };

            let index = index_str.parse::<u32>().map_err(|_| {
                WalletError::InvalidDerivationPath(format!("Invalid index: {}", index_str))
            })?;

            derivation = derivation.push(index, hardened);
        }

        Ok(derivation)
    }
}

impl Default for DerivationPath {
    fn default() -> Self {
        Self::bq_standard(0, 0, 0)
    }
}

impl std::fmt::Display for DerivationPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_string())
    }
}

/// Mnemonic phrase with quantum-resistant enhancements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mnemonic {
    /// The mnemonic words
    pub words: Vec<String>,
    /// Language of the wordlist
    pub language: String,
    /// Entropy bits
    pub entropy_bits: usize,
    /// Has quantum enhancement
    pub quantum_enhanced: bool,
}

impl Mnemonic {
    /// Generate new mnemonic with quantum enhancement
    pub fn generate(entropy_bits: usize, quantum_enhanced: bool) -> Result<Self> {
        if !entropy_bits.is_multiple_of(32) {
            return Err(SDKError::Wallet(WalletError::InvalidMnemonic(
                "Entropy bits must be multiple of 32".to_string(),
            )));
        }

        use bip39::{Mnemonic as Bip39Mnemonic, Language};
        
        let entropy_bytes = entropy_bits / 8;
        let mut entropy = vec![0u8; entropy_bytes];

        // Generate entropy
        getrandom::getrandom(&mut entropy)
            .map_err(|e| WalletError::KeyGenerationFailed(e.to_string()))?;

        // Add quantum enhancement if requested
        if quantum_enhanced {
            let mut quantum_entropy = vec![0u8; 16]; // 128 bits
            getrandom::getrandom(&mut quantum_entropy)
                .map_err(|e| WalletError::KeyGenerationFailed(e.to_string()))?;

            // Mix quantum entropy
            for i in 0..entropy.len().min(quantum_entropy.len()) {
                entropy[i] ^= quantum_entropy[i];
            }
        }

        let bip39_mnemonic = Bip39Mnemonic::from_entropy(&entropy, Language::English)
            .map_err(|_| WalletError::KeyGenerationFailed("Invalid entropy".to_string()))?;

        let words: Vec<String> = bip39_mnemonic
            .word_iter()
            .map(|w| w.to_string())
            .collect();

        Ok(Self {
            words,
            language: "en".to_string(),
            entropy_bits,
            quantum_enhanced,
        })
    }

    /// Parse mnemonic from string
    pub fn from_str(mnemonic: &str, quantum_enhanced: bool) -> Result<Self> {
        use bip39::{Mnemonic as Bip39Mnemonic, Language};
        
        // Validate with bip39
        let bip39_mnemonic = Bip39Mnemonic::from_phrase(mnemonic, Language::English)
            .map_err(|_| WalletError::InvalidMnemonic("Invalid BIP-39 mnemonic".to_string()))?;

        let words: Vec<String> = bip39_mnemonic
            .word_iter()
            .map(|w| w.to_string())
            .collect();
            
        let entropy_bits = bip39_mnemonic.entropy().len() * 8;

        Ok(Self {
            words,
            language: "en".to_string(),
            entropy_bits,
            quantum_enhanced,
        })
    }

    /// Convert to string
    pub fn as_string(&self) -> String {
        self.words.join(" ")
    }

    /// Generate seed from mnemonic
    pub fn to_seed(&self, passphrase: &str) -> Result<[u8; 64]> {
        use pbkdf2::pbkdf2;
        use hmac::Hmac;
        use sha2::Sha512;

        let mnemonic_str = self.as_string();
        let salt = format!("mnemonic{}", passphrase);

        let mut seed = [0u8; 64];
        pbkdf2::<Hmac<Sha512>>(
            mnemonic_str.as_bytes(),
            salt.as_bytes(),
            2048,
            &mut seed,
        );

        Ok(seed)
    }
}

impl std::fmt::Display for Mnemonic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_string())
    }
}

/// Wallet configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletConfig {
    /// Network
    pub network: Network,
    /// Signature algorithms to support
    pub signature_algorithms: Vec<SignatureAlgorithm>,
    /// Key derivation settings
    pub derivation: DerivationConfig,
    /// Security settings
    pub security: SecurityConfig,
    /// Performance settings
    pub performance: PerformanceConfig,
}

/// Derivation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DerivationConfig {
    /// Use BIP32 standard paths
    pub bip32_standard: bool,
    /// Custom derivation path
    pub custom_path: Option<DerivationPath>,
    /// Account gap limit
    pub gap_limit: u32,
}

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Require both PQC and ECDSA signatures
    pub hybrid_signatures: bool,
    /// Memory locking for private keys
    pub memory_locking: bool,
    /// Cache timeout for decrypted keys
    pub cache_timeout: Option<std::time::Duration>,
    /// Quantum entropy source
    pub quantum_entropy: bool,
}

/// Performance configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// Enable key caching
    pub enable_cache: bool,
    /// Maximum cache entries
    pub max_cache_entries: usize,
    /// Pre-generate addresses
    pub pregenerate_addresses: u32,
}

impl WalletConfig {
    /// Create new configuration
    pub fn new(network: Network) -> Self {
        Self {
            network,
            signature_algorithms: vec![SignatureAlgorithm::Dilithium5],
            derivation: DerivationConfig::default(),
            security: SecurityConfig::default(),
            performance: PerformanceConfig::default(),
        }
    }

    /// Server configuration (high security)
    pub fn server() -> Self {
        Self {
            network: Network::Mainnet,
            signature_algorithms: vec![SignatureAlgorithm::Dilithium5],
            derivation: DerivationConfig::default(),
            security: SecurityConfig {
                hybrid_signatures: false,
                memory_locking: true,
                cache_timeout: None,
                quantum_entropy: true,
            },
            performance: PerformanceConfig {
                enable_cache: false,
                max_cache_entries: 0,
                pregenerate_addresses: 0,
            },
        }
    }

    /// Mobile configuration (balanced)
    pub fn mobile() -> Self {
        Self {
            network: Network::Mainnet,
            signature_algorithms: vec![SignatureAlgorithm::Dilithium5],
            derivation: DerivationConfig::default(),
            security: SecurityConfig {
                hybrid_signatures: false,
                memory_locking: true,
                cache_timeout: Some(std::time::Duration::from_secs(300)),
                quantum_entropy: true,
            },
            performance: PerformanceConfig {
                enable_cache: true,
                max_cache_entries: 100,
                pregenerate_addresses: 10,
            },
        }
    }

    /// Desktop configuration (performance)
    pub fn desktop() -> Self {
        Self {
            network: Network::Mainnet,
            signature_algorithms: vec![SignatureAlgorithm::Dilithium5],
            derivation: DerivationConfig::default(),
            security: SecurityConfig {
                hybrid_signatures: false,
                memory_locking: true,
                cache_timeout: Some(std::time::Duration::from_secs(600)),
                quantum_entropy: true,
            },
            performance: PerformanceConfig {
                enable_cache: true,
                max_cache_entries: 1000,
                pregenerate_addresses: 50,
            },
        }
    }
}

impl Default for DerivationConfig {
    fn default() -> Self {
        Self {
            bip32_standard: true,
            custom_path: None,
            gap_limit: 20,
        }
    }
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            hybrid_signatures: false,
            memory_locking: true,
            cache_timeout: Some(std::time::Duration::from_secs(300)),
            quantum_entropy: true,
        }
    }
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            enable_cache: true,
            max_cache_entries: 100,
            pregenerate_addresses: 10,
        }
    }
}

/// Main wallet trait
pub trait Wallet {
    /// The error type for wallet operations
    type Error: std::error::Error;

    /// Generate new wallet
    fn generate(config: &WalletConfig) -> Result<Self>
    where
        Self: Sized;

    /// Restore from mnemonic
    fn from_mnemonic(mnemonic: &Mnemonic, config: &WalletConfig) -> Result<Self>
    where
        Self: Sized;

    /// Get address at derivation path
    fn get_address(&self, path: &DerivationPath) -> Result<Address>;

    /// Get public key for address
    fn get_public_key(&self, path: &DerivationPath) -> Result<Vec<u8>>;

    /// Sign PQ-PSBT
    fn sign_psbt(&mut self, psbt: &mut PQPSBT) -> Result<()>;

    /// Get mnemonic (if available)
    fn get_mnemonic(&self) -> Option<&Mnemonic>;

    /// Lock wallet (clear sensitive data)
    fn lock(&mut self);

    /// Check if wallet is locked
    fn is_locked(&self) -> bool;

    /// Get wallet configuration
    fn config(&self) -> &WalletConfig;
}

/// Simple wallet implementation
#[derive(Debug)]
pub struct SimpleWallet {
    config: WalletConfig,
    mnemonic: Option<Mnemonic>,
    seed: Option<[u8; 64]>,
    addresses: HashMap<DerivationPath, Address>,
    locked: bool,
}

impl SimpleWallet {
    /// Create new wallet
    pub fn new(config: WalletConfig) -> Result<Self> {
        let mnemonic = Mnemonic::generate(256, config.security.quantum_entropy)?;
        let seed = mnemonic.to_seed("")?;

        Ok(Self {
            config,
            mnemonic: Some(mnemonic),
            seed: Some(seed),
            addresses: HashMap::new(),
            locked: false,
        })
    }

    /// Restore from mnemonic
    pub fn from_mnemonic(mnemonic: Mnemonic, config: WalletConfig) -> Result<Self> {
        let seed = mnemonic.to_seed("")?;

        Ok(Self {
            config,
            mnemonic: Some(mnemonic),
            seed: Some(seed),
            addresses: HashMap::new(),
            locked: false,
        })
    }

    /// Derive key at path
    fn derive_key(&self, path: &DerivationPath) -> Result<DilithiumKeyPair> {
        if self.locked {
            return Err(SDKError::Wallet(WalletError::WalletLocked));
        }

        let seed = self
            .seed
            .ok_or(SDKError::Wallet(WalletError::WalletLocked))?;

        // Simplified key derivation (in production, use proper BIP32)
        let mut hasher = sha2::Sha256::new();
        hasher.update(seed);
        hasher.update(path.as_string().as_bytes());
        let hash = hasher.finalize();

        // Generate Dilithium keypair from hash
        let keypair = DilithiumKeyPair::from_seed(&hash)?;

        Ok(keypair)
    }
}

impl Wallet for SimpleWallet {
    type Error = WalletError;

    fn generate(config: &WalletConfig) -> Result<Self> {
        Self::new(config.clone())
    }

    fn from_mnemonic(mnemonic: &Mnemonic, config: &WalletConfig) -> Result<Self> {
        Self::from_mnemonic(mnemonic.clone(), config.clone())
    }

    fn get_address(&self, path: &DerivationPath) -> Result<Address> {
        // Check cache first
        if let Some(address) = self.addresses.get(path) {
            return Ok(address.clone());
        }

        // Derive key and generate address
        let keypair = self.derive_key(path)?;
        let address = Address::pq_p2pkh(self.config.network, &keypair.public_key)?;

        Ok(address)
    }

    fn get_public_key(&self, path: &DerivationPath) -> Result<Vec<u8>> {
        let keypair = self.derive_key(path)?;
        Ok(keypair.public_key.to_vec())
    }

    fn sign_psbt(&mut self, psbt: &mut PQPSBT) -> Result<()> {
        if self.locked {
            return Err(SDKError::Wallet(WalletError::WalletLocked));
        }

        // Compute sighash: SHA-256 over all inputs (prev_txid, prev_vout) and
        // all outputs (amount, script_pubkey) to commit to the transaction data.
        let sighash = {
            let mut hasher = sha2::Sha256::new();

            // Commit to all inputs
            for inp in psbt.inputs.iter() {
                if let Some(txid_bytes) = inp.get_field(
                    &crate::psbt::InputKey::PreviousTxid([0u8; 32]),
                ) {
                    hasher.update(txid_bytes);
                }
                if let Some(vout_bytes) = inp.get_field(
                    &crate::psbt::InputKey::PreviousOutputIndex(0),
                ) {
                    hasher.update(vout_bytes);
                }
            }

            // Commit to all outputs
            for out in psbt.outputs.iter() {
                if let Some(amount) = out.get_amount() {
                    hasher.update(&amount.to_le_bytes());
                }
                if let Some(script) = out.get_script_pubkey() {
                    hasher.update(&script);
                }
            }

            let hash = hasher.finalize();
            let mut sighash = [0u8; 32];
            sighash.copy_from_slice(&hash);
            sighash
        };

        // Sign each input that needs signing
        for (i, input) in psbt.inputs.iter_mut().enumerate() {
            if input.get_dilithium_signature().is_none() {
                // Derive key for this input (simplified)
                let path = DerivationPath::bq_standard(0, 0, i as u32);
                let keypair = self.derive_key(&path)?;

                let signature = keypair.sign(&sighash)?;

                input.set_dilithium_signature(signature);
                input.set_dilithium_public_key(keypair.public_key);
            }
        }

        Ok(())
    }

    fn get_mnemonic(&self) -> Option<&Mnemonic> {
        self.mnemonic.as_ref()
    }

    fn lock(&mut self) {
        self.locked = true;
        if let Some(mut seed) = self.seed.take() {
            seed.zeroize();
        }
        self.addresses.clear();
    }

    fn is_locked(&self) -> bool {
        self.locked
    }

    fn config(&self) -> &WalletConfig {
        &self.config
    }
}

impl Drop for SimpleWallet {
    fn drop(&mut self) {
        self.lock();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mnemonic_generation() {
        let mnemonic = Mnemonic::generate(256, true).unwrap();
        assert_eq!(mnemonic.words.len(), 24); // 256 bits = 24 words
        assert!(mnemonic.quantum_enhanced);
    }

    #[test]
    fn test_derivation_path() {
        let path = DerivationPath::bq_standard(0, 1, 2);
        assert_eq!(path.as_string(), "m/123'/0'/0'/1/2");

        let parsed = DerivationPath::parse(&path.as_string()).unwrap();
        assert_eq!(path, parsed);
    }

    #[test]
    fn test_wallet_generation() {
        let config = WalletConfig::desktop();
        let wallet = SimpleWallet::generate(&config).unwrap();

        assert!(!wallet.is_locked());
        assert!(wallet.get_mnemonic().is_some());

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
}
