# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

## [0.0.1-alpha] - 2025-10-26

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

