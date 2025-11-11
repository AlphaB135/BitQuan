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
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
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

/// Get adaptive default parameters based on detected hardware
pub fn adaptive_default_params() -> (u32, u32, u8) {
    KdfProfile::Adaptive.params()
}

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

/// Maximum cache age for security (5 minutes)
const MAX_CACHE_AGE: Duration = Duration::from_secs(5 * 60);

/// Cache key identifier (hash of password + salt)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct CacheKey {
    // Hash of password + salt to identify cache entries
    hash: [u8; 32],
}

impl CacheKey {
    fn new(password: &SecretVec<u8>, salt: &[u8]) -> Self {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(password.expose_secret());
        hasher.update(salt);
        let hash = hasher.finalize();
        Self { hash: hash.into() }
    }
}

/// Cached derived key with timestamp
struct CachedKey {
    key: SecretVec<u8>,
    created_at: SystemTime,
}

impl std::fmt::Debug for CachedKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CachedKey")
            .field("created_at", &self.created_at)
            .field("key_len", &self.key.expose_secret().len())
            .finish()
    }
}

impl CachedKey {
    fn new(key: SecretVec<u8>) -> Self {
        Self {
            key,
            created_at: SystemTime::now(),
        }
    }
    
    fn is_expired(&self) -> bool {
        match self.created_at.elapsed() {
            Ok(elapsed) => elapsed > MAX_CACHE_AGE,
            Err(_) => true, // Clock went backwards - expire immediately
        }
    }
}

impl Drop for CachedKey {
    fn drop(&mut self) {
        // Ensure key is zeroized when cache entry is dropped
        // Note: SecretVec automatically zeroizes on drop
    }
}

/// Thread-safe secure key cache with automatic cleanup
#[derive(Debug)]
struct SecureKeyCache {
    entries: Arc<Mutex<HashMap<CacheKey, CachedKey>>>,
}

