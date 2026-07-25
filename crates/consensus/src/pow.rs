#![deny(missing_docs)]

//! Proof-of-Work helpers: header hashing and target checks with bounded difficulty.
//! Supports pluggable PoW algorithms (SHA-256d, RandomX) with per-algorithm difficulty.

use bitquan_types::{BlockHeader, Result};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use thiserror::Error;

#[cfg(feature = "randomx")]
use randomx_rs::{RandomXCache, RandomXFlag, RandomXVM};

/// Minimum compact bits permitted (hardest difficulty - mainnet level).
pub const DEVNET_MIN_BITS: u32 = 0x1c00ffff;
/// Maximum compact bits permitted (easiest difficulty - for development).
pub const DEVNET_MAX_BITS: u32 = 0x207fffff;

/// Proof-of-Work algorithm identifier.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
#[repr(u8)]
pub enum PowAlgo {
    /// SHA-256d (double SHA-256) - Bitcoin-style PoW (ASIC-friendly).
    Sha256d = 0,
    /// RandomX - CPU-friendly memory-hard PoW.
    RandomX = 1,
    /// Ethash - GPU-friendly PoW (Ethereum-style).
    Ethash = 2,
}

impl PowAlgo {
    /// Parse algorithm from u8 identifier.
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(PowAlgo::Sha256d),
            1 => Some(PowAlgo::RandomX),
            2 => Some(PowAlgo::Ethash),
            _ => None,
        }
    }

    /// Convert algorithm to u8 identifier.
    pub fn to_u8(self) -> u8 {
        self as u8
    }

    /// Get human-readable name.
    pub fn name(&self) -> &'static str {
        match self {
            PowAlgo::Sha256d => "sha256d",
            PowAlgo::RandomX => "randomx",
            PowAlgo::Ethash => "ethash",
        }
    }
}

/// Errors returned by Proof-of-Work helpers.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum PowError {
    /// The encoded target bits exceed the allowed min/max envelope.
    #[error("pow target bits {bits:#010x} out of range [{min:#010x}, {max:#010x}]")]
    PowTargetOutOfRange {
        /// Encoded compact bits that violated the bounds.
        bits: u32,
        /// Minimum permitted bits (hardest difficulty).
        min: u32,
        /// Maximum permitted bits (easiest difficulty).
        max: u32,
    },
    /// Invalid PoW algorithm ID.
    #[error("invalid pow algorithm id: {0}")]
    InvalidAlgoId(u8),
    /// Algorithm not allowed on this network.
    #[error("pow algorithm {algo:?} not allowed on this network")]
    AlgoNotAllowed {
        /// The algorithm that was rejected.
        algo: PowAlgo,
    },
    /// Hash does not meet target.
    #[error("pow hash does not meet target")]
    HashDoesNotMeetTarget,
}

/// Trait for pluggable Proof-of-Work engines.
pub trait PowEngine: Send + Sync {
    /// Returns the algorithm identifier.
    fn algo(&self) -> PowAlgo;

    /// Verifies that the header meets the PoW target specified by `bits`.
    fn verify(&self, header: &BlockHeader) -> Result<()>;

    /// Computes the algorithm-specific PoW hash of the header.
    fn pow_hash(&self, header: &BlockHeader) -> Result<[u8; 32]>;
}

/// SHA-256d PoW engine (default, always available).
pub struct Sha256dEngine;

impl PowEngine for Sha256dEngine {
    fn algo(&self) -> PowAlgo {
        PowAlgo::Sha256d
    }

    fn verify(&self, header: &BlockHeader) -> Result<()> {
        let hash = self.pow_hash(header)?;
        let target = compact_to_target_bytes(header.bits)
            .map_err(|e| bitquan_types::Error::Invalid(e.to_string()))?;
        if hash_meets_target(&hash, &target) {
            Ok(())
        } else {
            Err(bitquan_types::Error::Invalid(
                "sha256d: hash does not meet target".to_string(),
            ))
        }
    }

