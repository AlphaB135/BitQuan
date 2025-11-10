//! BIP39 mnemonic seed phrase support for wallet backup and recovery.
//!
//! Implements BIP39 standard for generating human-readable backup phrases
//! that can restore wallet keys.

#![allow(dead_code)]

use bip39::Mnemonic;
use bitquan_types::error::{Error, Result};

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
        _ => {
            return Err(Error::Invalid(
                "Invalid word count: must be 12, 15, 18, 21, or 24".to_string(),
            ))
        }
    };

    let entropy_bytes = entropy_bits / 8;
    let mut entropy = vec![0u8; entropy_bytes];
    getrandom::getrandom(&mut entropy)
        .map_err(|e| Error::Invalid(format!("Failed to generate mnemonic entropy: {e}")))?;

    Mnemonic::from_entropy(&entropy)
        .map_err(|e| Error::Invalid(format!("Failed to generate mnemonic: {:?}", e)))
}

/// Converts a mnemonic phrase to a seed.
///
/// # Arguments
/// * `mnemonic` - The mnemonic phrase
/// * `passphrase` - Optional passphrase for additional security (BIP39 extension)
pub fn mnemonic_to_seed(mnemonic: &Mnemonic, passphrase: Option<&str>) -> [u8; 64] {
    mnemonic.to_seed(passphrase.unwrap_or(""))
}

