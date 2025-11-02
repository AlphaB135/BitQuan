//! Secure wallet backup and restore functionality
//!
//! Provides encrypted backup with separate password, metadata, and tamper detection.

use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use argon2::{Argon2, Params};
use base64::{engine::general_purpose, Engine as _};
use rand::rngs::OsRng;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use zeroize::Zeroize;

const SALT_LEN: usize = 32;
const NONCE_LEN: usize = 12;
const MAC_LEN: usize = 32;

/// Network type for backup
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Network {
    /// Mainnet
    Mainnet,
    /// Testnet
    Testnet,
    /// Devnet
    Devnet,
}

/// KDF parameters for backup encryption
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupKdfParams {
    /// Memory cost in KiB
    pub mem_kib: u32,
    /// Time cost (iterations)
    pub time_cost: u32,
    /// Parallelism degree
    pub parallelism: u32,
}

impl Default for BackupKdfParams {
    fn default() -> Self {
        Self {
            mem_kib: 65536, // 64 MiB (stronger than keystore)
            time_cost: 3,
            parallelism: 4,
        }
    }
}

/// Encrypted wallet backup with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletBackup {
    /// Format version
    pub version: u32,
    /// Backup timestamp (Unix seconds)
    pub timestamp: u64,
    /// Network type
    pub network: Network,
    /// Optional backup label
    pub label: Option<String>,
    /// KDF parameters
    pub kdf: BackupKdfParams,
    /// KDF salt
    pub salt: String, // base64
    /// AES-GCM nonce
    pub nonce: String, // base64
    /// Encrypted wallet data
    pub ciphertext: String, // base64
    /// HMAC for tamper detection
    pub mac: String, // base64
}

impl WalletBackup {
    /// Current backup format version
    pub const CURRENT_VERSION: u32 = 1;

    /// Creates a new encrypted backup from wallet data
    ///
    /// # Arguments
    /// * `wallet_data` - Raw wallet data to backup (e.g., keystore JSON)
    /// * `password` - Backup password (separate from wallet password)
    /// * `network` - Network type
    /// * `label` - Optional label for this backup
    pub fn create(
        wallet_data: &[u8],
        password: &str,
        network: Network,
        label: Option<String>,
    ) -> Result<Self, String> {
        Self::create_with_params(
            wallet_data,
            password,
            network,
            label,
            BackupKdfParams::default(),
        )
    }

    /// Creates backup with custom KDF parameters
    pub fn create_with_params(
        wallet_data: &[u8],
        password: &str,
        network: Network,
        label: Option<String>,
        kdf: BackupKdfParams,
    ) -> Result<Self, String> {
        // Generate random salt and nonce
        let mut salt = vec![0u8; SALT_LEN];
        OsRng.fill_bytes(&mut salt);

        let mut nonce_bytes = vec![0u8; NONCE_LEN];
        OsRng.fill_bytes(&mut nonce_bytes);

        // Derive encryption key from password
        let mut key_bytes =
            derive_key(password, &salt, kdf.mem_kib, kdf.time_cost, kdf.parallelism)?;

        // Encrypt wallet data
        #[allow(deprecated)]
        let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
        let cipher = Aes256Gcm::new(key);
        #[allow(deprecated)]
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = cipher
            .encrypt(
                nonce,
                Payload {
                    msg: wallet_data,
                    aad: b"",
                },
            )
            .map_err(|e| format!("Encryption failed: {}", e))?;

        // Compute HMAC for tamper detection
        let mac = compute_hmac(&key_bytes, &salt, &nonce_bytes, &ciphertext);

        // Zeroize sensitive data
        key_bytes.zeroize();

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Ok(WalletBackup {
            version: Self::CURRENT_VERSION,
            timestamp,
            network,
            label,
            kdf,
            salt: general_purpose::STANDARD.encode(&salt),
            nonce: general_purpose::STANDARD.encode(&nonce_bytes),
            ciphertext: general_purpose::STANDARD.encode(&ciphertext),
            mac: general_purpose::STANDARD.encode(&mac),
        })
    }

    /// Restores wallet data from encrypted backup
    ///
    /// # Arguments
    /// * `password` - Backup password
    ///
    /// # Returns
    /// Decrypted wallet data
    pub fn restore(&self, password: &str) -> Result<Vec<u8>, String> {
        // Version check
        if self.version > Self::CURRENT_VERSION {
            return Err(format!(
                "Unsupported backup version: {} (current: {})",
                self.version,
                Self::CURRENT_VERSION
            ));
        }

        // Decode base64 fields
        let salt = general_purpose::STANDARD
            .decode(&self.salt)
            .map_err(|e| format!("Invalid salt: {}", e))?;

        let nonce_bytes = general_purpose::STANDARD
            .decode(&self.nonce)
            .map_err(|e| format!("Invalid nonce: {}", e))?;

        let ciphertext = general_purpose::STANDARD
            .decode(&self.ciphertext)
            .map_err(|e| format!("Invalid ciphertext: {}", e))?;

        let expected_mac = general_purpose::STANDARD
            .decode(&self.mac)
            .map_err(|e| format!("Invalid MAC: {}", e))?;

        // Derive encryption key
        let mut key_bytes = derive_key(
            password,
            &salt,
            self.kdf.mem_kib,
            self.kdf.time_cost,
            self.kdf.parallelism,
        )?;

        // Verify HMAC (tamper detection)
        let computed_mac = compute_hmac(&key_bytes, &salt, &nonce_bytes, &ciphertext);
        if computed_mac != expected_mac.as_slice() {
            key_bytes.zeroize();
            return Err("Backup integrity check failed (tampered or wrong password)".to_string());
        }

        // Decrypt
        #[allow(deprecated)]
        let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
        let cipher = Aes256Gcm::new(key);
        #[allow(deprecated)]
        let nonce = Nonce::from_slice(&nonce_bytes);

        let plaintext = cipher
            .decrypt(
                nonce,
                Payload {
                    msg: ciphertext.as_ref(),
                    aad: b"",
                },
            )
            .map_err(|_| "Decryption failed (wrong password or corrupted backup)".to_string())?;

        // Zeroize sensitive data
        key_bytes.zeroize();

        Ok(plaintext)
    }