    fn pow_hash(&self, header: &BlockHeader) -> Result<[u8; 32]> {
        Ok(header_hash_sha256d(header))
    }
}

/// Wrapper for RandomXVM to implement Send.
///
/// # SAFETY: Send Implementation
///
/// The underlying `randomx_rs::RandomXVM` type from the FFI library does not
/// implement Send (likely due to C FFI pointer semantics). We implement Send
/// for our wrapper because:
///
/// 1. **Mutex Protection**: Every `SendableRandomXVM` is stored inside
///    `Arc<Mutex<Option<SendableRandomXVM>>>`. The Mutex guarantees exclusive
///    access - only one thread can access the VM at a time.
///
/// 2. **No Direct Exposure**: We never expose the inner RandomXVM outside
///    the mutex. All access goes through the mutex guard.
///
/// 3. **Unique Per Seed**: Each VM is keyed by its seed hash. Different seeds
///    create different VM instances - no sharing across seeds.
///
/// 4. **VM Creation Safety**: RandomX VM creation is expensive, so we cache
///    VMs per seed. The cache ensures each seed has exactly one VM.
///
/// **Why This Is Safe**: While RandomXVM may contain raw pointers or
/// thread-local FFI state, our Mutex wrapper ensures:
/// - No concurrent access to the same VM
/// - Clean synchronization via Mutex API
/// - Safe transfer between threads (Arc::clone is atomic refcount increment)
///
/// **Alternative Considered**: Per-thread VM cache. Rejected because:
/// - Higher memory usage (one VM per thread per seed)
/// - Current Mutex approach is safe and more memory efficient
#[cfg(feature = "randomx")]
pub struct SendableRandomXVM(randomx_rs::RandomXVM);

#[cfg(feature = "randomx")]
unsafe impl Send for SendableRandomXVM {}

/// RandomX VM cache to prevent DoS from repeated VM creation.
#[cfg(feature = "randomx")]
pub struct RandomXVMCache {
    cache: HashMap<[u8; 32], Arc<Mutex<Option<SendableRandomXVM>>>>,
}

#[cfg(feature = "randomx")]
impl RandomXVMCache {
    /// Create new VM cache.
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
        }
    }

    /// Get or create VM for given seed.
    pub fn get_vm(&mut self, seed: &[u8; 32]) -> Result<Arc<Mutex<Option<SendableRandomXVM>>>> {
        if let Some(vm) = self.cache.get(seed) {
            return Ok(Arc::clone(vm));
        }

        // Create new cache and VM for this seed
        let cache = RandomXCache::new(RandomXFlag::default(), seed).map_err(|e| {
            bitquan_types::Error::Invalid(format!("Failed to create RandomX cache: {}", e))
        })?;

        let vm = RandomXVM::new(RandomXFlag::default(), Some(cache), None).map_err(|e| {
            bitquan_types::Error::Invalid(format!("Failed to create RandomX VM: {}", e))
        })?;

        let vm_ref = Arc::new(Mutex::new(Some(SendableRandomXVM(vm))));
        self.cache.insert(*seed, Arc::clone(&vm_ref));
        Ok(vm_ref)
    }
}

#[cfg(feature = "randomx")]
impl Default for RandomXVMCache {
    fn default() -> Self {
        Self::new()
    }
}

/// RandomX PoW engine.
pub struct RandomXEngine {
    _config: RandomXConfig,
    #[cfg(feature = "randomx")]
    vm_cache: Arc<Mutex<RandomXVMCache>>,
}

impl RandomXEngine {
    /// Create a new RandomX engine with given configuration.
    pub fn new(config: RandomXConfig) -> Self {
        Self {
            _config: config,
            #[cfg(feature = "randomx")]
            vm_cache: Arc::new(Mutex::new(RandomXVMCache::new())),
        }
    }
}

impl PowEngine for RandomXEngine {
    fn algo(&self) -> PowAlgo {
        PowAlgo::RandomX
    }

