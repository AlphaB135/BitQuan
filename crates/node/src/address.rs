//! Address encoding for BitQuan wallets.

#![allow(dead_code)]

use bech32::{Bech32m, Hrp};

/// Human-readable part for mainnet addresses
const HRP_MAINNET: &str = "q";

/// Encodes a public key hash as a Bech32m address.
pub fn encode_bech32m(pubkey_hash: &[u8; 32]) -> String {
    let hrp = Hrp::parse(HRP_MAINNET).expect("valid HRP");
    bech32::encode::<Bech32m>(hrp, pubkey_hash).expect("valid bech32m encoding")
}

/// Alias for encode_bech32m
#[allow(dead_code)]
pub fn encode(pubkey_hash: &[u8; 32]) -> String {
    encode_bech32m(pubkey_hash)
}

/// Decodes a Bech32m address to a public key hash.
#[allow(dead_code)]
pub fn decode_bech32m(address: &str) -> Result<[u8; 32], String> {
    let (_hrp, data) = bech32::decode(address).map_err(|e| format!("invalid bech32m: {}", e))?;

    // data is already the raw bytes after bech32 decoding
    if data.len() != 32 {
        return Err(format!(
            "invalid data length: expected 32, got {}",
            data.len()
        ));
    }

    let mut bytes = [0u8; 32];
    bytes.copy_from_slice(&data);
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode_roundtrip() {
        let hash = [0x42; 32];
        let address = encode_bech32m(&hash);
        assert!(address.starts_with("q1"));

        let decoded = decode_bech32m(&address).unwrap();
        assert_eq!(decoded, hash);
    }
}
