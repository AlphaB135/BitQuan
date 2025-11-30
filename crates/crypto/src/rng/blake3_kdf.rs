//! BLAKE3 KDF helpers for RNG stream derivation.

use blake3::Hasher;

/// Expands the master seed into a 32-byte sub-seed using BLAKE3 Key Derivation.
///
/// This uses BLAKE3's built-in key derivation mode, which is faster and simpler
/// than HKDF-SHA256 while providing 128-bit security (quantum-safe).
pub fn blake3_expand(master: &[u8; 32], label: &str) -> [u8; 32] {
    // BLAKE3 derive_key mode takes a context string (label) and key material (master seed)
    let mut hasher = Hasher::new_derive_key(label);
    hasher.update(master);
    hasher.finalize().into()
}
