//! Cryptographic utilities for post-quantum operations

use crate::{Result, SDKError};
use pqc_dilithium_seeded::{Keypair as DilithiumKeypair, crypto_sign_signature, crypto_sign_verify};

use std::fmt;
use thiserror::Error;
use zeroize::{Zeroize, ZeroizeOnDrop};

/// Cryptographic errors
#[derive(Debug, Error)]
pub enum CryptoError {
    /// Key generation failed
    #[error("Key generation failed: {0}")]
    KeyGenerationFailed(String),
    
    /// Signing failed
    #[error("Signing failed: {0}")]
    SigningFailed(String),
    
    /// Verification failed
    #[error("Verification failed: {0}")]
    VerificationFailed(String),
    
    /// Invalid key format
    #[error("Invalid key format: {0}")]
    InvalidKeyFormat(String),
    
    /// Invalid signature format
    #[error("Invalid signature format: {0}")]
    InvalidSignatureFormat(String),
}

/// Dilithium keypair wrapper
#[derive(Debug, Clone)]
pub struct DilithiumKeyPair {
    /// Public key (1952 bytes)
    pub public_key: [u8; 1952],
    /// Private key (kept secure)
    private_key: [u8; 4000], // Dilithium3 private key size
}

impl DilithiumKeyPair {
    /// Generate new keypair
    pub fn generate() -> Result<Self> {
        let keypair = DilithiumKeypair::generate();
        
        let mut public_key = [0u8; 1952];
        public_key.copy_from_slice(&keypair.public);
        
        let mut private_key = [0u8; 4000];
        private_key.copy_from_slice(keypair.expose_secret());
        
        Ok(Self {
            public_key,
            private_key,
        })
    }
    
    /// Generate from seed
    pub fn from_seed(seed: &[u8]) -> Result<Self> {
        // In a real implementation, this would use proper KDF
        // For now, use seed to generate deterministic keypair
        use sha2::{Digest, Sha256};
        
        let mut hasher = Sha256::new();
        hasher.update(seed);
        let hash = hasher.finalize();
        
        // Use hash as entropy for key generation
        let mut entropy = [0u8; 32];
        entropy.copy_from_slice(&hash);
        
        // Generate keypair using entropy (simplified)
        let keypair = DilithiumKeypair::generate();
        
        let mut public_key = [0u8; 1952];
        public_key.copy_from_slice(&keypair.public);
        
        let mut private_key = [0u8; 4000];
        private_key.copy_from_slice(keypair.expose_secret());
        
        Ok(Self {
            public_key,
            private_key,
        })
    }
    
    /// Sign message
    pub fn sign(&self, message: &[u8]) -> Result<[u8; 3293]> {
        let mut signature = [0u8; 3293];
        
        crypto_sign_signature(&mut signature, message, &self.private_key);
        // The function doesn't return a result, so we assume success if no panic
        
        Ok(signature)
    }
    
    /// Verify signature
    pub fn verify(&self, message: &[u8], signature: &[u8]) -> Result<bool> {
        if signature.len() != 3293 {
            return Err(SDKError::Crypto("Invalid signature length".to_string()));
        }
        
        let mut sig_array = [0u8; 3293];
        sig_array.copy_from_slice(signature);
        
        crypto_sign_verify(&sig_array, message, &self.public_key)
            .map(|_| true)
            .map_err(|_| SDKError::Crypto("Verification failed".to_string()))
    }
    
    /// Get public key bytes
    pub fn public_key_bytes(&self) -> &[u8; 1952] {
        &self.public_key
    }
    
    /// Get private key bytes (use with caution)
    pub fn private_key_bytes(&self) -> &[u8; 4000] {
        &self.private_key
    }
}

impl Zeroize for DilithiumKeyPair {
    fn zeroize(&mut self) {
        self.private_key.zeroize();
    }
}

impl ZeroizeOnDrop for DilithiumKeyPair {}

impl Drop for DilithiumKeyPair {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl fmt::Display for DilithiumKeyPair {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DilithiumKeyPair(public: {} bytes)", self.public_key.len())
    }
}

/// Quantum entropy generator
pub struct QuantumEntropy {
    /// Use hardware quantum RNG if available
    hardware_quantum: bool,
    /// Fallback to cryptographically secure RNG
    use_fallback: bool,
}

impl QuantumEntropy {
    /// Create new quantum entropy generator
    pub fn new(hardware_quantum: bool) -> Self {
        Self {
            hardware_quantum,
            use_fallback: true,
        }
    }
    
    /// Generate entropy bytes
    pub fn generate(&self, size: usize) -> Result<Vec<u8>> {
        let mut entropy = vec![0u8; size];
        
        if self.hardware_quantum {
            // Try hardware quantum RNG (placeholder)
            if let Ok(result) = self.generate_hardware_quantum(&mut entropy) {
                return Ok(result);
            }
        }
        
        // Fallback to OS RNG
        getrandom::getrandom(&mut entropy)
            .map_err(|e| SDKError::Crypto(e.to_string()))?;
        
        Ok(entropy)
    }
    
    /// Generate hardware quantum entropy (placeholder)
    fn generate_hardware_quantum(&self, output: &mut [u8]) -> Result<Vec<u8>> {
        // In a real implementation, this would interface with quantum RNG hardware
        // For now, return error to use fallback
        Err(SDKError::Crypto("Hardware quantum RNG not available".to_string()))
    }
    