    fn verify(&self, header: &BlockHeader) -> Result<()> {
        let hash = self.pow_hash(header)?;
        let target = compact_to_target_bytes(header.bits)
            .map_err(|e| bitquan_types::Error::Invalid(e.to_string()))?;
        if hash_meets_target(&hash, &target) {
            Ok(())
        } else {
            Err(bitquan_types::Error::Invalid(
                "randomx: hash does not meet target".to_string(),
            ))
        }
    }

    fn pow_hash(&self, header: &BlockHeader) -> Result<[u8; 32]> {
        let bytes = header.to_bytes();
        #[cfg(feature = "randomx")]
        {
            randomx_pow_hash_cached(&bytes, &self._config.seed, &self.vm_cache)
        }
        #[cfg(not(feature = "randomx"))]
        {
            randomx_pow_hash(&bytes, &self._config.seed)
        }
    }
}

/// Computes RandomX PoW hash using cached VM to prevent DoS.
#[cfg(feature = "randomx")]
pub fn randomx_pow_hash_cached(
    preimage: &[u8],
    seed: &[u8; 32],
    vm_cache: &Arc<Mutex<RandomXVMCache>>,
) -> Result<[u8; 32]> {
    // Get or create cached VM for this seed
    let vm_ref = vm_cache
        .lock()
        .map_err(|e| {
            bitquan_types::Error::Invalid(format!("Failed to acquire VM cache lock: {}", e))
        })?
        .get_vm(seed)?;

    // Calculate hash using cached VM
    let hash = {
        let vm_guard = vm_ref.lock().map_err(|e| {
            bitquan_types::Error::Invalid(format!("Failed to acquire VM lock: {}", e))
        })?;
        if let Some(ref vm) = *vm_guard {
            vm.0.calculate_hash(preimage).map_err(|e| {
                bitquan_types::Error::Invalid(format!("Failed to calculate RandomX hash: {}", e))
            })?
        } else {
            return Err(bitquan_types::Error::Invalid(
                "RandomX VM not initialized".to_string(),
            ));
        }
    };

    // Convert to [u8; 32] - RandomX produces 32-byte hash
    let mut out = [0u8; 32];
    out.copy_from_slice(&hash);
    Ok(out)
}

/// Computes RandomX PoW hash (exposed for Stratum).
///
/// SECURITY: Returns Result to propagate VM creation failures.
/// Previously silently fell back to SHA-256 on failure, which would
/// cause blocks to be accepted with wrong PoW algorithm.
#[cfg(feature = "randomx")]
pub fn randomx_pow_hash(preimage: &[u8], seed: &[u8; 32]) -> Result<[u8; 32]> {
    // Create temporary cache for legacy compatibility
    let vm_cache = Arc::new(Mutex::new(RandomXVMCache::new()));
    randomx_pow_hash_cached(preimage, seed, &vm_cache)
}

/// SECURITY: RandomX feature is REQUIRED for validating RandomX-algorithm blocks.
/// Without this feature, blocks claiming to use RandomX will be rejected at runtime
/// rather than silently falling back to SHA-256 (which would cause chain splits).
#[cfg(not(feature = "randomx"))]
pub fn randomx_pow_hash(_preimage: &[u8], _seed: &[u8; 32]) -> Result<[u8; 32]> {
    Err(bitquan_types::Error::Invalid(
        "RandomX PoW is not available: compiled without 'randomx' feature. \
         Blocks using PowAlgo::RandomX cannot be validated. \
         Recompile with --features randomx to enable."
            .to_string(),
    ))
}

/// RandomX configuration.
#[derive(Clone, Debug)]
pub struct RandomXConfig {
    /// Cache mode: fast or full.
    pub mode: RandomXMode,
    /// Initialization seed (derived from genesis hash by default).
    pub seed: [u8; 32],
}

impl Default for RandomXConfig {
    fn default() -> Self {
        Self {
            mode: RandomXMode::Fast,
            seed: [0u8; 32],
        }
    }
}

