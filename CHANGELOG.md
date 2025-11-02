# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

### Fixed
- **AES-GCM Deprecation Warnings** (2025-11-02)
  - Added `#[allow(deprecated)]` annotations for `from_slice()` calls in keystore
  - Maintained compatibility with aes-gcm 0.10 while suppressing warnings
  - All wallet tests passing (27/27)

- **BasicAuth Deprecation Warnings** (2025-11-02)
  - Isolated deprecated `RpcAuth` code with `#[allow(deprecated)]` annotations
  - Marked BasicAuth as legacy authentication method (JWT recommended)
  - Updated all usage sites in node CLI and RPC server
  - All RPC tests passing (21/21)
  - No deprecated warnings in compilation output

### Added
- **BIP39 Deterministic Mnemonic Support** (2025-11-02)
  - Full deterministic Dilithium3 keypair generation from BIP39 mnemonic seeds
  - Support for 12-word and 24-word mnemonic phrases
  - HMAC-SHA512 based key derivation for cryptographic security
  - Multiple key derivation from single mnemonic (index-based)
  - Passphrase support for additional security layer
  - Wallet recovery: same mnemonic → same keypair (reproducible)
  - CLI commands: `wallet-gen-mnemonic` and `wallet-from-mnemonic`
  - Comprehensive test coverage (12/12 tests passing)
  - Production-ready implementation using patched `pqc_dilithium`

### Changed
- Migrated from `pqcrypto-dilithium` to patched `pqc-dilithium-seeded`
  - Exposed `crypto_sign_keypair` function for deterministic generation
  - Enables wallet backup and recovery via mnemonic phrases
  - Maintains post-quantum security with Dilithium3

## [0.0.1-alpha] - 2025-10-27

### Added

#### Core Protocol
- Transaction structure with PQC witness segregation
- Block structure with 120-byte fixed header
- UTXO set management and validation
- Transaction builder with coin selection
- Coinbase transaction generation
- Replay protection via chain-id in sighash

#### Consensus
- Block weight calculation: base_size*4 + sig_count*384
- Block weight validation (MAX: 4,000,000 WU)
- ASERT difficulty retarget (1-day half-life, 10-min block time)
- Fork choice (longest chain rule)
- Reorg handling (max depth: 100 blocks)
- PoW verification (SHA-256d)
- Merkle tree construction and validation

#### Fee Market (BQIP-0002)
- Mempool fee-per-weight ordering (qbits/WU)
- Protected fee rate policy (>= 10 qbits/WU)
- Smart eviction (lowest fee first)
- Block template selection by fee density
- Weight limits for DoS protection

#### Cryptography
- Dilithium3 baseline (NIST Level 3)
- Falcon512 support (optional)
- SPHINCS+ enum (future)
- Domain-separated KDF/RNG
- Signature verification pipeline

#### Networking
- P2P protocol with version handshake
- Net-magic per network (mainnet/testnet/devnet/regtest)
- Inv/getdata relay
- Peer ban scoring
- DoS protection (rate limits, message size)

#### RPC
- JSON-RPC 2.0 server
- Blockchain methods (getblockcount, getblockchaininfo, getbestblockhash)
- Mining methods (getwork, submitwork, getblocktemplate)
- Transaction methods (gettransaction)
- Method allow-list and rate limiting

#### Storage
- RocksDB backend with column families
- UTXO set (txid:vout → TxOut)
- Block index (height → hash, hash → height)
- Block data storage
- Compaction hints
- Salvage tool stub

#### Testing
- 121 tests passing (48 consensus, 7 mempool, 11 crypto, 31 storage, etc.)
- Property-based tests (proptest): 7 tests
  * Weight calculation determinism
  * Signature weight linearity
  * Block weight composition
  * ASERT monotonicity
  * ASERT determinism
  * ASERT bounds
- Integration tests: 8 reorg scenarios
  * Deep reorg (5 blocks)
  * Sequential reorgs (2→3→4)
  * Equal height tie-breaking
  * Max depth enforcement
  * Orphan/duplicate detection

