#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! HKDF-SHA256 helpers for RNG stream derivation.

use hkdf::Hkdf;
use sha2::Sha256;

/// Expands the master seed into a 32-byte sub-seed using HKDF-SHA256.
pub fn hkdf_expand(master: &[u8; 32], label: &str) -> [u8; 32] {
    let hkdf = Hkdf::<Sha256>::new(None, master);
    let mut okm = [0u8; 32];
    hkdf.expand(label.as_bytes(), &mut okm).expect("HKDF");
    okm
}
