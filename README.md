# BitQuan

[![CI](https://github.com/AlphaB135/BitQuan/workflows/CI/badge.svg)](https://github.com/AlphaB135/BitQuan/actions)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)

A minimal Proof-of-Work blockchain with Dilithium PQC and public UTXO ledger.

## Overview

BitQuan is a cryptocurrency designed for 50+ year security resilience against quantum computing threats. It uses lattice-based cryptography (Dilithium) for digital signatures and maintains Bitcoin's proven Proof-of-Work consensus model with block weight accounting for large PQC signatures.

## Quick Start

```bash
# Build
cargo build --release

# Run tests
cargo test --all --locked

# Generate wallet keypair
./target/release/bitquan-node wallet-gen --output wallet.keystore

# Get wallet address
./target/release/bitquan-node wallet-address --keystore wallet.keystore

# Check balance (requires script pubkey hex)
./target/release/bitquan-node balance --script-hex <SCRIPT_HEX>

# Mine genesis block
./target/release/bitquan-node mine-genesis

# Start continuous mining
./target/release/bitquan-node mine
```

See [command.txt](command.txt) for complete CLI reference.

## Documentation

Core Documents:
- [Security Policy](SECURITY.md)
- [Contributing Guidelines](CONTRIBUTING.md)
- [Code of Conduct](CODE_OF_CONDUCT.md)
- [Release Process](RELEASE.md)
- [Reproducible Builds](REPRODUCIBILITY.md)
- [Changelog](CHANGELOG.md)

Technical Specifications:
- [Transaction Specification](docs/spec/transaction.md)
- [Block Specification](docs/spec/block.md)
- [Block Weight & Fee Market](docs/spec/block-weight.md)
- [BQIP Proposals](docs/bqip/)

Guides:
- [Command Reference](command.txt)
- [Roadmap](ROADMAP.md)

## Features

- Post-Quantum Cryptography (Dilithium3, 3293-byte signatures)
- Block weight accounting (4,000,000 WU cap, 384 WU per PQC sig)
- ASERT difficulty adjustment (per-block, 1-day half-life)
- Fee-per-weight mempool ordering
- Proof-of-Work consensus (SHA-256d)
- UTXO model with coin maturity (100 blocks)
- Persistent storage (RocksDB)
- P2P networking with relay policy
- JSON-RPC 2.0 API
- Reproducible builds

## Repository Structure

```
bitquan/
├── crates/          # Rust workspace crates
│   ├── consensus/   # Consensus rules and validation
│   ├── crypto/      # Cryptographic primitives
│   ├── mempool/     # Transaction pool
│   ├── network/     # P2P networking
│   ├── node/        # Main node implementation
│   ├── rpc/         # JSON-RPC server
│   ├── storage/     # Database backend
│   └── types/       # Core data structures
├── docs/            # Documentation
├── scripts/         # Utility scripts
└── bindings/        # Language bindings
```

## Security

- No backdoors, admin keys, or hidden switches
- GPG-signed commits and releases required
- Reproducible builds with attestation
- All core code open-source, auditable
- Security audits planned for beta

Report security vulnerabilities to: security@bitquan.org

See [SECURITY.md](SECURITY.md) for disclosure policy and response SLAs.

## Development Status

Current version: v0.0.1-alpha (devnet ready)
Completion: 96%
Tests: 129 passing

See [ROADMAP.md](ROADMAP.md) for detailed progress and milestones.

## Building from Source

Requirements:
- Rust 1.82.0 or later (stable)
- RocksDB development libraries (optional, bundled by default)

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Build BitQuan
cargo build --release --locked

# Run full test suite
cargo test --all --locked

# Reproducible build
export SOURCE_DATE_EPOCH=1700000000
cargo build --release --locked
```

See [REPRODUCIBILITY.md](REPRODUCIBILITY.md) for deterministic builds.

## License

Apache License 2.0

See [LICENSE](LICENSE) for details.

## Community

- Repository: https://github.com/AlphaB135/BitQuan
- Issues: https://github.com/AlphaB135/BitQuan/issues
- Discussions: https://github.com/AlphaB135/BitQuan/discussions
- Security: security@bitquan.org

## Contributing

1. Read [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines
2. Sign commits with GPG (`git commit -S`)
3. Ensure all tests pass (`cargo test --all --locked`)
4. Follow code style (`cargo fmt --all`)
5. Pass linting (`cargo clippy --all-targets --all-features`)

Optional: Enable pre-commit hooks with `./scripts/install-hooks.sh`