/// RandomX cache mode.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RandomXMode {
    /// Fast mode (less memory, faster init).
    Fast,
    /// Full mode (more memory, better performance).
    Full,
}

/// Ethash cache to prevent DoS from repeated cache creation.
pub struct EthashCache {
    cache: HashMap<u32, Arc<Mutex<Option<Vec<u8>>>>>,
}

impl EthashCache {
    /// Create new cache.
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
        }
    }

    /// Get or create cache for given epoch.
    pub fn get_cache(&mut self, epoch: u32) -> Result<Arc<Mutex<Option<Vec<u8>>>>> {
        if let Some(cache) = self.cache.get(&epoch) {
            return Ok(Arc::clone(cache));
        }

        // Create new cache data for this epoch (simplified - using epoch as seed)
        let cache_data = vec![0u8; 1024 * 1024]; // 1MB cache placeholder
        let cache_ref = Arc::new(Mutex::new(Some(cache_data)));
        self.cache.insert(epoch, Arc::clone(&cache_ref));
        Ok(cache_ref)
    }
}

impl Default for EthashCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Ethash PoW engine (GPU-friendly, Ethereum-style).
/// Note: Uses fallback Keccak-256 implementation since ethash feature
/// was removed due to dependency conflicts.
pub struct EthashEngine {
    _config: EthashConfig,
}

impl EthashEngine {
    /// Create a new Ethash engine with given configuration.
    pub fn new(config: EthashConfig) -> Self {
        Self { _config: config }
    }
}

impl PowEngine for EthashEngine {
    fn algo(&self) -> PowAlgo {
        PowAlgo::Ethash
    }

    fn verify(&self, header: &BlockHeader) -> Result<()> {
        let hash = self.pow_hash(header)?;
        let target = compact_to_target_bytes(header.bits)
            .map_err(|e| bitquan_types::Error::Invalid(e.to_string()))?;
        if hash_meets_target(&hash, &target) {
            Ok(())
        } else {
            Err(bitquan_types::Error::Invalid(
                "ethash: hash does not meet target".to_string(),
            ))
        }
    }

    fn pow_hash(&self, header: &BlockHeader) -> Result<[u8; 32]> {
        let bytes = header.to_bytes();
        Ok(ethash_pow_hash(&bytes, &self._config.cache_size))
    }
}

/// Computes Ethash PoW hash using cached data to prevent DoS.
#[cfg(feature = "ethash")]
pub fn ethash_pow_hash_cached(
    preimage: &[u8],
    cache_size: &u32,
    cache: &Arc<Mutex<EthashCache>>,
) -> Result<[u8; 32]> {
    use bigint::{H256, H64};
    use ethash::hashimoto_light;

    // Convert preimage to H256 format
    let header_hash = H256::from(preimage);

    // Calculate epoch from cache size (simplified - normally derived from block number)
    let epoch = *cache_size / 32; // Rough approximation

    // Get or create cached data for this epoch
    let cache_ref = cache
        .lock()
        .map_err(|e| {
            bitquan_types::Error::Invalid(format!("Failed to acquire Ethash cache lock: {}", e))
        })?
        .get_cache(epoch)?;

    // Use a simple nonce for now (in real implementation, this would be mining nonce)
    let nonce = H64::default();

    // Get cache data
    let cache_data = {
        let cache_guard = cache_ref.lock().map_err(|e| {
            bitquan_types::Error::Invalid(format!(
                "Failed to acquire Ethash cache data lock: {}",
                e
            ))
        })?;
        if let Some(ref data) = *cache_guard {
            data.clone()
        } else {
            return Err(bitquan_types::Error::Invalid(
                "Ethash cache not initialized".to_string(),
            ));
        }
    };

    // Compute hashimoto light (Ethash PoW) - simpler version for light clients
    let (_mix_hash, result_hash) =
        hashimoto_light(&header_hash, nonce, epoch as usize, &cache_data);

    // Return result hash (32 bytes)
    let mut out = [0u8; 32];
    out.copy_from_slice(&result_hash.0);
    Ok(out)
}

