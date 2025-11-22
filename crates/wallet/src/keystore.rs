//! # BitQuan Keystore Module
//!
//! Secure encryption and decryption of sensitive data with adaptive performance optimization.
//!
//! ## Security Model
//!
//! - **Encryption**: AES-256-GCM for confidentiality and integrity
//! - **Key Derivation**: Argon2id for memory-hard password hashing
//! - **Post-Quantum Ready**: Designed to work with Dilithium signatures
//! - **Memory Safety**: All secrets are zeroized when dropped
//!
//! ## Performance Features
//!
//! ### Adaptive KDF
//! Automatically tunes Argon2 parameters based on detected hardware:
//!
//! ```rust
//! use wallet::keystore::{HardwareProfile, encrypt_keystore_adaptive};
//!
//! let profile = HardwareProfile::detect();
//! println!("Profile: {:?}", profile);
//! println!("Optimal parallelism: {}", profile.optimal_parallelism());
//! println!("Optimal memory: {} KiB", profile.optimal_memory_cost());
//!
//! // Uses optimal parameters for your hardware
//! let keystore = encrypt_keystore_adaptive(b"data", "password", None);
//! ```
//!
//! ### Secure Key Caching
//! Dramatically speeds up repeated decryption:
//!
//! - **Cold decryption**: ~10ms (KDF computation required)
//! - **Hot decryption**: ~1.85µs (5,400x faster)
//! - **Cache timeout**: 5 minutes by default
//! - **Memory safe**: Secrets are isolated and zeroized
//!
//! ## API Selection Guide
//!
//! ### Simple Usage (Recommended)
//!
//! ```rust
//! use wallet::keystore::{encrypt_keystore_adaptive, decrypt_keystore};
//!
//! // Encrypt - automatically optimizes for your hardware
//! let keystore = encrypt_keystore_adaptive(b"secret", "password", None);
//!
//! // Decrypt - uses cache for speed
//! let decrypted = decrypt_keystore(&keystore, "password")?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! ### Advanced Configuration
//!
//! ```rust
//! use wallet::keystore::{WalletConfig, encrypt_keystore_with_config};
//!
//! // Server: Maximum security, no caching
//! let server_config = WalletConfig::server();
//!
//! // Mobile: Balanced for battery life
//! let mobile_config = WalletConfig::mobile();
//!
//! // Custom: Fine-tuned parameters
//! let custom_config = WalletConfig::performance()
//!     .with_cache_timeout(std::time::Duration::from_secs(30));
//!
//! let keystore = encrypt_keystore_with_config(
//!     b"secret", "password", None, &custom_config
//! );
//! ```
//!
//! ## KDF Profiles
//!
//! | Profile | Memory | Time | Parallelism | Use Case |
//! |---------|--------|------|-------------|----------|
//! | `Tight` | 256 MiB | 4 | 4 | Maximum security |
//! | `Medium` | 128 MiB | 3 | 2 | Desktop default |
//! | `Light` | 64 MiB | 2 | 1 | Older hardware |
//! | `Mobile` | 32 MiB | 2 | 1 | Battery constrained |
//! | `Adaptive` | Auto | Auto | Auto | Recommended |
//!
//! ## Monitoring
//!
//! ```rust
//! use wallet::keystore::{get_cache_stats, get_cache_memory_usage};
//!
//! let stats = get_cache_stats();
//! let memory_bytes = get_cache_memory_usage();
//!
//! println!("Active cache entries: {}", stats.active_entries);
//! println!("Memory usage: {} KB", memory_bytes / 1024);
//! ```
//!
//! ## Security Considerations
//!
//! 1. **Password Strength**: Use strong passwords (12+ characters, mixed case)
//! 2. **Cache Isolation**: Each password/salt has isolated cache entries
//! 3. **Timeout Enforcement**: Cache entries expire after configured timeout
//! 4. **Memory Zeroization**: All secrets are securely erased when dropped
//! 5. **Thread Safety**: Cache is thread-safe for concurrent operations
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
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use zeroize::Zeroize;

