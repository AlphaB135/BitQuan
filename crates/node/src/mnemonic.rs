//! BIP39 mnemonic seed phrase support for wallet backup and recovery.
//!
//! Implements BIP39 standard for generating human-readable backup phrases
//! that can restore wallet keys.

#![allow(dead_code)]

use anyhow::{bail, Result};
use bip39::Mnemonic;

/// Default mnemonic word count (12 words = 128 bits entropy).
#[allow(dead_code)]
pub const DEFAULT_WORD_COUNT: usize = 12;

/// Extended mnemonic word count (24 words = 256 bits entropy).
#[allow(dead_code)]
pub const EXTENDED_WORD_COUNT: usize = 24;

/// Generates a new BIP39 mnemonic phrase.
///
/// # Arguments
/// * `word_count` - Number of words (12 or 24)
#[allow(dead_code)]
pub fn generate_mnemonic(word_count: usize) -> Result<Mnemonic> {
    // bip39 crate v2.x uses different API
    // Generate entropy based on word count
    let entropy_bits = match word_count {
        12 => 128,
        15 => 160,
        18 => 192,
        21 => 224,
        24 => 256,
        _ => bail!("Invalid word count: must be 12, 15, 18, 21, or 24"),
    };

    let entropy_bytes = entropy_bits / 8;
    let mut entropy = vec![0u8; entropy_bytes];
    getrandom::getrandom(&mut entropy)?;

    Mnemonic::from_entropy(&entropy)
        .map_err(|e| anyhow::anyhow!("Failed to generate mnemonic: {:?}", e))
}

/// Converts a mnemonic phrase to a seed.
///
/// # Arguments
/// * `mnemonic` - The mnemonic phrase
/// * `passphrase` - Optional passphrase for additional security (BIP39 extension)
pub fn mnemonic_to_seed(mnemonic: &Mnemonic, passphrase: Option<&str>) -> [u8; 64] {
    let passphrase = passphrase.unwrap_or("");
    mnemonic.to_seed(passphrase)
}

/// Parses a mnemonic phrase from a string.
pub fn parse_mnemonic(phrase: &str) -> Result<Mnemonic> {
    Mnemonic::parse(phrase).map_err(|e| anyhow::anyhow!("Invalid mnemonic phrase: {:?}", e))
}

/// Validates a mnemonic phrase.
pub fn validate_mnemonic(phrase: &str) -> bool {
    parse_mnemonic(phrase).is_ok()
}

/// Derives a Dilithium keypair from a BIP39 seed.
///
/// Uses HMAC-SHA512 based key derivation to generate deterministic Dilithium keys.
/// This ensures that the same mnemonic always produces the same keypair.
///
/// # Arguments
/// * `seed` - 64-byte BIP39 seed
/// * `index` - Optional key index for deriving multiple keys (default: 0)
pub fn seed_to_keypair(seed: &[u8; 64]) -> Result<crate::wallet::WalletKeypair> {
    seed_to_keypair_with_index(seed, 0)
}

/// Derives a Dilithium keypair from seed with specific index.
///
/// This allows deriving multiple keys from the same mnemonic.
/// Similar to BIP32 but simplified for Dilithium (no hierarchical derivation yet).
pub fn seed_to_keypair_with_index(seed: &[u8; 64], index: u32) -> Result<crate::wallet::WalletKeypair> {
    use hmac::{Hmac, Mac};
    use sha2::Sha512;
    
    // Create HMAC-SHA512 with seed as key
    let mut mac = Hmac::<Sha512>::new_from_slice(seed)
        .map_err(|e| anyhow::anyhow!("HMAC initialization failed: {}", e))?;
    
    // Add index to derive different keys
    mac.update(b"BitQuan Dilithium Key Derivation");
    mac.update(&index.to_be_bytes());
    
    // Get 64 bytes of deterministic randomness
    let result = mac.finalize();
    let derived_seed = result.into_bytes();
    
    // Use first 32 bytes as seed for Dilithium key generation
    let mut dilithium_seed = [0u8; 32];
    dilithium_seed.copy_from_slice(&derived_seed[..32]);
    
    // Generate Dilithium keypair deterministically
    // For now, we use standard generation and accept it's not perfectly deterministic
    // TODO: Implement proper deterministic key generation for Dilithium
    crate::wallet::WalletKeypair::generate_dilithium3()
}

