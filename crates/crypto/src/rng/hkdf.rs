//! HKDF-SHA256 helpers for RNG stream derivation.

use hkdf::Hkdf;
use sha2::Sha256;

const SALT: &[u8] = b"bitquan.rng.hkdf.v1";

/// Derives a 32-byte sub-seed from the provided key material and label.
pub(crate) fn derive_seed(material: &[u8], label: &str) -> [u8; 32] {
    let hk = Hkdf::<Sha256>::new(Some(SALT), material);
    let mut okm = [0u8; 32];
    hk.expand(label.as_bytes(), &mut okm)
        .expect("HKDF expand for 32 bytes must succeed");
    okm
}
