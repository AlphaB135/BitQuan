//! CLI helper functions for BitQuan node

use crate::address::AddressNetwork;
use bitquan_types::error::{Error, Result};
use bitquan_types::NetworkId;

/// Format qbits to BQ (1 BQ = 10^18 qbits)
pub fn format_bq(qbits: u128) -> String {
    let bq = qbits / 1_000_000_000_000_000_000;
    let qats = qbits % 1_000_000_000_000_000_000;
    format!("{}.{:018}", bq, qats)
}

/// Parse network ID from string
pub fn parse_network_id(value: &str) -> Result<NetworkId> {
    match value.to_lowercase().as_str() {
        "mainnet" => Ok(NetworkId::Mainnet),
        "testnet" => Ok(NetworkId::Testnet),
        "devnet" => Ok(NetworkId::Devnet),
        "regtest" => Ok(NetworkId::Regtest),
        _ => Err(bitquan_types::error::Error::Invalid(format!(
            "Unknown network: {}. Valid options: mainnet, testnet, devnet, regtest",
            value
        ))),
    }
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