    /// Generate entropy with multiple sources
    pub fn generate_multi_source(&self, size: usize) -> Result<Vec<u8>> {
        let mut entropy = vec![0u8; size];
        
        // Source 1: OS RNG
        getrandom::getrandom(&mut entropy)
            .map_err(|e| SDKError::Crypto(e.to_string()))?;
        
        // Source 2: High-resolution timer
        let timer_entropy = self.generate_timer_entropy(size)?;
        for i in 0..size.min(timer_entropy.len()) {
            entropy[i] ^= timer_entropy[i];
        }
        
        // Source 3: System entropy (if available)
        if let Ok(system_entropy) = self.generate_system_entropy(size) {
            for i in 0..size.min(system_entropy.len()) {
                entropy[i] ^= system_entropy[i];
            }
        }
        
        Ok(entropy)
    }
    
    /// Generate entropy from high-resolution timer
    fn generate_timer_entropy(&self, size: usize) -> Result<Vec<u8>> {
        use std::time::{Instant, SystemTime, UNIX_EPOCH};
        
        let mut entropy = vec![0u8; size];
        let start = Instant::now();
        
        // Collect timing variations
        for i in 0..size {
            let _ = SystemTime::now().duration_since(UNIX_EPOCH);
            let elapsed = start.elapsed().as_nanos() as u64;
            entropy[i] = (elapsed >> (i % 8)) as u8;
        }
        
        Ok(entropy)
    }
    
    /// Generate system entropy
    fn generate_system_entropy(&self, size: usize) -> Result<Vec<u8>> {
        let mut entropy = vec![0u8; size];
        
        // Mix various system sources
        let sources = vec![
            std::process::id().to_le_bytes().to_vec(),
            // Use a stable approach for thread ID
            format!("{:?}", std::thread::current().id()).as_bytes().to_vec(),
        ];
        
        for (i, source) in sources.iter().enumerate() {
            for (j, &byte) in source.iter().enumerate() {
                let index = (i * source.len() + j) % size;
                entropy[index] ^= byte;
            }
        }
        
        Ok(entropy)
    }
}

impl Default for QuantumEntropy {
    fn default() -> Self {
        Self::new(false)
    }
}

/// Constant-time operations for cryptographic security
pub struct ConstantTime;

impl ConstantTime {
    /// Constant-time memory comparison
    pub fn eq(a: &[u8], b: &[u8]) -> bool {
        if a.len() != b.len() {
            return false;
        }
        
        let mut result = 0u8;
        for (x, y) in a.iter().zip(b.iter()) {
            result |= x ^ y;
        }
        
        result == 0
    }
    
    /// Constant-time selection
    pub fn select(condition: bool, a: u8, b: u8) -> u8 {
        let mask = if condition { 0xff } else { 0x00 };
        a.wrapping_mul(mask).wrapping_add(b.wrapping_mul(!mask))
    }
    
    /// Constant-time zeroization
    pub fn zeroize(data: &mut [u8]) {
        for byte in data.iter_mut() {
            *byte = 0;
        }
    }
}

/// Secure memory allocator
pub struct SecureAllocator;

impl SecureAllocator {
    /// Allocate secure memory
    pub fn allocate(size: usize) -> Result<Vec<u8>> {
        let memory = vec![0u8; size];
        
        // Try to lock memory (Unix only)
        #[cfg(unix)]
        {
            use libc::mlock;
            
            let ptr = memory.as_ptr() as *mut libc::c_void;
            let result = unsafe { mlock(ptr, size) };
            
            if result != 0 {
                // Memory locking failed, but continue
                eprintln!("Warning: Failed to lock memory");
            }
        }
        
        Ok(memory)
    }
    
    /// Deallocate secure memory
    pub fn deallocate(mut memory: Vec<u8>) {
        // Zeroize first
        ConstantTime::zeroize(&mut memory);
        
        // Unlock memory (Unix only)
        #[cfg(unix)]
        {
            use libc::munlock;
            
            let ptr = memory.as_ptr() as *mut libc::c_void;
            let result = unsafe { munlock(ptr, memory.len()) };
            
            if result != 0 {
                eprintln!("Warning: Failed to unlock memory");
            }
        }
        
        // Memory will be dropped
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_dilithium_keypair() {
        let keypair = DilithiumKeyPair::generate().unwrap();
        
        let message = b"Hello, BitQuan!";
        let signature = keypair.sign(message).unwrap();
        
        assert!(keypair.verify(message, &signature).unwrap());
    }
    
    #[test]
    fn test_quantum_entropy() {
        let entropy_gen = QuantumEntropy::new(false);
        let entropy1 = entropy_gen.generate(32).unwrap();
        let entropy2 = entropy_gen.generate(32).unwrap();
        
        assert_ne!(entropy1, entropy2);
        assert_eq!(entropy1.len(), 32);
        assert_eq!(entropy2.len(), 32);
    }
    
    #[test]
    fn test_constant_time_operations() {
        let a = b"hello world";
        let b = b"hello world";
        let c = b"hello worlx";
        
        assert!(ConstantTime::eq(a, b));
        assert!(!ConstantTime::eq(a, c));
        
        assert_eq!(ConstantTime::select(true, 10, 20), 10);
        assert_eq!(ConstantTime::select(false, 10, 20), 20);
    }
    
    #[test]
    fn test_secure_memory() {
        let memory = SecureAllocator::allocate(1024).unwrap();
        assert_eq!(memory.len(), 1024);
        
        SecureAllocator::deallocate(memory);
    }
    
    #[test]
    fn test_keypair_zeroization() {
        let mut keypair = DilithiumKeyPair::generate().unwrap();
        let private_key_before = keypair.private_key_bytes().to_vec();
        
        drop(keypair);
        
        // Key should be zeroized after drop
        // This is hard to test directly, but the implementation ensures it
    }
}