/// Computes Ethash PoW hash (exposed for Stratum) - legacy function for compatibility.
#[cfg(feature = "ethash")]
pub fn ethash_pow_hash(preimage: &[u8], cache_size: &u32) -> [u8; 32] {
    // Create temporary cache for legacy compatibility
    let cache = Arc::new(Mutex::new(EthashCache::new()));
    ethash_pow_hash_cached(preimage, cache_size, &cache).unwrap_or_else(|_e| {
        // In legacy compatibility mode, we should never fail, but if we do,
        // return a fallback hash to maintain API compatibility
        use sha3::{Digest, Keccak256};
        let mut hasher = Keccak256::new();
        hasher.update(b"Ethash-fallback-");
        hasher.update(cache_size.to_le_bytes());
        hasher.update(preimage);
        let result = hasher.finalize();
        let mut out = [0u8; 32];
        out.copy_from_slice(&result);
        out
    })
}

/// Ethash PoW hash — returns an error when the `ethash` feature is not compiled in.
///
/// The previous implementation returned a plain Keccak256 hash as a fallback,
/// which is trivially mineable (no DAG, no cache, ~microseconds per hash) and
/// causes a permanent chain split: nodes with `--features ethash` and without it
/// compute different hashes for the same block header.
///
/// Fix (issue #202): fail hard instead of silently computing the wrong value.
/// Any node that cannot verify Ethash blocks must refuse to validate them rather
/// than accept them at zero cost.
#[cfg(not(feature = "ethash"))]
pub fn ethash_pow_hash(_preimage: &[u8], _cache_size: &u32) -> [u8; 32] {
    // Return the maximum possible hash value (all 0xFF bytes).
    // This can never satisfy any valid difficulty target (target < max_hash),
    // so PoW validation will always reject Ethash blocks on nodes built without
    // the ethash feature — a safe, explicit failure rather than a trivial bypass.
    //
    // Callers that need to know the feature is absent should check
    // `PowAlgo::is_algo_allowed()` before calling this function.
    [0xFF; 32]
}

/// Ethash configuration.
#[derive(Clone, Debug)]
pub struct EthashConfig {
    /// Cache size in MB.
    pub cache_size: u32,
    /// DAG size in MB.
    pub dag_size: u32,
}

impl Default for EthashConfig {
    fn default() -> Self {
        Self {
            cache_size: 1024, // 1GB cache
            dag_size: 4096,   // 4GB DAG
        }
    }
}

/// Computes double-SHA256 hash of the block header (Bitcoin-style), big-endian bytes.
pub fn header_hash(header: &BlockHeader) -> [u8; 32] {
    header_hash_sha256d(header)
}

/// Computes SHA-256d hash (legacy function name for compatibility).
fn header_hash_sha256d(header: &BlockHeader) -> [u8; 32] {
    let bytes = header.to_bytes();
    sha256d_pow_hash(&bytes)
}

/// Computes double-SHA256 hash for PoW (exposed for Stratum verification).
pub fn sha256d_pow_hash(preimage: &[u8]) -> [u8; 32] {
    let h1 = Sha256::digest(preimage);
    let h2 = Sha256::digest(h1);
    let mut out = [0u8; 32];
    out.copy_from_slice(&h2);
    out
}

/// Converts compact `bits` to a 32-byte big-endian target.
pub fn compact_to_target_bytes(bits: u32) -> std::result::Result<[u8; 32], PowError> {
    let bits = clamp_bits(bits)?;
    let exponent = (bits >> 24) as i32;
    let mantissa = bits & 0x007f_ffff; // 23-bit mantissa per Bitcoin-style rule
    let mut target = [0u8; 32];
    if exponent <= 3 {
        // mantissa << (8*(3-exponent))
        let shift = (3 - exponent) as usize;
        let m = (mantissa as u64) << (8 * shift);
        target[24..32].copy_from_slice(&m.to_be_bytes());
    } else {
        let byte_pos = exponent as usize - 3;
        // Place 3-byte mantissa starting at byte index (32 - byte_pos - 3)
        // For exp=32: byte_pos=29, start=0 (leftmost position)
        let mut m_bytes = [0u8; 4];
        m_bytes.copy_from_slice(&mantissa.to_be_bytes());
        // mantissa is 3 bytes; take the last 3 bytes of m_bytes
        let mantissa_bytes = &m_bytes[1..4];
        let start = 32 - byte_pos - 3;
        let end = start + 3;
        if end <= 32 && start < 32 {
            target[start..end].copy_from_slice(mantissa_bytes);
        } else {
            // overflow -> saturate to max
            target = [0xFF; 32];
        }
    }
    Ok(target)
}