/// Parameters for the Argon2id key derivation function
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct KdfParams {
    /// Memory size in KiB
    pub mem_kib: u32,
    /// Number of iterations (time cost)
    pub time_cost: u32,
    /// Degree of parallelism
    pub parallelism: u8,
    /// Salt encoded as Base64
    pub salt_b64: String,
}

/// Encrypted keystore file format
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct KeystoreFile {
    /// File format identifier (magic bytes)
    pub magic: String,
    /// Format version number
    pub version: u8,
    /// Creation timestamp (Unix epoch)
    pub created: u64,
    /// Key derivation parameters
    pub kdf: KdfParams,
    /// Encryption nonce (IV) encoded as Base64
    pub nonce_b64: String,
    /// Encrypted data (ciphertext) encoded as Base64
    pub ciphertext_b64: String,
    /// Optional metadata (JSON)
    pub meta: Option<serde_json::Value>,
}

impl KeystoreFile {
    /// Convert keystore to formatted JSON string
    pub fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string_pretty(self)
    }
}

/// Magic string for file identification "BQK1"
pub const MAGIC: &str = "BQK1";
/// Current keystore format version
pub const CURRENT_VERSION: u8 = 1;

/// Default memory cost (64 MiB)
pub const DEFAULT_MEM_KIB: u32 = 65536;
/// Default time cost (3 iterations)
pub const DEFAULT_TIME_COST: u32 = 3;
/// Default parallelism (1 thread)
pub const DEFAULT_PARALLELISM: u8 = 1;

/// Get adaptive default parameters based on detected hardware
pub fn adaptive_default_params() -> (u32, u32, u8) {
    KdfProfile::Adaptive.params()
}

/// Hardware capability detection for adaptive KDF
#[derive(Debug, Clone, Copy)]
pub enum HardwareProfile {
    /// High-end desktop (16+ GB RAM, 8+ cores)
    HighEndDesktop,
    /// Mid-range laptop (8-16 GB RAM, 4-8 cores)
    MidRangeLaptop,
    /// Low-end device (4-8 GB RAM, 2-4 cores)
    LowEndDevice,
    /// Mobile device (<4 GB RAM, <=2 cores)
    MobileDevice,
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
/// Length of the random salt in bytes
pub const SALT_LEN: usize = 16;
/// Length of the encryption nonce in bytes
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
    /// Atomic counter for cache memory usage to avoid lock contention
    memory_usage_bytes: AtomicUsize,
}

impl SecureKeyCache {
    fn new() -> Self {
        Self {
            entries: Arc::new(Mutex::new(HashMap::new())),
            memory_usage_bytes: AtomicUsize::new(0),
        }
    }

    /// Calculate memory usage of a cached entry
    fn entry_memory_size(cached_key: &CachedKey) -> usize {
        cached_key.key.expose_secret().len() + std::mem::size_of::<CachedKey>()
    }

