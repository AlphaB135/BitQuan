//! Address encoding utilities for BitQuan wallets.
#![allow(dead_code)]

use bech32::{self, Bech32m, Hrp};

/// Human-readable part for mainnet addresses (current).
const HRP_MAINNET: &str = "bq";
/// Legacy mainnet HRP retained for backward compatibility.
const HRP_MAINNET_LEGACY: &str = "q";
/// Human-readable part for public test networks.
const HRP_TESTNET: &str = "bqt";

/// Known BitQuan address networks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AddressNetwork {
    /// Mainnet network (prefix `bq`).
    Mainnet,
    /// Public testnet network (prefix `bqt`).
    Testnet,
    /// Legacy mainnet format preserved for migration (prefix `q`).
    LegacyMainnet,
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

/// Encodes a public key hash as a Bech32m address using witness version 1.
#[allow(clippy::expect_used)]
pub fn encode(pubkey_hash: &[u8; 32]) -> String {
    // SAFETY: HRP constants are validated at compile-time via const assertion
    let hrp = Hrp::parse(HRP_MAINNET).expect("built-in HRP is valid");
    let mut data = Vec::with_capacity(33);
    data.push(1u8); // witness version
    data.extend_from_slice(pubkey_hash);
    // SAFETY: encoding with valid HRP and valid data cannot fail
    bech32::encode::<Bech32m>(hrp, &data).expect("encoding with valid HRP/data")
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
        HRP_TESTNET => AddressNetwork::Testnet,
        HRP_MAINNET_LEGACY => AddressNetwork::LegacyMainnet,
        other => {
            return Err(format!(
                "unsupported HRP `{other}` (expected `{HRP_MAINNET}` for mainnet or `{HRP_TESTNET}` for testnet)"
            ))
        }
    };

    let payload_slice: &[u8] = match network {
        AddressNetwork::Mainnet | AddressNetwork::Testnet => {
            if data.is_empty() {
                return Err("address missing witness data".to_string());
            }

            let witness_version = data[0];
            if witness_version != 1 {
                return Err(format!(
                    "unsupported witness version {} (expected 1)",
                    witness_version
                ));
            }

            if data.len() != 33 {
                return Err(format!(
                    "invalid payload length: expected 33 (version + hash), got {}",
                    data.len()
                ));
            }
            &data[1..]
        }
        AddressNetwork::LegacyMainnet => {
            if data.len() != 32 {
                return Err(format!(
                    "invalid payload length: expected 32, got {}",
                    data.len()
                ));
            }
            &data
        }
    };

    let mut payload = [0u8; 32];
    payload.copy_from_slice(payload_slice);

    Ok(AddressInfo {
        hrp,
        network,
        normalized,
        payload,
    })
}

/// Decodes a Bech32m address to a public key hash.
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

/// Encode with custom prefix (for multisig addresses)
#[allow(clippy::expect_used)]
pub fn encode_bech32m_with_prefix(pubkey_hash: &[u8], prefix: &str) -> String {
    // SAFETY: Network HRPs are validated at compile-time
    let hrp = Hrp::parse(prefix).expect("network HRP is valid");
    let mut data = Vec::with_capacity(pubkey_hash.len() + 1);
    data.push(1u8); // witness version
    data.extend_from_slice(pubkey_hash);
    // SAFETY: encoding with valid HRP and valid data cannot fail
    bech32::encode::<Bech32m>(hrp, &data).expect("encoding with valid HRP/data")
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode_roundtrip() {
        let hash = [0x42; 32];
        let address = encode(&hash);
        assert!(address.starts_with("bq1"));

        let decoded_result = decode_bech32m(&address);
        assert!(
            decoded_result.is_ok(),
            "Failed to decode bech32m address: {:?}",
            decoded_result.err()
        );
        assert_eq!(decoded_result.expect("Failed to decode address"), hash);
    }

    #[test]
    fn test_inspect_valid_address() {
        let hash = [0x11; 32];
        let address = encode(&hash);
        let info = inspect(&address).expect("address should validate");

        assert_eq!(info.network, AddressNetwork::Mainnet);
        assert_eq!(info.normalized, address);
        assert_eq!(info.payload, hash);
    }

    #[test]
    fn test_inspect_legacy_q_address() {
        let hrp = Hrp::parse(HRP_MAINNET_LEGACY).expect("Failed to parse legacy HRP");
        let hash = [0x44; 32];
        let legacy = bech32::encode::<Bech32m>(hrp, &hash).expect("encode legacy");
        let info = inspect(&legacy).expect("legacy address should validate");

        assert_eq!(info.network, AddressNetwork::LegacyMainnet);
        assert_eq!(info.payload, hash);
    }

    #[test]
    fn test_inspect_rejects_wrong_hrp() {
        let hash = [0x22; 32];
        let wrong_address =
            bech32::encode::<Bech32m>(Hrp::parse("x").expect("Failed to parse HRP"), &hash)
                .expect("encode");
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
