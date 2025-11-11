// crates/wallet/src/keystore.rs
// Requires deps: argon2, aes-gcm, rand, base64, serde, serde_json, zeroize, secrecy
use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use argon2::{Argon2, Params};
use base64::{engine::general_purpose, Engine as _};
use rand::rngs::OsRng;
use rand::RngCore;
use secrecy::{ExposeSecret, SecretVec};
use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::Path;

use std::time::{SystemTime, UNIX_EPOCH};
use zeroize::Zeroize;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct KdfParams {
    pub mem_kib: u32,
    pub time_cost: u32,
    pub parallelism: u8,
    pub salt_b64: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct KeystoreFile {
    pub magic: String,
    pub version: u8,
    pub created: u64,
    pub kdf: KdfParams,
    pub nonce_b64: String,
    pub ciphertext_b64: String,
    pub meta: Option<serde_json::Value>,
}

impl KeystoreFile {
    pub fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string_pretty(self)
    }
}

pub const MAGIC: &str = "BQK1";
pub const CURRENT_VERSION: u8 = 1;

pub const DEFAULT_MEM_KIB: u32 = 65536;
pub const DEFAULT_TIME_COST: u32 = 3;
pub const DEFAULT_PARALLELISM: u8 = 1;

/// Hardware capability detection for adaptive KDF
#[derive(Debug, Clone, Copy)]
pub enum HardwareProfile {
    HighEndDesktop,  // 16+ GB RAM, 8+ cores
    MidRangeLaptop,  // 8-16 GB RAM, 4-8 cores  
    LowEndDevice,    // 4-8 GB RAM, 2-4 cores
    MobileDevice,    // <4 GB RAM, <=2 cores
}

impl HardwareProfile {
    /// Detect hardware capabilities and return appropriate profile
    pub fn detect() -> Self {
        let cores = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1);
        
        let memory_gb = get_available_memory_gb();
        
        match (cores, memory_gb) {
            (cores, mem) if cores >= 8 && mem >= 16 => HardwareProfile::HighEndDesktop,
            (cores, mem) if cores >= 4 && mem >= 8 => HardwareProfile::MidRangeLaptop,
            (cores, mem) if cores >= 2 && mem >= 4 => HardwareProfile::LowEndDevice,
            _ => HardwareProfile::MobileDevice,
        }
    }
    
    /// Get optimal parallelism for this hardware profile
    pub fn optimal_parallelism(self) -> u8 {
        match self {
            HardwareProfile::HighEndDesktop => 8,
            HardwareProfile::MidRangeLaptop => 4,
            HardwareProfile::LowEndDevice => 2,
            HardwareProfile::MobileDevice => 2,
        }
    }
    
    /// Get optimal memory cost for this hardware profile
    pub fn optimal_memory_cost(self) -> u32 {
        match self {
            HardwareProfile::HighEndDesktop => 65536, // 64 MiB
            HardwareProfile::MidRangeLaptop => 32768, // 32 MiB
            HardwareProfile::LowEndDevice => 16384,   // 16 MiB
            HardwareProfile::MobileDevice => 8192,    // 8 MiB
        }
    }
    
    /// Get optimal time cost for this hardware profile
    pub fn optimal_time_cost(self) -> u32 {
        match self {
            HardwareProfile::HighEndDesktop => 3,
            HardwareProfile::MidRangeLaptop => 2,
            HardwareProfile::LowEndDevice => 2,
            HardwareProfile::MobileDevice => 1,
        }
    }
}

/// Get available system memory in GB (approximate)
fn get_available_memory_gb() -> u32 {
    #[cfg(unix)]
    {
        use std::fs;
        match fs::read_to_string("/proc/meminfo") {
            Ok(content) => {
                for line in content.lines() {
                    if line.starts_with("MemTotal:") {
                        if let Some(kb_str) = line.split_whitespace().nth(1) {
                            if let Ok(kb) = kb_str.parse::<u64>() {
                                return (kb / (1024 * 1024)) as u32; // Convert KB to GB
                            }
                        }
                    }
                }
                4 // Fallback for unknown systems
            }
            Err(_) => 4, // Fallback if can't read meminfo
        }
    }
    
    #[cfg(windows)]
    {
        // Windows memory detection would require additional dependencies
        // For now, assume mid-range specs
        8
    }
    
    #[cfg(not(any(unix, windows)))]
    {
        // Unknown platform - conservative estimate
        4
    }
}

