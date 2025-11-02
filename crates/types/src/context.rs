//! Transaction context for network isolation and replay protection.

use crate::transaction::NetworkId;
use serde::{Deserialize, Serialize};

/// Magic bytes for each network to prevent cross-network replay attacks.
impl NetworkId {
    /// Returns the 4-byte magic value for this network.
    /// These magic bytes are used in network messages and transaction context.
    pub const fn magic_bytes(self) -> [u8; 4] {
        match self {
            // "BIQN" in ASCII for mainnet
            NetworkId::Mainnet => [0x42, 0x49, 0x51, 0x4E],
            // "BIQT" for testnet
            NetworkId::Testnet => [0x42, 0x49, 0x51, 0x54],
            // "BIQD" for devnet
            NetworkId::Devnet => [0x42, 0x49, 0x51, 0x44],
            // "BIQR" for regtest
            NetworkId::Regtest => [0x42, 0x49, 0x51, 0x52],
        }
    }

    /// Returns the network name as a string.
    pub const fn name(self) -> &'static str {
        match self {
            NetworkId::Mainnet => "mainnet",
            NetworkId::Testnet => "testnet",
            NetworkId::Devnet => "devnet",
            NetworkId::Regtest => "regtest",
        }
    }
}

/// Transaction context that binds a transaction to a specific network and chain.
///
/// This prevents replay attacks across different networks or chain forks by
/// embedding the network identifier and genesis hash into transaction signatures.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TxContext {
    /// Network identifier (mainnet, testnet, devnet, regtest).
    pub network_id: NetworkId,
    /// Genesis block hash that uniquely identifies this chain.
    pub genesis_hash: [u8; 32],
}

impl TxContext {
    /// Creates a new transaction context.
    pub const fn new(network_id: NetworkId, genesis_hash: [u8; 32]) -> Self {
        Self {
            network_id,
            genesis_hash,
        }
    }

    /// Creates a mainnet context with the given genesis hash.
    pub const fn mainnet(genesis_hash: [u8; 32]) -> Self {
        Self::new(NetworkId::Mainnet, genesis_hash)
    }

    /// Creates a testnet context with the given genesis hash.
    pub const fn testnet(genesis_hash: [u8; 32]) -> Self {
        Self::new(NetworkId::Testnet, genesis_hash)
    }

    /// Creates a devnet context with the given genesis hash.
    pub const fn devnet(genesis_hash: [u8; 32]) -> Self {
        Self::new(NetworkId::Devnet, genesis_hash)
    }

    /// Creates a regtest context with the given genesis hash.
    pub const fn regtest(genesis_hash: [u8; 32]) -> Self {
        Self::new(NetworkId::Regtest, genesis_hash)
    }

    /// Serializes the context to bytes for signing.
    ///
    /// Format: [network_id (1 byte)] + [genesis_hash (32 bytes)]
    pub fn to_bytes(&self) -> [u8; 33] {
        let mut bytes = [0u8; 33];
        bytes[0] = self.network_id.as_u8();
        bytes[1..33].copy_from_slice(&self.genesis_hash);
        bytes
    }

    /// Returns the magic bytes for this context's network.
    pub const fn magic_bytes(&self) -> [u8; 4] {
        self.network_id.magic_bytes()
    }

    /// Returns the network name.
    pub const fn network_name(&self) -> &'static str {
        self.network_id.name()
    }
}