    /// Writes backup to file (atomic write)
    pub fn save<P: AsRef<Path>>(&self, path: P) -> std::io::Result<()> {
        let json = serde_json::to_string_pretty(self)?;

        // Write atomically with temp file
        let path = path.as_ref();
        let temp_path = path.with_extension("tmp");

        {
            let mut file = OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(&temp_path)?;
            file.write_all(json.as_bytes())?;
            file.sync_all()?;
        }

        std::fs::rename(&temp_path, path)?;
        Ok(())
    }

    /// Reads backup from file
    pub fn load<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        let mut file = File::open(path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;

        serde_json::from_str(&contents)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }
}

/// Derives encryption key from password using Argon2
fn derive_key(
    password: &str,
    salt: &[u8],
    mem_kib: u32,
    time_cost: u32,
    parallelism: u32,
) -> Result<Vec<u8>, String> {
    let params = Params::new(mem_kib, time_cost, parallelism, Some(32))
        .map_err(|e| format!("Invalid KDF params: {}", e))?;

    let argon2 = Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params);

    let mut output = vec![0u8; 32];
    argon2
        .hash_password_into(password.as_bytes(), salt, &mut output)
        .map_err(|e| format!("Key derivation failed: {}", e))?;

    Ok(output)
}

/// Computes HMAC-SHA256 for tamper detection
fn compute_hmac(key: &[u8], salt: &[u8], nonce: &[u8], ciphertext: &[u8]) -> Vec<u8> {
    use sha2::{Digest, Sha256};

    let mut hasher = Sha256::new();
    hasher.update(key);
    hasher.update(salt);
    hasher.update(nonce);
    hasher.update(ciphertext);

    hasher.finalize().to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backup_restore_roundtrip() {
        let wallet_data = b"test wallet keystore data";
        let password = "backup_password_123";

        let backup = WalletBackup::create(
            wallet_data,
            password,
            Network::Testnet,
            Some("Test Backup".to_string()),
        )
        .unwrap();

        let restored = backup.restore(password).unwrap();
        assert_eq!(restored, wallet_data);
    }

    #[test]
    fn test_wrong_password_fails() {
        let wallet_data = b"test wallet data";
        let backup =
            WalletBackup::create(wallet_data, "correct_password", Network::Mainnet, None).unwrap();

        let result = backup.restore("wrong_password");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("wrong password"));
    }

    #[test]
    fn test_tamper_detection() {
        let wallet_data = b"test wallet data";
        let password = "backup_password";

        let mut backup =
            WalletBackup::create(wallet_data, password, Network::Devnet, None).unwrap();

        // Tamper with ciphertext
        let mut ciphertext_bytes = general_purpose::STANDARD
            .decode(&backup.ciphertext)
            .unwrap();
        ciphertext_bytes[0] ^= 0xFF;
        backup.ciphertext = general_purpose::STANDARD.encode(&ciphertext_bytes);

        let result = backup.restore(password);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("integrity check failed"));
    }

    #[test]
    fn test_file_save_load() {
        let wallet_data = b"test wallet keystore";
        let password = "file_backup_password";
        let temp_path = std::env::temp_dir().join("test_backup.json");

        let backup = WalletBackup::create(
            wallet_data,
            password,
            Network::Testnet,
            Some("File Test".to_string()),
        )
        .unwrap();

        backup.save(&temp_path).unwrap();
        let loaded = WalletBackup::load(&temp_path).unwrap();

        assert_eq!(backup.version, loaded.version);
        assert_eq!(backup.network, loaded.network);
        assert_eq!(backup.label, loaded.label);

        let restored = loaded.restore(password).unwrap();
        assert_eq!(restored, wallet_data);

        std::fs::remove_file(&temp_path).ok();
    }

    #[test]
    fn test_version_check() {
        let wallet_data = b"test data";
        let password = "password";

        let mut backup =
            WalletBackup::create(wallet_data, password, Network::Mainnet, None).unwrap();

        backup.version = 999; // Future version

        let result = backup.restore(password);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unsupported backup version"));
    }

    #[test]
    fn test_metadata_preserved() {
        let wallet_data = b"wallet data";
        let password = "password";
        let label = "My Important Wallet".to_string();

        let backup =
            WalletBackup::create(wallet_data, password, Network::Mainnet, Some(label.clone()))
                .unwrap();

        assert_eq!(backup.version, WalletBackup::CURRENT_VERSION);
        assert_eq!(backup.network, Network::Mainnet);
        assert_eq!(backup.label, Some(label));
        assert!(backup.timestamp > 0);
    }
}
