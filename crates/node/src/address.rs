//! Address encoding utilities for BitQuan wallets.
#![allow(dead_code)]

use bech32::{self, Bech32m, Hrp};

/// Human-readable part for mainnet addresses.
const HRP_MAINNET: &str = "q";

/// Known BitQuan address networks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AddressNetwork {
    /// Mainnet network (prefix `q`).
    Mainnet,
}

/// Decoded information about a BitQuan address.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AddressInfo {
    /// Human-readable part that determined the network.
    pub hrp: Hrp,
    /// Derived network based on the HRP.
    pub network: AddressNetwork,
    /// Lowercase, checksum-validated representation.
    pub normalized: String,
    /// 32-byte public key hash extracted from the payload.
    pub payload: [u8; 32],
}

/// Encodes a public key hash as a Bech32m address.
pub fn encode_bech32m(pubkey_hash: &[u8; 32]) -> String {
    let hrp = Hrp::parse(HRP_MAINNET).expect("valid HRP");
    bech32::encode::<Bech32m>(hrp, pubkey_hash).expect("valid bech32m encoding")
}

/// Alias for `encode_bech32m`.
#[allow(dead_code)]
pub fn encode(pubkey_hash: &[u8; 32]) -> String {
    encode_bech32m(pubkey_hash)
}

/// Returns decoded metadata for a BitQuan Bech32m address.
pub fn inspect(address: &str) -> Result<AddressInfo, String> {
    let trimmed = address.trim();
    if trimmed.is_empty() {
        return Err("address is empty".to_string());
    }

    let (hrp, data) =
        bech32::decode(trimmed).map_err(|e| format!("invalid bech32 encoding: {e}"))?;

    let normalized = bech32::encode::<Bech32m>(hrp, &data)
        .map_err(|e| format!("failed to re-encode address: {e}"))?;

    // Ensure the provided string matches the expected Bech32m checksum (case-insensitive).
    if normalized != trimmed.to_lowercase() {
        return Err("address checksum must use Bech32m encoding".to_string());
    }

    let network = match hrp.as_str() {
        HRP_MAINNET => AddressNetwork::Mainnet,
        other => {
            return Err(format!(
                "unsupported HRP `{other}` (expected `{HRP_MAINNET}` for mainnet)"
            ))
        }
    };

    if data.len() != 32 {
        return Err(format!(
            "invalid data length: expected 32, got {}",
            data.len()
        ));
    }

    let mut payload = [0u8; 32];
    payload.copy_from_slice(&data);

    Ok(AddressInfo {
        hrp,
        network,
        normalized,
        payload,
    })
}

/// Decodes a Bech32m address to a public key hash.
#[allow(dead_code)]
pub fn decode_bech32m(address: &str) -> Result<[u8; 32], String> {
    inspect(address).map(|info| info.payload)
}

/// Builds the standard script_pubkey for a BitQuan address (OP_HASH256 based).
pub fn script_from_pubkey_hash(pubkey_hash: &[u8; 32]) -> Vec<u8> {
    let mut script = Vec::with_capacity(35);
    script.push(0xa8); // OP_HASH256
    script.push(0x20); // Push 32 bytes
    script.extend_from_slice(pubkey_hash);
    script.push(0x87); // OP_EQUAL
    script
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

    #[test]
    fn test_inspect_valid_address() {
        let hash = [0x11; 32];
        let address = encode_bech32m(&hash);
        let info = inspect(&address).expect("address should validate");

        assert_eq!(info.network, AddressNetwork::Mainnet);
        assert_eq!(info.normalized, address);
        assert_eq!(info.payload, hash);
    }

    #[test]
    fn test_inspect_rejects_wrong_hrp() {
        let hash = [0x22; 32];
        let wrong_address =
            bech32::encode::<Bech32m>(Hrp::parse("x").unwrap(), &hash).expect("encode");
        let err = inspect(&wrong_address).unwrap_err();
        assert!(
            err.contains("unsupported HRP"),
            "unexpected error message: {err}"
        );
    }

    #[test]
    fn test_script_from_pubkey_hash_layout() {
        let hash = [0x33; 32];
        let script = script_from_pubkey_hash(&hash);
        assert_eq!(script.len(), 35);
        assert_eq!(script[0], 0xa8);
        assert_eq!(script[1], 0x20);
        assert_eq!(&script[2..34], &hash);
        assert_eq!(script[34], 0x87);
    }
}