#### Documentation
- BQIP 0001-0004 (PQC, block weight, ASERT, governance)
- Core specifications (transaction.md, block.md, block-weight.md)
- System architecture (21KB, diagrams)
- Data structures (12KB, formats)
- Command reference (command.txt)
- Code of Conduct
- Security policy (SECURITY.md)
- Contributing guidelines (CONTRIBUTING.md)
- Reproducibility guide (REPRODUCIBILITY.md)
- Release process (RELEASE.md)

#### CI/CD
- Cross-platform CI (Linux, macOS, Windows)
- Formatting checks (cargo fmt)
- Linting (cargo clippy -D warnings)
- Test suite (cargo test --all --locked)
- Dependency audit (cargo deny)
- Security audit (cargo audit)
- Release workflow with multi-platform builds
- Reproducible builds (SOURCE_DATE_EPOCH)
- Checksums (SHA256/SHA512)
- SBOM generation (CycloneDX)

#### Networks
- Mainnet (chain-id: 1, magic: 0xBQA1) - not active
- Testnet (chain-id: 2, magic: 0xBQT1) - not active
- Devnet (chain-id: 3, magic: 0xBQD1) - ready
- Regtest (chain-id: 4, magic: 0xBQR1) - ready

### Changed
- Documentation restructured to Bitcoin standard (no emojis)
- README updated with badges and correct links
- Mempool ordering from FIFO to fee-per-weight
- Block validation includes weight enforcement
- Transaction weight formula standardized (BQIP-0002)

### Fixed
- Broken documentation links (docs/guides/docs/guides)
- Community URLs point to AlphaB135/BitQuan
- RPC TestHandler missing methods (getwork/submitwork)
- Type mismatches (u16 vs u32 for signer_index)
- Unused variable warnings

### Security
- Chain-id replay protection
- Max reorg depth safety limit (100 blocks)
- Block weight limits prevent DoS
- Peer ban scoring for malformed messages
- RPC method allow-list
- Signature verification (Dilithium3)

### Performance
- Estimated TPS: 10-20 (depends on sig count)
- Block validation: O(log n) merkle verification
- UTXO lookup: O(1) via RocksDB
- Dilithium3: ~1000 sigs/sec per core

### Known Limitations
- No light clients (SPV) yet
- No compact blocks (BIP152-like)
- No Schnorr batch verification
- Limited mining pool features
- No cross-chain bridges

### Notes
- NOT PRODUCTION READY - Devnet/Regtest only
- No mainnet/testnet launch yet
- API may change in future versions
- Requires Rust 2021 edition
- No unsafe code (forbid)

## [0.0.1] - 2025-10-25

### Added
- Core specifications (transaction, block, block-weight)
- BQIP 0001-0004 (PQC signatures, block weight, ASERT, governance)
- UTXO set management and validation
- Transaction builder with coin selection
- P2P relay manager with inv/getdata handling
- Mining pool RPC (getwork/submitwork)
- Mempool fee-per-weight ordering
- Block weight enforcement (cap: 4,000,000 WU)
- ASERT difficulty retarget stub
- CI/CD pipeline (fmt, clippy, test, deny, audit)
- Reproducible build documentation
- Code of Conduct
- Security policy and disclosure process

### Changed
- Documentation restructured to Bitcoin standard
- README updated with correct links and badges
- Removed emojis from technical documentation

### Fixed
- Broken documentation links
- Community URLs point to correct repository

## [0.0.1] - 2025-10-25

### Added
- Initialize repository policies (LICENSE, SECURITY, CONTRIBUTING, RELEASE)
- Baseline entropy layer via bq-crypto crate
- Cross-platform CI pipeline with formatting, linting, and audit checks
- Quickstart instructions in README
- Genesis block generation
- RocksDB persistent storage backend
- JSON-RPC 2.0 server
- P2P networking protocol
- Dilithium3 wallet support

For earlier planning history, see ROADMAP.md.