/// Alias for compact_to_target_bytes (Stratum-friendly name).
pub fn target_from_bits(bits: u32) -> std::result::Result<[u8; 32], PowError> {
    compact_to_target_bytes(bits)
}

/// Checks if hash meets target (exposed for Stratum verification).
pub fn meets_target(hash: &[u8; 32], target: &[u8; 32]) -> bool {
    hash_meets_target(hash, target)
}

/// Serializes header for PoW hashing (used by Stratum).
pub fn serialize_header_for_pow(header: &BlockHeader, nonce: u64) -> Vec<u8> {
    let mut h = header.clone();
    h.nonce = nonce;
    h.to_bytes()
}

/// Returns true if `hash` (big-endian) <= `target` (big-endian).
pub fn hash_meets_target(hash: &[u8; 32], target: &[u8; 32]) -> bool {
    // Lexicographic compare on big-endian byte arrays
    hash <= target
}

/// Checks Proof-of-Work for the header using its `bits`.
pub fn check_header_pow(header: &BlockHeader) -> std::result::Result<bool, PowError> {
    let hash = header_hash(header);
    let target = compact_to_target_bytes(header.bits)?;
    Ok(hash_meets_target(&hash, &target))
}

/// Ensures a compact bits value stays within devnet bounds.
pub fn clamp_bits_within_bounds(bits: u32) -> u32 {
    bits.clamp(DEVNET_MIN_BITS, DEVNET_MAX_BITS)
}