/// Get optimal parallelism based on available CPU cores (legacy function)
pub fn optimal_parallelism() -> u8 {
    HardwareProfile::detect().optimal_parallelism()
}
pub const SALT_LEN: usize = 16;
pub const NONCE_LEN: usize = 12;

// Thread-local buffer pool for reusing allocations
thread_local! {
    static SALT_BUFFER: std::cell::RefCell<Vec<u8>> = std::cell::RefCell::new(vec![0u8; SALT_LEN]);
    static NONCE_BUFFER: std::cell::RefCell<Vec<u8>> = std::cell::RefCell::new(vec![0u8; NONCE_LEN]);
}

#[derive(Debug, Clone, Copy)]
pub enum KdfProfile {
    Tight,
    Medium,
    Light,
    Mobile,
}

impl KdfProfile {
    pub fn params(&self) -> (u32, u32, u8) {
        let parallelism = optimal_parallelism();
        match self {
            KdfProfile::Tight => (65536, 3, parallelism),
            KdfProfile::Medium => (32768, 3, parallelism),
            KdfProfile::Light => (16384, 3, parallelism),
            KdfProfile::Mobile => (8192, 3, parallelism.min(2)), // Mobile devices cap at 2 threads
        }
    }
}

fn derive_key(
    password: &SecretVec<u8>,
    salt: &[u8],
    mem_kib: u32,
    time_cost: u32,
    parallelism: u8,
) -> [u8; 32] {
    // SAFETY: Params::new can only fail if parameters are out of range, which never happens with our constants
    #[allow(clippy::expect_used)]
    let params = Params::new(mem_kib, time_cost, parallelism.into(), None).expect("argon params");
    let argon2 = Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params);

    let mut key = [0u8; 32];
    // SAFETY: hash_password_into can only fail if output buffer is wrong size, which is fixed at 32 bytes
    #[allow(clippy::expect_used)]
    argon2
        .hash_password_into(password.expose_secret(), salt, &mut key)
        .expect("Argon2 derive failed");
    key
}

pub fn encrypt_keystore(
    plaintext: &[u8],
    password: &str,
    meta: Option<serde_json::Value>,
    mem_kib: u32,
    time_cost: u32,
    parallelism: u8,
) -> KeystoreFile {
    let pw = SecretVec::new(password.as_bytes().to_vec());

    let salt_vec = SALT_BUFFER.with(|buf| {
        let mut salt_buf = buf.borrow_mut();
        OsRng.fill_bytes(&mut salt_buf);
        salt_buf.clone()
    });
    
    let nonce_vec = NONCE_BUFFER.with(|buf| {
        let mut nonce_buf = buf.borrow_mut();
        OsRng.fill_bytes(&mut nonce_buf);
        nonce_buf.clone()
    });

    let mut key_bytes = derive_key(&pw, &salt_vec, mem_kib, time_cost, parallelism);

    #[allow(deprecated)]
    let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);
    #[allow(deprecated)]
    let nonce = Nonce::from_slice(&nonce_vec);

    // SAFETY: AES-GCM encryption can only fail if key/nonce are wrong size, which are fixed at 32/12 bytes
    #[allow(clippy::expect_used)]
    let ciphertext = cipher
        .encrypt(
            nonce,
            Payload {
                msg: plaintext,
                aad: b"",
            },
        )
        .expect("encryption failure");

    key_bytes.zeroize();

    let created = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0); // Fallback to epoch if clock is wrong
    KeystoreFile {
        magic: MAGIC.to_string(),
        version: CURRENT_VERSION,
        created,
        kdf: KdfParams {
            mem_kib,
            time_cost,
            parallelism,
            salt_b64: general_purpose::STANDARD.encode(&salt_vec),
        },
        nonce_b64: general_purpose::STANDARD.encode(&nonce_vec),
        ciphertext_b64: general_purpose::STANDARD.encode(&ciphertext),
        meta,
    }
}

// Duplicate function removed - use encrypt_keystore instead