/// Parses a mnemonic phrase from a string.
pub fn parse_mnemonic(phrase: &str) -> Result<Mnemonic> {
    Mnemonic::parse(phrase).map_err(|e| Error::Invalid(format!("Invalid mnemonic phrase: {:?}", e)))
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
///
/// # Security
/// Uses HMAC-SHA512 to derive a deterministic 32-byte seed from the BIP39 seed,
/// then uses that to seed a ChaCha20 CSPRNG for Dilithium key generation.
/// This ensures:
/// - Same mnemonic + index = same keypair (deterministic)
/// - Different indices = different keypairs (key separation)
/// - Cryptographically secure (no weak randomness)
pub fn seed_to_keypair_with_index(
    seed: &[u8; 64],
    index: u32,
) -> Result<crate::wallet::WalletKeypair> {
    use hmac::{Hmac, Mac};
    use sha2::Sha512;

    // Create HMAC-SHA512 with seed as key
    let mut mac = Hmac::<Sha512>::new_from_slice(seed)
        .map_err(|e| Error::Invalid(format!("HMAC initialization failed: {e}")))?;

    // Add index to derive different keys
    mac.update(b"BitQuan Dilithium Key Derivation");
    mac.update(&index.to_be_bytes());

    // Get 64 bytes of deterministic randomness
    let result = mac.finalize();
    let derived_seed = result.into_bytes();

    // Use first 32 bytes as seed for Dilithium key generation
    let mut dilithium_seed = [0u8; 32];
    dilithium_seed.copy_from_slice(&derived_seed[..32]);

    // Generate Dilithium keypair deterministically from seed
    crate::wallet::WalletKeypair::from_seed_dilithium3(&dilithium_seed)
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
        let mnemonic = generate_mnemonic(12).expect("Failed to generate 12-word mnemonic");
        let phrase = mnemonic.to_string();
        let words: Vec<&str> = phrase.split_whitespace().collect();

        assert_eq!(words.len(), 12);
    }

    #[test]
    fn test_generate_mnemonic_24_words() {
        let mnemonic = generate_mnemonic(24).expect("Failed to generate 24-word mnemonic");
        let phrase = mnemonic.to_string();
        let words: Vec<&str> = phrase.split_whitespace().collect();

        assert_eq!(words.len(), 24);
    }

    #[test]
    fn test_mnemonic_roundtrip() {
        let helper = MnemonicHelper::generate().expect("Failed to generate mnemonic helper");
        let phrase = helper.phrase();

        // Parse it back
        let restored = MnemonicHelper::from_phrase(&phrase, None).expect("Failed to restore mnemonic from phrase");

        // Should generate same seed
        assert_eq!(helper.seed, restored.seed);
    }

    #[test]
    fn test_validate_mnemonic() {
        let helper = MnemonicHelper::generate().expect("Failed to generate mnemonic helper");
        let phrase = helper.phrase();

        // Valid phrase
        assert!(validate_mnemonic(&phrase));

        // Invalid phrases
        assert!(!validate_mnemonic("invalid phrase here"));
        assert!(!validate_mnemonic(""));
        assert!(!validate_mnemonic("word1 word2 word3"));
    }

    #[test]
    fn parse_mnemonic_invalid_returns_error() {
        assert!(parse_mnemonic("this is not a valid mnemonic").is_err());
    }

    #[test]
    fn test_passphrase_changes_seed() {
        let helper1 = MnemonicHelper::generate().expect("Failed to generate mnemonic helper");
        let phrase = helper1.phrase();

        // Same phrase, no passphrase
        let helper2 = MnemonicHelper::from_phrase(&phrase, None).expect("Failed to restore mnemonic without passphrase");
        assert_eq!(helper1.seed, helper2.seed);

        // Same phrase, with passphrase
        let helper3 = MnemonicHelper::from_phrase(&phrase, Some("password123")).expect("Failed to restore mnemonic with passphrase");
        assert_ne!(helper1.seed, helper3.seed);
    }

    #[test]
    fn test_mnemonic_to_keypair_deterministic() {
        // Generate mnemonic
        let helper = MnemonicHelper::generate().expect("Failed to generate mnemonic helper");
        let phrase = helper.phrase();

        // Derive keypair twice from same mnemonic
        let kp1 = helper.to_keypair().expect("Failed to derive keypair from mnemonic");

        // Recover from same mnemonic
        let helper2 = MnemonicHelper::from_phrase(&phrase, None).expect("Failed to restore mnemonic from phrase");
        let kp2 = helper2.to_keypair().expect("Failed to derive keypair from restored mnemonic");

        // Should produce identical keypairs
        assert_eq!(kp1.public_key, kp2.public_key);
        assert_eq!(kp1.secret_key, kp2.secret_key);
    }

    #[test]
    fn test_word_list() {
        let helper = MnemonicHelper::generate().expect("Failed to generate mnemonic helper");
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
        let helper = MnemonicHelper::from_phrase(phrase, None).expect("Failed to parse known mnemonic phrase");

        // Should be valid
        assert_eq!(helper.words().len(), 12);
        assert_eq!(helper.phrase(), phrase);
    }

    #[test]
    fn test_different_indices_produce_different_keys() {
        let helper = MnemonicHelper::generate().expect("Failed to generate mnemonic helper");

        // Derive keys at different indices
        let kp0 = seed_to_keypair_with_index(&helper.seed, 0).expect("Failed to derive keypair at index 0");
        let kp1 = seed_to_keypair_with_index(&helper.seed, 1).expect("Failed to derive keypair at index 1");
        let kp2 = seed_to_keypair_with_index(&helper.seed, 2).expect("Failed to derive keypair at index 2");

        // All keys should be different
        assert_ne!(kp0.public_key, kp1.public_key);
        assert_ne!(kp1.public_key, kp2.public_key);
        assert_ne!(kp0.public_key, kp2.public_key);

        assert_ne!(kp0.secret_key, kp1.secret_key);
        assert_ne!(kp1.secret_key, kp2.secret_key);
        assert_ne!(kp0.secret_key, kp2.secret_key);
    }

    #[test]
    fn test_same_index_produces_same_key_deterministically() {
        let helper = MnemonicHelper::generate().expect("Failed to generate mnemonic helper");

        // Derive same key index multiple times
        let kp1 = seed_to_keypair_with_index(&helper.seed, 5).expect("Failed to derive keypair first time");
        let kp2 = seed_to_keypair_with_index(&helper.seed, 5).expect("Failed to derive keypair second time");
        let kp3 = seed_to_keypair_with_index(&helper.seed, 5).expect("Failed to derive keypair third time");

        // All should be identical
        assert_eq!(kp1.public_key, kp2.public_key);
        assert_eq!(kp2.public_key, kp3.public_key);
        assert_eq!(kp1.secret_key, kp2.secret_key);
        assert_eq!(kp2.secret_key, kp3.secret_key);
    }

    #[test]
    fn test_passphrase_changes_derived_keys() {
        let phrase = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

        // Same phrase, no passphrase
        let helper1 = MnemonicHelper::from_phrase(phrase, None).expect("Failed to parse phrase without passphrase");
        let kp1 = helper1.to_keypair().expect("Failed to derive keypair without passphrase");

        // Same phrase, with passphrase
        let helper2 = MnemonicHelper::from_phrase(phrase, Some("my_secret_passphrase")).expect("Failed to parse phrase with passphrase");
        let kp2 = helper2.to_keypair().expect("Failed to derive keypair with passphrase");

        // Keys should be completely different
        assert_ne!(kp1.public_key, kp2.public_key);
        assert_ne!(kp1.secret_key, kp2.secret_key);
    }

    #[test]
    fn test_known_mnemonic_produces_consistent_key() {
        // Use a fixed known mnemonic to verify deterministic behavior
        let phrase = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

        // Derive key multiple times
        let helper1 = MnemonicHelper::from_phrase(phrase, None).expect("Failed to parse phrase first time");
        let kp1 = helper1.to_keypair().expect("Failed to derive keypair first time");

        let helper2 = MnemonicHelper::from_phrase(phrase, None).expect("Failed to parse phrase second time");
        let kp2 = helper2.to_keypair().expect("Failed to derive keypair second time");

        let helper3 = MnemonicHelper::from_phrase(phrase, None).expect("Failed to parse phrase third time");
        let kp3 = helper3.to_keypair().expect("Failed to derive keypair third time");

        // All derivations should produce identical keys
        assert_eq!(kp1.public_key, kp2.public_key);
        assert_eq!(kp2.public_key, kp3.public_key);
        assert_eq!(kp1.secret_key, kp2.secret_key);
        assert_eq!(kp2.secret_key, kp3.secret_key);
    }
}