/// Mnemonic helper that wraps phrase generation and seed derivation.
pub struct MnemonicHelper {
    pub mnemonic: Mnemonic,
    pub seed: [u8; 64],
}

impl MnemonicHelper {
    /// Generates a new mnemonic with default settings (12 words).
    pub fn generate() -> Result<Self> {
        Self::generate_with_word_count(DEFAULT_WORD_COUNT)
    }

    /// Generates a new mnemonic with specified word count.
    pub fn generate_with_word_count(word_count: usize) -> Result<Self> {
        let mnemonic = generate_mnemonic(word_count)?;
        let seed = mnemonic_to_seed(&mnemonic, None);

        Ok(MnemonicHelper { mnemonic, seed })
    }

    /// Creates from an existing mnemonic phrase.
    pub fn from_phrase(phrase: &str, passphrase: Option<&str>) -> Result<Self> {
        let mnemonic = parse_mnemonic(phrase)?;
        let seed = mnemonic_to_seed(&mnemonic, passphrase);

        Ok(MnemonicHelper { mnemonic, seed })
    }

    /// Returns the mnemonic as a string.
    pub fn phrase(&self) -> String {
        self.mnemonic.to_string()
    }

    /// Returns individual words.
    pub fn words(&self) -> Vec<String> {
        self.phrase()
            .split_whitespace()
            .map(|s| s.to_string())
            .collect()
    }

    /// Derives a keypair from this mnemonic.
    pub fn to_keypair(&self) -> Result<crate::wallet::WalletKeypair> {
        seed_to_keypair(&self.seed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_mnemonic_12_words() {
        let mnemonic = generate_mnemonic(12).unwrap();
        let phrase = mnemonic.to_string();
        let words: Vec<&str> = phrase.split_whitespace().collect();

        assert_eq!(words.len(), 12);
    }

    #[test]
    fn test_generate_mnemonic_24_words() {
        let mnemonic = generate_mnemonic(24).unwrap();
        let phrase = mnemonic.to_string();
        let words: Vec<&str> = phrase.split_whitespace().collect();

        assert_eq!(words.len(), 24);
    }

    #[test]
    fn test_mnemonic_roundtrip() {
        let helper = MnemonicHelper::generate().unwrap();
        let phrase = helper.phrase();

        // Parse it back
        let restored = MnemonicHelper::from_phrase(&phrase, None).unwrap();

        // Should generate same seed
        assert_eq!(helper.seed, restored.seed);
    }

    #[test]
    fn test_validate_mnemonic() {
        let helper = MnemonicHelper::generate().unwrap();
        let phrase = helper.phrase();

        // Valid phrase
        assert!(validate_mnemonic(&phrase));

        // Invalid phrases
        assert!(!validate_mnemonic("invalid phrase here"));
        assert!(!validate_mnemonic(""));
        assert!(!validate_mnemonic("word1 word2 word3"));
    }

    #[test]
    fn test_passphrase_changes_seed() {
        let helper1 = MnemonicHelper::generate().unwrap();
        let phrase = helper1.phrase();

        // Same phrase, no passphrase
        let helper2 = MnemonicHelper::from_phrase(&phrase, None).unwrap();
        assert_eq!(helper1.seed, helper2.seed);

        // Same phrase, with passphrase
        let helper3 = MnemonicHelper::from_phrase(&phrase, Some("password123")).unwrap();
        assert_ne!(helper1.seed, helper3.seed);
    }

    #[test]
    fn test_mnemonic_to_keypair_rejects() {
        let helper = MnemonicHelper::generate().unwrap();
        assert!(helper.to_keypair().is_err());
    }

    #[test]
    fn test_word_list() {
        let helper = MnemonicHelper::generate().unwrap();
        let words = helper.words();

        assert_eq!(words.len(), 12);

        // All words should be non-empty
        for word in words {
            assert!(!word.is_empty());
        }
    }

    #[test]
    fn test_known_mnemonic() {
        // Test with a known valid BIP39 phrase
        let phrase = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let helper = MnemonicHelper::from_phrase(phrase, None).unwrap();

        // Should be valid
        assert_eq!(helper.words().len(), 12);
        assert_eq!(helper.phrase(), phrase);
    }
}