pub fn decrypt_keystore(ks: &KeystoreFile, password: &str) -> Result<Vec<u8>, String> {
    if ks.magic != MAGIC {
        return Err(format!(
            "invalid magic: expected {}, got {}",
            MAGIC, ks.magic
        ));
    }
    if ks.version > CURRENT_VERSION {
        return Err(format!(
            "unsupported version: {} (max {})",
            ks.version, CURRENT_VERSION
        ));
    }

    let pw = SecretVec::new(password.as_bytes().to_vec());

    let salt = general_purpose::STANDARD
        .decode(&ks.kdf.salt_b64)
        .map_err(|e| format!("bad salt b64: {}", e))?;
    let nonce_bytes = general_purpose::STANDARD
        .decode(&ks.nonce_b64)
        .map_err(|e| format!("bad nonce b64: {}", e))?;
    let ciphertext = general_purpose::STANDARD
        .decode(&ks.ciphertext_b64)
        .map_err(|e| format!("bad cipher b64: {}", e))?;

    let mut key_bytes = derive_key(
        &pw,
        &salt,
        ks.kdf.mem_kib,
        ks.kdf.time_cost,
        ks.kdf.parallelism,
    );

    #[allow(deprecated)]
    let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);
    #[allow(deprecated)]
    let nonce = Nonce::from_slice(&nonce_bytes);

    let res = cipher
        .decrypt(
            nonce,
            Payload {
                msg: ciphertext.as_ref(),
                aad: b"",
            },
        )
        .map_err(|e| format!("decrypt failed: {:?}", e));

    key_bytes.zeroize();

    res
}

pub fn rotate_keystore(
    ks: &KeystoreFile,
    old_password: &str,
    new_password: &str,
    mem_kib: u32,
    time_cost: u32,
    parallelism: u8,
) -> Result<KeystoreFile, String> {
    let plaintext = decrypt_keystore(ks, old_password)?;
    let new_ks = encrypt_keystore(
        &plaintext,
        new_password,
        ks.meta.clone(),
        mem_kib,
        time_cost,
        parallelism,
    );
    Ok(new_ks)
}

pub fn write_keystore_file_atomic<P: AsRef<Path>>(
    path: P,
    ks: &KeystoreFile,
) -> std::io::Result<()> {
    let path = path.as_ref();
    let tmp_path = path.with_extension("tmp");
    let mut f = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&tmp_path)?;
    let json = ks.to_json().map_err(std::io::Error::other)?;
    f.write_all(json.as_bytes())?;
    f.flush()?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&tmp_path, std::fs::Permissions::from_mode(0o600))?;
    }

    #[cfg(windows)]
    {
        eprintln!(
            "WARNING: Windows file permissions not enforced. Use BitLocker/EFS to encrypt folder."
        );
    }

    std::fs::rename(tmp_path, path)?;
    Ok(())
}

