//! CLI helper functions for BitQuan node

use crate::address::AddressNetwork;
use crate::PowMode;
use bitquan_types::error::{Error, Result};
use bitquan_types::NetworkId;

/// Format qbits to BQ (1 BQ = 10^18 qbits)
pub fn format_bq(qbits: u128) -> String {
    let bq = qbits / 1_000_000_000_000_000_000;
    let qats = qbits % 1_000_000_000_000_000_000;
    format!("{}.{:018}", bq, qats)
}

/// Create invalid error
pub fn invalid<T>(msg: impl Into<String>) -> Result<T> {
    Err(bitquan_types::error::Error::Invalid(msg.into()))
}

/// Read password from stdin
pub fn read_password_from_stdin() -> Result<String> {
    // SECURITY: Use rpassword to prompt and hide input (no terminal echo)
    // prompt_password handles flushing stdout automatically
    rpassword::prompt_password("Password: ")
        .map_err(|e| Error::Invalid(format!("Failed to read password: {}", e)))
}

/// Get address network label
pub fn address_network_label(network: AddressNetwork) -> &'static str {
    match network {
        AddressNetwork::Mainnet => "Mainnet",
        AddressNetwork::Testnet => "Testnet",
        AddressNetwork::LegacyMainnet => "LegacyMainnet",
    }
}

/// Parse network ID from string
pub fn parse_network_id(value: &str) -> Result<NetworkId> {
    match value.to_ascii_lowercase().as_str() {
        "mainnet" => Ok(NetworkId::Mainnet),
        "testnet" => Ok(NetworkId::Testnet),
        "devnet" => Ok(NetworkId::Devnet),
        "regtest" => Ok(NetworkId::Regtest),
        other => invalid(format!("unknown network '{}'", other)),
    }
}

/// Ensure PoW mode is allowed for the given network
pub fn ensure_pow_allowed(pow_mode: PowMode, network: NetworkId) -> Result<()> {
    if matches!(pow_mode, PowMode::Mock) && matches!(network, NetworkId::Mainnet) {
        return invalid("mock PoW is disabled on mainnet");
    }
    #[cfg(feature = "randomx")]
    {
        // Allow hybrid mining on mainnet for multi-algorithm support
        if matches!(pow_mode, PowMode::RandomX) && matches!(network, NetworkId::Mainnet) {
            return invalid("RandomX only mode is disabled on mainnet (use hybrid)");
        }
    }
    Ok(())
}

/// Custom parser for u128 values in CLI arguments
/// Clap doesn't have built-in u128 support, so we use string parsing
pub fn parse_u128(s: &str) -> std::result::Result<u128, String> {
    s.parse::<u128>()
        .map_err(|e| format!("Invalid u128 amount: {}", e))
}

/// Load network ID from config file
pub fn load_network_from_config(path: &str) -> Result<NetworkId> {
    let content = std::fs::read_to_string(path).unwrap_or_default();
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with("id") {
            if let Some((_, value)) = line.split_once('=') {
                let val = value.trim().trim_matches('"').trim();
                return parse_network_id(val);
            }
        }
    }
    Ok(NetworkId::Mainnet)
}

/// Extract a simple key = "value" or key = value from TOML content
pub fn extract_config_value(content: &str, key: &str) -> Option<String> {
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with(key) {
            if let Some((_, value)) = line.split_once('=') {
                let val = value.trim().trim_matches('"').trim();
                return Some(val.to_string());
            }
        }
    }
    None
}

/// Extract an array value like bootstrap_nodes = ["addr1", "addr2"]
pub fn extract_config_array(content: &str, key: &str) -> Vec<String> {
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with(key) {
            if let Some((_, value)) = line.split_once('=') {
                let val = value.trim();
                // Parse simple array: ["a", "b"]
                if val.starts_with('[') && val.ends_with(']') {
                    let inner = &val[1..val.len() - 1];
                    return inner
                        .split(',')
                        .map(|s| s.trim().trim_matches('"').trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                }
            }
        }
    }
    Vec::new()
}