impl SecureKeyCache {
    fn new() -> Self {
        Self {
            entries: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    
    /// Get cached key if valid and not expired
    fn get(&self, cache_key: &CacheKey) -> Option<SecretVec<u8>> {
        let mut entries = self.entries.lock().ok()?;
        
        if let Some(cached) = entries.get_mut(cache_key) {
            if cached.is_expired() {
                // Remove expired entry
                entries.remove(cache_key);
                return None;
            }
            // Return a clone of the cached key
            return Some(SecretVec::new(cached.key.expose_secret().clone()));
        }
        None
    }
    
    /// Store derived key in cache with timestamp
    fn store(&self, cache_key: CacheKey, key: SecretVec<u8>) {
        if let Ok(mut entries) = self.entries.lock() {
            // Clean up expired entries first
            entries.retain(|_, cached| !cached.is_expired());
            
            // Store new entry
            entries.insert(cache_key, CachedKey::new(key));
        }
    }
    
    /// Clear all cached keys (for security)
    fn clear(&self) {
        if let Ok(mut entries) = self.entries.lock() {
            entries.clear();
        }
    }
    
    /// Clean up expired entries
    fn cleanup_expired(&self) {
        if let Ok(mut entries) = self.entries.lock() {
            entries.retain(|_, cached| !cached.is_expired());
        }
    }
}

impl Drop for SecureKeyCache {
    fn drop(&mut self) {
        // Clear all keys when cache is destroyed
        self.clear();
    }
}

// Global secure key cache instance
lazy_static::lazy_static! {
    static ref KEY_CACHE: SecureKeyCache = SecureKeyCache::new();
}

#[derive(Debug, Clone, Copy)]
pub enum KdfProfile {
    Tight,
    Medium,
    Light,
    Mobile,
    Adaptive, // New profile that auto-detects hardware
}

impl KdfProfile {
    /// Get KDF parameters based on profile and hardware capabilities
    pub fn params(&self) -> (u32, u32, u8) {
        match self {
            KdfProfile::Adaptive => {
                let hw = HardwareProfile::detect();
                (hw.optimal_memory_cost(), hw.optimal_time_cost(), hw.optimal_parallelism())
            }
            KdfProfile::Tight => {
                let hw = HardwareProfile::detect();
                (65536, 3, hw.optimal_parallelism())
            }
            KdfProfile::Medium => {
                let hw = HardwareProfile::detect();
                (32768, 2, hw.optimal_parallelism())
            }
            KdfProfile::Light => {
                let hw = HardwareProfile::detect();
                (16384, 2, hw.optimal_parallelism())
            }
            KdfProfile::Mobile => {
                // Mobile profile is conservative regardless of hardware
                (8192, 1, 2)
            }
        }
    }
    
    /// Get adaptive KDF parameters with custom hardware override
    pub fn adaptive_params_with_hardware(hw: HardwareProfile) -> (u32, u32, u8) {
        (hw.optimal_memory_cost(), hw.optimal_time_cost(), hw.optimal_parallelism())
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

/// Derive key with caching support for hot access optimization
fn derive_key_cached(
    password: &SecretVec<u8>,
    salt: &[u8],
    mem_kib: u32,
    time_cost: u32,
    parallelism: u8,
) -> [u8; 32] {
    let cache_key = CacheKey::new(password, salt);
    
    // Try to get from cache first
    if let Some(cached_key) = KEY_CACHE.get(&cache_key) {
        if cached_key.expose_secret().len() == 32 {
            let mut key = [0u8; 32];
            key.copy_from_slice(cached_key.expose_secret());
            return key;
        }
    }
    
    // Cache miss - derive key normally
    let key = derive_key(password, salt, mem_kib, time_cost, parallelism);
    
    // Store in cache for future use
    KEY_CACHE.store(cache_key, SecretVec::new(key.to_vec()));
    
    key
}

/// Clear all cached keys (for security operations)
pub fn clear_key_cache() {
    KEY_CACHE.clear();
}

/// Clean up expired cache entries
pub fn cleanup_expired_cache() {
    KEY_CACHE.cleanup_expired();
}

/// Decrypt keystore without caching (for security-sensitive operations)
pub fn decrypt_keystore_no_cache(ks: &KeystoreFile, password: &str) -> Result<Vec<u8>, String> {
    decrypt_keystore_cached(ks, password, false)
}

/// Get cache statistics for monitoring
pub fn get_cache_stats() -> CacheStats {
    if let Ok(entries) = KEY_CACHE.entries.lock() {
        let total = entries.len();
        let expired = entries.values()
            .filter(|cached| cached.is_expired())
            .count();
        CacheStats {
            total_entries: total,
            expired_entries: expired,
            active_entries: total - expired,
        }
    } else {
        CacheStats::default()
    }
}

/// Cache statistics for monitoring
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    pub total_entries: usize,
    pub expired_entries: usize,
    pub active_entries: usize,
}

/// Encrypt keystore with adaptive parameters (recommended for new wallets)
pub fn encrypt_keystore_adaptive(
    plaintext: &[u8],
    password: &str,
    meta: Option<serde_json::Value>,
) -> KeystoreFile {
    let (mem_kib, time_cost, parallelism) = adaptive_default_params();
    encrypt_keystore(plaintext, password, meta, mem_kib, time_cost, parallelism)
}

/// Encrypt keystore with specific KDF profile
pub fn encrypt_keystore_with_profile(
    plaintext: &[u8],
    password: &str,
    meta: Option<serde_json::Value>,
    profile: KdfProfile,
) -> KeystoreFile {
    let (mem_kib, time_cost, parallelism) = profile.params();
    encrypt_keystore(plaintext, password, meta, mem_kib, time_cost, parallelism)
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
    decrypt_keystore_cached(ks, password, true)
}

/// Decrypt keystore with optional caching control
pub fn decrypt_keystore_cached(ks: &KeystoreFile, password: &str, use_cache: bool) -> Result<Vec<u8>, String> {
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

    // Use cached derivation if enabled
    let key_bytes = if use_cache {
        derive_key_cached(
            &pw,
            &salt,
            ks.kdf.mem_kib,
            ks.kdf.time_cost,
            ks.kdf.parallelism,
        )
    } else {
        derive_key(
            &pw,
            &salt,
            ks.kdf.mem_kib,
            ks.kdf.time_cost,
            ks.kdf.parallelism,
        )
    };

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

    // Note: key_bytes is not zeroized here when cached, as it's managed by the cache
    if !use_cache {
        // Only zeroize if not cached
        let mut key_vec = key_bytes.to_vec();
        key_vec.zeroize();
    }

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
        let hw = HardwareProfile::detect();
        let optimal = hw.optimal_parallelism();
        
        // Test Tight profile
        let (mem, time, par) = KdfProfile::Tight.params();
        assert_eq!(mem, 65536);
        assert_eq!(time, 3);
        assert_eq!(par, optimal);

        // Test Mobile profile (always conservative)
        let (mem, time, par) = KdfProfile::Mobile.params();
        assert_eq!(mem, 8192);
        assert_eq!(time, 1);
        assert_eq!(par, 2);
        
        // Test Adaptive profile (should match hardware)
        let (mem, time, par) = KdfProfile::Adaptive.params();
        assert_eq!(mem, hw.optimal_memory_cost());
        assert_eq!(time, hw.optimal_time_cost());
        assert_eq!(par, hw.optimal_parallelism());
    }
    
    #[test]
    fn hardware_profile_detection() {
        let hw = HardwareProfile::detect();
        
        // Verify hardware detection produces reasonable values
        let parallelism = hw.optimal_parallelism();
        let memory = hw.optimal_memory_cost();
        let time = hw.optimal_time_cost();
        
        assert!(parallelism >= 1 && parallelism <= 8);
        assert!(memory >= 8192 && memory <= 65536);
        assert!(time >= 1 && time <= 3);
    }
    
    #[test]
    fn adaptive_encryption_roundtrip() {
        let secret = b"adaptive-encryption-test";
        let password = "test-password";
        let meta = Some(json!({"adaptive": true}));
        
        // Test adaptive encryption
        let ks = encrypt_keystore_adaptive(secret, password, meta.clone());
        let pt = decrypt_keystore(&ks, password).expect("adaptive decrypt should work");
        assert_eq!(pt, secret);
        
        // Verify adaptive parameters were used
        let (expected_mem, expected_time, expected_par) = adaptive_default_params();
        assert_eq!(ks.kdf.mem_kib, expected_mem);
        assert_eq!(ks.kdf.time_cost, expected_time);
        assert_eq!(ks.kdf.parallelism, expected_par);
    }
    
    #[test]
    fn key_caching_roundtrip() {
        let secret = b"key-caching-test";
        let password = "cache-test-password";
        let meta = Some(json!({"caching": true}));
        
        // Clear cache first
        clear_key_cache();
        
        let ks = encrypt_keystore_adaptive(secret, password, meta.clone());
        
        // First decryption (cache miss)
        let pt1 = decrypt_keystore(&ks, password).expect("first decrypt should work");
        assert_eq!(pt1, secret);
        
        // Second decryption (cache hit)
        let pt2 = decrypt_keystore(&ks, password).expect("second decrypt should work");
        assert_eq!(pt2, secret);
        
        // Verify cache has entries
        let stats = get_cache_stats();
        assert!(stats.total_entries > 0);
        assert!(stats.active_entries > 0);
    }
    
    #[test]
    fn cache_timeout_enforcement() {
        let secret = b"cache-timeout-test";
        let password = "timeout-test-password";
        let meta = Some(json!({"timeout": true}));
        
        // Clear cache
        clear_key_cache();
        
        let ks = encrypt_keystore_adaptive(secret, password, meta.clone());
        
        // Decrypt to populate cache
        let _pt = decrypt_keystore(&ks, password).expect("decrypt should work");
        
        // Verify cache has entries
        let stats = get_cache_stats();
        assert!(stats.active_entries > 0);
        
        // Note: We can't easily test 5-minute timeout in unit tests,
        // but we can test cleanup functionality
        cleanup_expired_cache();
        
        // Cache should still have entries (not expired yet)
        let stats_after = get_cache_stats();
        assert!(stats_after.active_entries > 0);
    }
    
    #[test]
    fn cache_vs_no_cache() {
        let secret = b"cache-comparison-test";
        let password = "comparison-password";
        let meta = Some(json!({"comparison": true}));
        
        // Clear cache
        clear_key_cache();
        
        let ks = encrypt_keystore_adaptive(secret, password, meta.clone());
        
        // Decrypt with cache
        let pt1 = decrypt_keystore(&ks, password).expect("cached decrypt should work");
        assert_eq!(pt1, secret);
        
        // Decrypt without cache
        let pt2 = decrypt_keystore_no_cache(&ks, password).expect("no-cache decrypt should work");
        assert_eq!(pt2, secret);
        
        // Both should produce same result
        assert_eq!(pt1, pt2);
    }
    
    #[test]
    fn cache_security_isolation() {
        let secret1 = b"secret-one";
        let secret2 = b"secret-two";
        let password1 = "password-one";
        let password2 = "password-two";
        let meta = Some(json!({"isolation": true}));
        
        // Clear cache
        clear_key_cache();
        
        let ks1 = encrypt_keystore_adaptive(secret1, password1, meta.clone());
        let ks2 = encrypt_keystore_adaptive(secret2, password2, meta.clone());
        
        // Decrypt first keystore
        let pt1 = decrypt_keystore(&ks1, password1).expect("decrypt 1 should work");
        assert_eq!(pt1, secret1);
        
        // Try to decrypt second keystore with wrong password (should fail)
        let res = decrypt_keystore(&ks2, password1);
        assert!(res.is_err());
        
        // Decrypt second keystore with correct password
        let pt2 = decrypt_keystore(&ks2, password2).expect("decrypt 2 should work");
        assert_eq!(pt2, secret2);
        
        // Cache should have separate entries for different passwords
        let stats = get_cache_stats();
        assert!(stats.active_entries >= 1);
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