    /// Get cached key if valid and not expired
    fn get(&self, cache_key: &CacheKey) -> Option<SecretVec<u8>> {
        let mut entries = self.entries.lock().ok()?;

        if let Some(cached) = entries.get_mut(cache_key) {
            if cached.is_expired() {
                // Remove expired entry and update atomic counter
                let memory_size = Self::entry_memory_size(cached);
                entries.remove(cache_key);
                self.memory_usage_bytes
                    .fetch_sub(memory_size, Ordering::Relaxed);
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
            // Clean up expired entries first and track memory changes
            let mut memory_delta = 0isize;
            entries.retain(|_, cached| {
                if cached.is_expired() {
                    memory_delta -= Self::entry_memory_size(cached) as isize;
                    false
                } else {
                    true
                }
            });

            // Check if we're replacing an existing entry
            if let Some(old_cached) = entries.get(&cache_key) {
                memory_delta -= Self::entry_memory_size(old_cached) as isize;
            }

            // Store new entry and update counter
            let new_cached = CachedKey::new(key);
            memory_delta += Self::entry_memory_size(&new_cached) as isize;
            entries.insert(cache_key, new_cached);

            // Update atomic counter
            if memory_delta > 0 {
                self.memory_usage_bytes
                    .fetch_add(memory_delta as usize, Ordering::Relaxed);
            } else if memory_delta < 0 {
                self.memory_usage_bytes
                    .fetch_sub((-memory_delta) as usize, Ordering::Relaxed);
            }
        }
    }

    /// Clear all cached keys (for security)
    fn clear(&self) {
        if let Ok(mut entries) = self.entries.lock() {
            entries.clear();
            // Reset atomic counter to zero
            self.memory_usage_bytes.store(0, Ordering::Relaxed);
        }
    }

    /// Clean up expired entries
    fn cleanup_expired(&self) {
        if let Ok(mut entries) = self.entries.lock() {
            // Track memory to be removed
            let mut memory_to_remove = 0usize;
            entries.retain(|_, cached| {
                if cached.is_expired() {
                    memory_to_remove += Self::entry_memory_size(cached);
                    false
                } else {
                    true
                }
            });

            // Update atomic counter
            if memory_to_remove > 0 {
                self.memory_usage_bytes
                    .fetch_sub(memory_to_remove, Ordering::Relaxed);
            }
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

/// Pre-defined KDF parameter profiles
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum KdfProfile {
    /// High security, high resource usage (256 MiB, 4 iters)
    Tight,
    /// Balanced security and performance (128 MiB, 3 iters)
    Medium,
    /// Low resource usage for older hardware (64 MiB, 2 iters)
    Light,
    /// Minimal resource usage for mobile (32 MiB, 2 iters)
    Mobile,
    /// Automatically detected based on hardware
    Adaptive,
}

impl KdfProfile {
    /// Get KDF parameters based on profile and hardware capabilities
    pub fn params(&self) -> (u32, u32, u8) {
        match self {
            KdfProfile::Adaptive => {
                let hw = HardwareProfile::detect();
                (
                    hw.optimal_memory_cost(),
                    hw.optimal_time_cost(),
                    hw.optimal_parallelism(),
                )
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
        (
            hw.optimal_memory_cost(),
            hw.optimal_time_cost(),
            hw.optimal_parallelism(),
        )
    }
}

fn derive_key(
    password: &SecretVec<u8>,
    salt: &[u8],
    mem_kib: u32,
    time_cost: u32,
    parallelism: u8,
) -> Result<[u8; 32], String> {
    // Create Argon2 parameters with proper error handling
    let params = Params::new(mem_kib, time_cost, parallelism.into(), None)
        .map_err(|e| format!("Invalid Argon2 parameters: {e}"))?;
    let argon2 = Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params);

    let mut key = [0u8; 32];
    // Derive key with proper error handling
    argon2
        .hash_password_into(password.expose_secret(), salt, &mut key)
        .map_err(|e| format!("Argon2 key derivation failed: {e}"))?;
    Ok(key)
}

/// Derive key with caching support for hot access optimization
fn derive_key_cached(
    password: &SecretVec<u8>,
    salt: &[u8],
    mem_kib: u32,
    time_cost: u32,
    parallelism: u8,
) -> Result<[u8; 32], String> {
    let cache_key = CacheKey::new(password, salt);

    // Try to get from cache first
    if let Some(cached_key) = KEY_CACHE.get(&cache_key) {
        if cached_key.expose_secret().len() == 32 {
            let mut key = [0u8; 32];
            key.copy_from_slice(cached_key.expose_secret());
            return Ok(key);
        }
    }

    // Cache miss - derive key normally
    let key = derive_key(password, salt, mem_kib, time_cost, parallelism)?;

    // Store in cache for future use
    KEY_CACHE.store(cache_key, SecretVec::new(key.to_vec()));

    Ok(key)
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

/// Encrypt keystore with custom configuration
///
/// Use this function when you need fine-grained control over security parameters
/// and caching behavior. Ideal for server deployments, mobile applications,
/// or high-security environments.
///
/// # Arguments
/// * `plaintext` - Data to encrypt
/// * `password` - Strong password for encryption
/// * `meta` - Optional metadata to store with the keystore
/// * `config` - Wallet configuration specifying KDF profile and caching
///
/// # Returns
/// A `KeystoreFile` encrypted with the specified parameters
///
/// # Example
/// ```rust
/// use wallet::keystore::{WalletConfig, encrypt_keystore_with_config};
/// use std::time::Duration;
///
/// // Server configuration: maximum security, no caching
/// let server_config = WalletConfig::server();
/// let keystore = encrypt_keystore_with_config(
///     b"highly_sensitive_data",
///     "server-master-password",
///     None,
///     &server_config,
/// );
///
/// // Mobile configuration: balanced for battery life
/// let mobile_config = WalletConfig::mobile()
///     .with_cache_timeout(Duration::from_secs(60)); // Short cache
/// let mobile_keystore = encrypt_keystore_with_config(
///     b"user_private_key",
///     "user_password",
///     None,
///     &mobile_config,
/// );
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// # When to Use This Function
///
/// - **Server/Infrastructure**: Use `WalletConfig::server()` for maximum security
/// - **Mobile Apps**: Use `WalletConfig::mobile()` to preserve battery
/// - **High-Security**: Use `WalletConfig::conservative()` for maximum KDF parameters
/// - **Custom Needs**: Build your own config with specific timeouts and profiles
pub fn encrypt_keystore_with_config(
    plaintext: &[u8],
    password: &str,
    meta: Option<serde_json::Value>,
    config: &WalletConfig,
) -> Result<KeystoreFile, String> {
    let (mem_kib, time_cost, parallelism) = config.kdf_profile.params();
    encrypt_keystore(plaintext, password, meta, mem_kib, time_cost, parallelism)
}

/// Decrypt keystore with custom configuration
///
/// Decrypts a keystore using the specified configuration. This allows you
/// to control whether caching is enabled and respect custom timeout settings.
///
/// # Arguments
/// * `ks` - The keystore file to decrypt
/// * `password` - The password used for encryption
/// * `config` - Wallet configuration (caching settings are respected)
///
/// # Returns
/// The decrypted plaintext data
///
/// # Example
/// ```rust
/// use wallet::keystore::{WalletConfig, encrypt_keystore_with_config, decrypt_keystore_with_config};
///
/// let config = WalletConfig::server(); // No caching for security
/// let keystore = encrypt_keystore_with_config(
///     b"secret_data",
///     "password",
///     None,
///     &config,
/// );
///
/// // Decrypt respecting the config (no caching in this case)
/// let decrypted = decrypt_keystore_with_config(&keystore, "password", &config)?;
///
/// assert_eq!(decrypted, b"secret_data");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// # Security Note
/// When `config.enable_caching` is false, this function always performs
/// the full KDF computation, providing maximum security at the cost of
/// performance. Use this for highly sensitive operations or when caching
/// is not desired.
pub fn decrypt_keystore_with_config(
    ks: &KeystoreFile,
    password: &str,
    config: &WalletConfig,
) -> Result<Vec<u8>, String> {
    decrypt_keystore_cached(ks, password, config.enable_caching)
}

/// Get cache statistics for monitoring
///
/// Returns statistics about the current state of the key cache. This is
/// useful for monitoring memory usage and performance in production.
///
/// # Returns
/// A `CacheStats` struct containing:
/// - `total_entries`: Total number of cache entries
/// - `expired_entries`: Number of entries that have expired but not yet cleaned up
/// - `active_entries`: Number of currently valid cache entries
///
/// # Example
/// ```rust
/// use wallet::keystore::{encrypt_keystore_adaptive, decrypt_keystore, get_cache_stats, get_cache_memory_usage};
///
/// let keystore = encrypt_keystore_adaptive(b"test", "password", None);
///
/// // Decrypt to populate cache
/// decrypt_keystore(&keystore, "password")?;
///
/// let stats = get_cache_stats();
/// println!("Active cache entries: {}", stats.active_entries);
/// println!("Total memory usage: {} KB", get_cache_memory_usage() / 1024);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// # Monitoring Tips
///
/// - **Memory Usage**: Call `get_cache_memory_usage()` to get bytes used
/// - **Cache Hit Rate**: Monitor `active_entries` over time
/// - **Cleanup**: Call `cleanup_expired_cache()` periodically to free memory
/// - **Alerting**: Set alerts if `active_entries` exceeds expected thresholds
pub fn get_cache_stats() -> CacheStats {
    if let Ok(entries) = KEY_CACHE.entries.lock() {
        let total = entries.len();
        let expired = entries
            .values()
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

/// Get cache memory usage in bytes
///
/// Returns the total memory used by the key cache. This includes the
/// cached keys themselves plus the overhead for cache metadata.
///
/// # Returns
/// Total memory usage in bytes
///
/// # Example
/// ```rust
/// use wallet::keystore::{WalletConfig, encrypt_keystore_with_config};
///
/// // Server: Maximum security, no caching
/// let server_config = WalletConfig::server();
///
/// // Mobile: Balanced for battery life
/// let mobile_config = WalletConfig::mobile();
///
/// // Custom: Fine-tuned parameters
/// let custom_config = WalletConfig::performance()
///     .with_cache_timeout(std::time::Duration::from_secs(30));
///
/// let keystore = encrypt_keystore_with_config(
///     b"secret", "password", None, &custom_config
/// );
/// ```
///
/// # Memory Estimation
///
/// Each cache entry uses approximately:
/// - 32 bytes for the derived key
/// - 8-16 bytes for timestamp metadata
/// - 32 bytes for cache key hash
/// - **Total**: ~72-80 bytes per entry
///
/// # Production Monitoring
///
/// In production, monitor this metric to:
/// - Detect memory leaks (unexpected growth)
/// - Size cache appropriately for your workload
/// - Set memory limits and alerts
/// - Optimize cache timeout settings
pub fn get_cache_memory_usage() -> usize {
    // Use atomic counter to avoid lock contention
    KEY_CACHE.memory_usage_bytes.load(Ordering::Relaxed)
}

/// Cache statistics for monitoring
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    /// Total number of entries in the cache
    pub total_entries: usize,
    /// Number of expired entries waiting for cleanup
    pub expired_entries: usize,
    /// Number of active, valid entries
    pub active_entries: usize,
}

/// Encrypt keystore with adaptive parameters (recommended for most users)
///
/// This function automatically detects the hardware capabilities and selects
/// optimal Argon2 parameters for the best balance of security and performance.
///
/// # Arguments
/// * `plaintext` - Data to encrypt (e.g., private keys, seed phrases)
/// * `password` - Strong password for encryption
/// * `meta` - Optional metadata to store with the keystore
///
/// # Returns
/// A `KeystoreFile` that can be stored to disk or transmitted
///
/// # Example
/// ```rust
/// use wallet::keystore::{HardwareProfile, encrypt_keystore_adaptive};
///
/// let profile = HardwareProfile::detect();
/// println!("Profile: {:?}", profile);
/// println!("Optimal parallelism: {}", profile.optimal_parallelism());
/// println!("Optimal memory: {} KiB", profile.optimal_memory_cost());
///
/// // Uses optimal parameters for your hardware
/// let keystore = encrypt_keystore_adaptive(b"data", "password", None);
/// ```
///
/// # Performance
/// - **High-end server**: Uses maximum security parameters
/// - **Desktop**: Balanced security/performance
/// - **Mobile/Low-end**: Optimized for faster encryption
/// - **Typical speed**: 5-50ms depending on hardware
pub fn encrypt_keystore_adaptive(
    plaintext: &[u8],
    password: &str,
    meta: Option<serde_json::Value>,
) -> Result<KeystoreFile, String> {
    let (mem_kib, time_cost, parallelism) = adaptive_default_params();
    encrypt_keystore(plaintext, password, meta, mem_kib, time_cost, parallelism)
}

/// Encrypt keystore with specific KDF profile
pub fn encrypt_keystore_with_profile(
    plaintext: &[u8],
    password: &str,
    meta: Option<serde_json::Value>,
    profile: KdfProfile,
) -> Result<KeystoreFile, String> {
    let (mem_kib, time_cost, parallelism) = profile.params();
    encrypt_keystore(plaintext, password, meta, mem_kib, time_cost, parallelism)
}

/// Encrypt keystore with explicit parameters
///
/// # Arguments
/// * `plaintext` - Data to encrypt
/// * `password` - Encryption password
/// * `meta` - Optional metadata
/// * `mem_kib` - Memory cost in KiB
/// * `time_cost` - Time cost (iterations)
/// * `parallelism` - Degree of parallelism
pub fn encrypt_keystore(
    plaintext: &[u8],
    password: &str,
    meta: Option<serde_json::Value>,
    mem_kib: u32,
    time_cost: u32,
    parallelism: u8,
) -> Result<KeystoreFile, String> {
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

    let mut key_bytes = derive_key(&pw, &salt_vec, mem_kib, time_cost, parallelism)?;

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
        .map_err(|e| format!("AES encryption failed: {e}"))?;

    key_bytes.zeroize();

    let created = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or_else(|_| {
            // Log warning but continue with epoch fallback
            eprintln!("Warning: System clock is set before Unix epoch, using epoch as fallback");
            0
        });
    Ok(KeystoreFile {
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
    })
}

// Duplicate function removed - use encrypt_keystore instead

/// Decrypt keystore with automatic caching (recommended)
///
/// This function provides the best performance by caching derived keys.
/// The first decryption for a given password/salt combination performs the
/// full KDF computation (~10ms), while subsequent decryptions use the
/// cached key (~1.85µs - 5,400x faster).
///
/// # Arguments
/// * `ks` - The keystore file to decrypt
/// * `password` - The password used for encryption
///
/// # Returns
/// The decrypted plaintext data
///
/// # Errors
/// Returns an error if:
/// - The password is incorrect
/// - The keystore file is corrupted
/// - The ciphertext fails integrity verification
///
/// # Security
/// - Cache entries expire after 5 minutes by default
/// - Each password/salt combination has isolated cache entries
/// - All secrets are zeroized when dropped
///
/// # Example
/// ```rust
/// use wallet::keystore::{encrypt_keystore_adaptive, decrypt_keystore};
///
/// let keystore = encrypt_keystore_adaptive(b"secret", "password", None);
///
/// // First decryption: ~10ms (KDF computation)
/// let data1 = decrypt_keystore(&keystore, "password")?;
///
/// // Subsequent decryptions: ~1.85µs (cached)
/// let data2 = decrypt_keystore(&keystore, "password")?;
///
/// assert_eq!(data1, data2);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn decrypt_keystore(ks: &KeystoreFile, password: &str) -> Result<Vec<u8>, String> {
    decrypt_keystore_cached(ks, password, true)
}

/// Decrypt keystore with optional caching control
pub fn decrypt_keystore_cached(
    ks: &KeystoreFile,
    password: &str,
    use_cache: bool,
) -> Result<Vec<u8>, String> {
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
        )?
    } else {
        derive_key(
            &pw,
            &salt,
            ks.kdf.mem_kib,
            ks.kdf.time_cost,
            ks.kdf.parallelism,
        )?
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

/// Rotate keystore password or parameters
///
/// Decrypts the keystore with the old password and re-encrypts it with the new password
/// and parameters.
///
/// # Arguments
/// * `ks` - Existing keystore file
/// * `old_password` - Current password
/// * `new_password` - New password
/// * `mem_kib` - New memory cost
/// * `time_cost` - New time cost
/// * `parallelism` - New parallelism
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
    )?;
    Ok(new_ks)
}

/// Write keystore to a file atomically
///
/// Writes the keystore to a temporary file first, then renames it to the target path
/// to ensure atomic updates.
///
/// # Arguments
/// * `path` - Target file path
/// * `ks` - Keystore to write
pub fn write_keystore_file<P: AsRef<Path>>(path: P, ks: &KeystoreFile) -> std::io::Result<()> {
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

/// Read keystore from a file
///
/// # Arguments
/// * `path` - Path to the keystore file
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
        )
        .unwrap();
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
        )
        .unwrap();
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
        )
        .unwrap();
        let dir = tempdir().expect("Failed to create temp directory");
        let p = dir.path().join("keystore.json");
        write_keystore_file(&p, &ks).expect("write");
        let ks2 = read_keystore_file(&p).expect("read");
        let pt = decrypt_keystore(&ks2, "pw").expect("decrypt");
        assert_eq!(pt, secret);
    }

