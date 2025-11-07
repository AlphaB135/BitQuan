# BitQuan Architecture Overview

**Last Updated: 2025-01-07**

High-level architecture overview of BitQuan components and design.

For detailed Thai documentation, see [architecture/overview.md](../architecture/overview.md).

## Core Components

### Consensus Layer
- Proof-of-Work (SHA-256d mainnet, hybrid testnet)
- ASERT difficulty adjustment
- Block weight accounting

### Cryptography
- CRYSTALS-Dilithium3 signatures (PQC)
- BLAKE2b hashing
- Bech32m addresses

### Storage
- RocksDB for blockchain data
- UTXO set indexing
- Efficient pruning support

### Network
- P2P gossip protocol
- Stratum mining protocol
- JSON-RPC API

### Wallet
- HD key derivation (BIP32/44)
- BIP39 mnemonic support
- Multi-signature capability

## See Also

- [Detailed Architecture](../architecture/overview.md) - Full documentation (Thai)
- [System Overview](../architecture/system-overview.md)
- [Data Structures](../architecture/data-structures.md)

---

*Updated on: 2025-01-07*