pub fn read_keystore_file<P: AsRef<Path>>(path: P) -> std::io::Result<KeystoreFile> {
    let f = File::open(path)?;
    let ks: KeystoreFile = serde_json::from_reader(f)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    Ok(ks)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::tempdir;

    #[test]
    fn roundtrip_encrypt_decrypt() {
        let secret = b"this-is-my-private-key-bytes";
        let ks = encrypt_keystore(
            secret,
            "correct horse battery staple",
            Some(json!({"hint": "test"})),
            DEFAULT_MEM_KIB,
            DEFAULT_TIME_COST,
            DEFAULT_PARALLELISM,
        );
        let pt = decrypt_keystore(&ks, "correct horse battery staple").expect("should decrypt");
        assert_eq!(pt, secret);
    }

    #[test]
    fn wrong_password_fails() {
        let secret = b"abcdef012345";
        let ks = encrypt_keystore(
            secret,
            "pw1",
            None,
            DEFAULT_MEM_KIB,
            DEFAULT_TIME_COST,
            DEFAULT_PARALLELISM,
        );
        let res = decrypt_keystore(&ks, "pw2");
        assert!(res.is_err());
    }

    #[test]
    fn write_and_read_atomic_file() {
        let secret = b"file-secret";
        let ks = encrypt_keystore(
            secret,
            "pw",
            None,
            DEFAULT_MEM_KIB,
            DEFAULT_TIME_COST,
            DEFAULT_PARALLELISM,
        );
        let dir = tempdir().expect("Failed to create temp directory");
        let p = dir.path().join("keystore.json");
        write_keystore_file_atomic(&p, &ks).expect("write");
        let ks2 = read_keystore_file(&p).expect("read");
        let pt = decrypt_keystore(&ks2, "pw").expect("decrypt");
        assert_eq!(pt, secret);
    }

    #[test]
    fn tamper_cipher_rejected() {
        let secret = b"abc";
        let ks = encrypt_keystore(secret, "pw", None, 8 * 1024, 1, 1);
        let mut ks_bad = ks.clone();
        let mut c = general_purpose::STANDARD
            .decode(&ks_bad.ciphertext_b64)
            .expect("Failed to decode ciphertext");
        c[0] ^= 0xFF;
        ks_bad.ciphertext_b64 = general_purpose::STANDARD.encode(&c);
        assert!(decrypt_keystore(&ks_bad, "pw").is_err());
    }

    #[test]
    fn invalid_magic_rejected() {
        let secret = b"test";
        let mut ks = encrypt_keystore(secret, "pw", None, 8 * 1024, 1, 1);
        ks.magic = "FAKE".to_string();
        assert!(decrypt_keystore(&ks, "pw").is_err());
    }

    #[test]
    fn future_version_rejected() {
        let secret = b"test";
        let mut ks = encrypt_keystore(secret, "pw", None, 8 * 1024, 1, 1);
        ks.version = 99;
        let result = decrypt_keystore(&ks, "pw");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("unsupported version"));
    }

    #[test]
    fn large_secret_roundtrip() {
        let secret = vec![0x42u8; 32 * 1024];
        let ks = encrypt_keystore(&secret, "longpw", None, 8 * 1024, 1, 1);
        let pt = decrypt_keystore(&ks, "longpw").expect("decrypt");
        assert_eq!(pt, secret);
    }

    #[test]
    fn kdf_profile_params() {
        let optimal = optimal_parallelism();
        let (mem, time, par) = KdfProfile::Tight.params();
        assert_eq!(mem, 65536);
        assert_eq!(time, 3);
        assert_eq!(par, optimal);

        let (mem, time, par) = KdfProfile::Mobile.params();
        assert_eq!(mem, 8192);
        assert_eq!(time, 3);
        assert_eq!(par, optimal.min(2)); // Mobile caps at 2 threads
    }

    #[test]
    fn rotate_password() {
        let secret = b"my-key";
        let ks = encrypt_keystore(secret, "old-pw", None, 8 * 1024, 1, 1);
        let rotated = rotate_keystore(&ks, "old-pw", "new-pw", 8 * 1024, 1, 1).expect("rotate");

        assert!(decrypt_keystore(&rotated, "old-pw").is_err());

        let pt = decrypt_keystore(&rotated, "new-pw").expect("decrypt with new pw");
        assert_eq!(pt, secret);
    }
}

#[test]
fn corrupted_file_handling() {
    use tempfile::tempdir;
    let dir = tempdir().expect("Failed to create temp directory");
    let path = dir.path().join("corrupt.keystore");

    // Invalid JSON
    std::fs::write(&path, b"{invalid}").expect("Failed to write invalid JSON");
    assert!(read_keystore_file(&path).is_err());

    // Truncated valid keystore
    let ks = encrypt_keystore(b"test", "pw", None, 8 * 1024, 1, 1);
    let json = ks.to_json().expect("Failed to serialize keystore");
    std::fs::write(&path, &json[..json.len() / 2]).expect("Failed to write truncated keystore");
    assert!(read_keystore_file(&path).is_err());

    // Missing required fields
    std::fs::write(&path, r#"{"version": 1}"#).expect("Failed to write incomplete keystore");
    assert!(read_keystore_file(&path).is_err());
}

#[test]
fn decrypt_corrupted_fields() {
    let ks = encrypt_keystore(b"test", "pw", None, 8 * 1024, 1, 1);

    // Corrupt salt
    let mut bad = ks.clone();
    bad.kdf.salt_b64 = "invalid!!!base64".to_string();
    assert!(decrypt_keystore(&bad, "pw").is_err());

    // Corrupt nonce
    let mut bad = ks.clone();
    bad.nonce_b64 = "bad".to_string();
    assert!(decrypt_keystore(&bad, "pw").is_err());

    // Corrupt ciphertext
    let mut bad = ks.clone();
    bad.ciphertext_b64 = "xyz".to_string();
    assert!(decrypt_keystore(&bad, "pw").is_err());
}