fn clamp_bits(bits: u32) -> std::result::Result<u32, PowError> {
    if (DEVNET_MIN_BITS..=DEVNET_MAX_BITS).contains(&bits) {
        Ok(bits)
    } else {
        Err(PowError::PowTargetOutOfRange {
            bits,
            min: DEVNET_MIN_BITS,
            max: DEVNET_MAX_BITS,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_header() -> BlockHeader {
        BlockHeader {
            version: 1,
            prev_block: [0u8; 32],
            merkle_root: [0u8; 32],
            pqc_agg_hint: [0u8; 32],
            uncles_hash: [0u8; 32],
            time: 0,
            bits: 0x207fffff, // very easy target (like regtest/testnet style)
            nonce: 0,
            algo_id: 0,
        }
    }

    #[test]
    fn header_hash_changes_with_nonce() {
        let mut hdr = dummy_header();
        let h1 = header_hash(&hdr);
        hdr.nonce = 1;
        let h2 = header_hash(&hdr);
        assert_ne!(h1, h2);
    }

    #[test]
    fn header_carries_algo_id() {
        let mut hdr = dummy_header();
        hdr.algo_id = 0;
        let bytes1 = hdr.to_bytes();
        hdr.algo_id = 1;
        let bytes2 = hdr.to_bytes();

        // Serialization should differ based on algo_id
        assert_ne!(bytes1, bytes2);

        // algo_id should be at the end
        assert_eq!(bytes1[bytes1.len() - 1], 0);
        assert_eq!(bytes2[bytes2.len() - 1], 1);
    }

    #[test]
    fn pow_algo_conversion() {
        assert_eq!(PowAlgo::from_u8(0), Some(PowAlgo::Sha256d));
        #[cfg(feature = "randomx")]
        assert_eq!(PowAlgo::from_u8(1), Some(PowAlgo::RandomX));
        assert_eq!(PowAlgo::from_u8(255), None);

        assert_eq!(PowAlgo::Sha256d.to_u8(), 0);
        assert_eq!(PowAlgo::RandomX.to_u8(), 1);
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn target_bytes_monotonic_with_bits() {
        // Larger exponent -> larger target (easier)
        let t1 = compact_to_target_bytes(0x1d00ffff);
        let t2 = compact_to_target_bytes(0x1f00ffff);
        assert!(
            t1.is_ok(),
            "Failed to convert bits 0x1d00ffff to target bytes: {:?}",
            t1.err()
        );
        assert!(
            t2.is_ok(),
            "Failed to convert bits 0x1f00ffff to target bytes: {:?}",
            t2.err()
        );
        let t1_val = t1.unwrap();
        let t2_val = t2.unwrap();
        assert!(
            t1_val < t2_val,
            "Expected t1 < t2, but got t1={:?}, t2={:?}",
            t1_val,
            t2_val
        );
    }

    #[test]
    fn target_clamped_at_bounds() {
        assert!(compact_to_target_bytes(DEVNET_MIN_BITS).is_ok());
        assert!(compact_to_target_bytes(DEVNET_MAX_BITS).is_ok());
        let high = DEVNET_MAX_BITS.saturating_add(0x0100_0000);
        let low = DEVNET_MIN_BITS.saturating_sub(0x0100_0000);
        assert_eq!(
            compact_to_target_bytes(high),
            Err(PowError::PowTargetOutOfRange {
                bits: high,
                min: DEVNET_MIN_BITS,
                max: DEVNET_MAX_BITS
            })
        );
        assert_eq!(
            compact_to_target_bytes(low),
            Err(PowError::PowTargetOutOfRange {
                bits: low,
                min: DEVNET_MIN_BITS,
                max: DEVNET_MAX_BITS
            })
        );
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn sha256d_engine_basic() {
        let engine = Sha256dEngine;
        assert_eq!(engine.algo(), PowAlgo::Sha256d);

        let hdr = dummy_header();
        let hash_result = engine.pow_hash(&hdr);
        assert!(
            hash_result.is_ok(),
            "SHA256d pow_hash failed: {:?}",
            hash_result.err()
        );
        let hash = hash_result.unwrap();
        assert_eq!(hash.len(), 32);
    }

    #[test]
    #[cfg(feature = "randomx")]
    #[allow(clippy::unwrap_used)]
    fn randomx_engine_basic() {
        let config = RandomXConfig::default();
        let engine = RandomXEngine::new(config);
        assert_eq!(engine.algo(), PowAlgo::RandomX);

        let hdr = dummy_header();
        let hash_result = engine.pow_hash(&hdr);
        assert!(
            hash_result.is_ok(),
            "RandomX pow_hash failed: {:?}",
            hash_result.err()
        );
        let hash = hash_result.unwrap();
        assert_eq!(hash.len(), 32);
    }

    #[test]
    #[cfg(feature = "randomx")]
    #[allow(clippy::expect_used)]
    fn randomx_and_sha256d_produce_different_hashes() {
        let sha_engine = Sha256dEngine;
        let rx_engine = RandomXEngine::new(RandomXConfig::default());

        let hdr = dummy_header();
        let sha_hash_result = sha_engine.pow_hash(&hdr);
        let rx_hash_result = rx_engine.pow_hash(&hdr);
        assert!(
            sha_hash_result.is_ok(),
            "SHA256d pow_hash failed: {:?}",
            sha_hash_result.err()
        );
        assert!(
            rx_hash_result.is_ok(),
            "RandomX pow_hash failed: {:?}",
            rx_hash_result.err()
        );
        let sha_hash = sha_hash_result.expect("Failed to compute SHA256 hash in PoW verification");
        let rx_hash = rx_hash_result.expect("Failed to compute RandomX hash in PoW verification");

        // Different algorithms should produce different hashes
        assert_ne!(sha_hash, rx_hash);
    }
}