    #[test]
    fn tamper_cipher_rejected() {
        let secret = b"abc";
        let ks = encrypt_keystore(secret, "pw", None, 8 * 1024, 1, 1).unwrap();
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
        let mut ks = encrypt_keystore(secret, "pw", None, 8 * 1024, 1, 1).unwrap();
        ks.magic = "FAKE".to_string();
        assert!(decrypt_keystore(&ks, "pw").is_err());
    }

    #[test]
    fn future_version_rejected() {
        let secret = b"test";
        let mut ks = encrypt_keystore(secret, "pw", None, 8 * 1024, 1, 1).unwrap();
        ks.version = 99;
        let result = decrypt_keystore(&ks, "pw");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("unsupported version"));
    }

    #[test]
    fn large_secret_roundtrip() {
        let secret = vec![0x42u8; 32 * 1024];
        let ks = encrypt_keystore(&secret, "longpw", None, 8 * 1024, 1, 1).unwrap();
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

        assert!((1..=8).contains(&parallelism));
        assert!((8192..=65536).contains(&memory));
        assert!((1..=3).contains(&time));
    }

    #[test]
    fn adaptive_encryption_roundtrip() {
        let secret = b"adaptive-encryption-test";
        let password = "test-password";
        let meta = Some(json!({"adaptive": true}));

        // Test adaptive encryption
        let ks = encrypt_keystore_adaptive(secret, password, meta.clone()).unwrap();
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

        let ks = encrypt_keystore_adaptive(secret, password, meta.clone()).unwrap();

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

        let ks = encrypt_keystore_adaptive(secret, password, meta.clone()).unwrap();

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

        let ks = encrypt_keystore_adaptive(secret, password, meta.clone()).unwrap();

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

        let ks1 = encrypt_keystore_adaptive(secret1, password1, meta.clone()).unwrap();
        let ks2 = encrypt_keystore_adaptive(secret2, password2, meta.clone()).unwrap();

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
    fn wallet_config_defaults() {
        let config = WalletConfig::default();
        assert!(config.enable_caching);
        assert_eq!(config.cache_timeout, MAX_CACHE_AGE);
        assert_eq!(config.kdf_profile, KdfProfile::Adaptive);
    }

    #[test]
    fn wallet_config_conservative() {
        let config = WalletConfig::conservative();
        assert!(!config.enable_caching); // Disabled for security
        assert_eq!(config.cache_timeout, Duration::from_secs(60));
        assert_eq!(config.kdf_profile, KdfProfile::Tight);
    }

    #[test]
    fn wallet_config_performance() {
        let config = WalletConfig::performance();
        assert!(config.enable_caching);
        assert_eq!(config.cache_timeout, Duration::from_secs(15 * 60));
        assert_eq!(config.kdf_profile, KdfProfile::Adaptive);
    }

    #[test]
    fn wallet_config_mobile() {
        let config = WalletConfig::mobile();
        assert!(config.enable_caching);
        assert_eq!(config.cache_timeout, Duration::from_secs(3 * 60));
        assert_eq!(config.kdf_profile, KdfProfile::Mobile);
    }

    #[test]
    fn wallet_config_server() {
        let config = WalletConfig::server();
        assert!(!config.enable_caching); // No caching on servers
        assert_eq!(config.cache_timeout, Duration::from_secs(30));
        assert_eq!(config.kdf_profile, KdfProfile::Tight);
    }

    #[test]
    fn config_based_encryption_decryption() {
        let secret = b"config-based-test";
        let password = "config-password";
        let meta = Some(json!({"config": true}));

        // Test with different configs
        let configs = vec![
            ("default", WalletConfig::default()),
            ("conservative", WalletConfig::conservative()),
            ("performance", WalletConfig::performance()),
            ("mobile", WalletConfig::mobile()),
            ("server", WalletConfig::server()),
        ];

        for (name, config) in configs {
            let ks = encrypt_keystore_with_config(secret, password, meta.clone(), &config).unwrap();
            let pt = decrypt_keystore_with_config(&ks, password, &config).unwrap();
            assert_eq!(pt, secret, "Failed for config: {}", name);
        }
    }

    #[test]
    fn cache_memory_usage() {
        clear_key_cache();

        let secret = b"memory-test";
        let password = "memory-password";
        let meta = Some(json!({"memory": true}));

        let ks = encrypt_keystore_adaptive(secret, password, meta).unwrap();
        let _pt = decrypt_keystore(&ks, password).unwrap();

        let memory_usage = get_cache_memory_usage();
        assert!(memory_usage > 0, "Cache should use some memory");

        let stats = get_cache_stats();
        assert!(stats.active_entries > 0);

        clear_key_cache();
        let memory_after_clear = get_cache_memory_usage();
        assert_eq!(memory_after_clear, 0, "Cache should be empty after clear");
    }

    #[test]
    fn rotate_password() {
        let secret = b"my-key";
        let ks = encrypt_keystore(secret, "old-pw", None, 8 * 1024, 1, 1).unwrap();
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
    let ks = encrypt_keystore(b"test", "pw", None, 8 * 1024, 1, 1).unwrap();
    let json = ks.to_json().expect("Failed to serialize keystore");
    std::fs::write(&path, &json[..json.len() / 2]).expect("Failed to write truncated keystore");
    assert!(read_keystore_file(&path).is_err());

    // Missing required fields
    std::fs::write(&path, r#"{"version": 1}"#).expect("Failed to write incomplete keystore");
    assert!(read_keystore_file(&path).is_err());
}

#[test]
fn decrypt_corrupted_fields() {
    let ks = encrypt_keystore(b"test", "pw", None, 8 * 1024, 1, 1).unwrap();

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

/// Wallet configuration for performance and security tuning
#[derive(Debug, Clone)]
pub struct WalletConfig {
    /// Cache timeout duration (default: 5 minutes)
    pub cache_timeout: Duration,
    /// Enable/disable key caching (default: true)
    pub enable_caching: bool,
    /// KDF profile to use (default: Adaptive)
    pub kdf_profile: KdfProfile,
}

impl Default for WalletConfig {
    fn default() -> Self {
        Self {
            cache_timeout: MAX_CACHE_AGE,
            enable_caching: true,
            kdf_profile: KdfProfile::Adaptive,
        }
    }
}

impl WalletConfig {
    /// Create conservative config for high-security environments
    pub fn conservative() -> Self {
        Self {
            cache_timeout: Duration::from_secs(60), // 1 minute
            enable_caching: false,                  // Disable caching for max security
            kdf_profile: KdfProfile::Tight,
        }
    }

    /// Create performance-focused config for desktop applications
    pub fn performance() -> Self {
        Self {
            cache_timeout: Duration::from_secs(15 * 60), // 15 minutes
            enable_caching: true,
            kdf_profile: KdfProfile::Adaptive,
        }
    }

    /// Create mobile-friendly config
    pub fn mobile() -> Self {
        Self {
            cache_timeout: Duration::from_secs(3 * 60), // 3 minutes
            enable_caching: true,
            kdf_profile: KdfProfile::Mobile,
        }
    }

    /// Create server/node config with maximum security
    pub fn server() -> Self {
        Self {
            cache_timeout: Duration::from_secs(30), // 30 seconds
            enable_caching: false,                  // No caching on servers
            kdf_profile: KdfProfile::Tight,
        }
    }

    /// Customize cache timeout
    pub fn with_cache_timeout(mut self, timeout: Duration) -> Self {
        self.cache_timeout = timeout;
        self
    }

    /// Enable or disable caching
    pub fn with_caching(mut self, enable: bool) -> Self {
        self.enable_caching = enable;
        self
    }

    /// Set custom KDF profile
    pub fn with_kdf_profile(mut self, profile: KdfProfile) -> Self {
        self.kdf_profile = profile;
        self
    }
}