impl Default for TxContext {
    fn default() -> Self {
        Self::devnet([0u8; 32])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_magic_bytes() {
        // Test that each network has unique magic bytes
        assert_eq!(NetworkId::Mainnet.magic_bytes(), [0x42, 0x49, 0x51, 0x4E]); // "BIQN"
        assert_eq!(NetworkId::Testnet.magic_bytes(), [0x42, 0x49, 0x51, 0x54]); // "BIQT"
        assert_eq!(NetworkId::Devnet.magic_bytes(), [0x42, 0x49, 0x51, 0x44]); // "BIQD"
        assert_eq!(NetworkId::Regtest.magic_bytes(), [0x42, 0x49, 0x51, 0x52]); // "BIQR"

        // Verify they're all different
        let mainnet = NetworkId::Mainnet.magic_bytes();
        let testnet = NetworkId::Testnet.magic_bytes();
        let devnet = NetworkId::Devnet.magic_bytes();
        let regtest = NetworkId::Regtest.magic_bytes();

        assert_ne!(mainnet, testnet);
        assert_ne!(mainnet, devnet);
        assert_ne!(mainnet, regtest);
        assert_ne!(testnet, devnet);
        assert_ne!(testnet, regtest);
        assert_ne!(devnet, regtest);
    }

    #[test]
    fn test_network_names() {
        assert_eq!(NetworkId::Mainnet.name(), "mainnet");
        assert_eq!(NetworkId::Testnet.name(), "testnet");
        assert_eq!(NetworkId::Devnet.name(), "devnet");
        assert_eq!(NetworkId::Regtest.name(), "regtest");
    }

    #[test]
    fn test_tx_context_creation() {
        let genesis = [1u8; 32];

        let ctx = TxContext::new(NetworkId::Mainnet, genesis);
        assert_eq!(ctx.network_id, NetworkId::Mainnet);
        assert_eq!(ctx.genesis_hash, genesis);

        let ctx_mainnet = TxContext::mainnet(genesis);
        assert_eq!(ctx_mainnet.network_id, NetworkId::Mainnet);

        let ctx_testnet = TxContext::testnet(genesis);
        assert_eq!(ctx_testnet.network_id, NetworkId::Testnet);

        let ctx_devnet = TxContext::devnet(genesis);
        assert_eq!(ctx_devnet.network_id, NetworkId::Devnet);

        let ctx_regtest = TxContext::regtest(genesis);
        assert_eq!(ctx_regtest.network_id, NetworkId::Regtest);
    }

    #[test]
    fn test_tx_context_to_bytes() {
        let genesis = [0xAAu8; 32];
        let ctx = TxContext::new(NetworkId::Mainnet, genesis);

        let bytes = ctx.to_bytes();
        assert_eq!(bytes.len(), 33);
        assert_eq!(bytes[0], NetworkId::Mainnet.as_u8());
        assert_eq!(&bytes[1..33], &genesis[..]);
    }

    #[test]
    fn test_different_networks_different_context() {
        let genesis = [0xBBu8; 32];

        let mainnet_ctx = TxContext::mainnet(genesis);
        let testnet_ctx = TxContext::testnet(genesis);

        // Same genesis but different networks should produce different bytes
        assert_ne!(mainnet_ctx.to_bytes(), testnet_ctx.to_bytes());
        assert_ne!(mainnet_ctx.magic_bytes(), testnet_ctx.magic_bytes());
    }

    #[test]
    fn test_different_genesis_different_context() {
        let genesis1 = [0xCCu8; 32];
        let genesis2 = [0xDDu8; 32];

        let ctx1 = TxContext::mainnet(genesis1);
        let ctx2 = TxContext::mainnet(genesis2);

        // Same network but different genesis should produce different bytes
        assert_ne!(ctx1.to_bytes(), ctx2.to_bytes());
    }

    #[test]
    fn test_context_helpers() {
        let genesis = [0xEEu8; 32];
        let ctx = TxContext::mainnet(genesis);

        assert_eq!(ctx.magic_bytes(), NetworkId::Mainnet.magic_bytes());
        assert_eq!(ctx.network_name(), "mainnet");
    }

    #[test]
    fn test_default_context() {
        let ctx = TxContext::default();
        assert_eq!(ctx.network_id, NetworkId::Devnet);
        assert_eq!(ctx.genesis_hash, [0u8; 32]);
    }
}
