# BitQuan

A post-quantum secure blockchain implementing Proof-of-Work consensus with CRYSTALS-Dilithium signatures.

## Overview

BitQuan is a cryptocurrency designed for 50+ year security resilience against quantum computing threats. It uses lattice-based cryptography (Dilithium) for digital signatures and maintains Bitcoin's proven Proof-of-Work consensus model.

## Quick Start

```bash
# Build
cargo build --release --features rocksdb-backend

# Run tests
cargo test --all

# Generate wallet
./target/release/bitquan-node wallet-gen --algo dilithium3 --output wallet.keystore

# Mine genesis block
./target/release/bitquan-node mine-genesis --max-tries 10000000
```

## Documentation

- [Installation Guide](docs/guides/INSTALL.md)
- [Quick Start Guide](docs/guides/QUICKSTART.md)
- [Architecture Overview](docs/architecture/overview.md)
- [Specifications](docs/spec/)
- [Security Policy](SECURITY.md)
- [Contributing Guidelines](CONTRIBUTING.md)
- [Release Process](docs/guides/docs/guides/RELEASE.md)

## Features

- Post-Quantum Cryptography (CRYSTALS-Dilithium)
- Proof-of-Work consensus (SHA-256d)
- Persistent storage (RocksDB)
- P2P networking protocol
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
- GPG-signed commits and releases
- Reproducible builds
- Security audits (planned)

Report security vulnerabilities to: security@bitquan.org

See [SECURITY.md](SECURITY.md) for details.

## Development Status

Current phase: Implementation (70% complete)

See [ROADMAP.md](ROADMAP.md) for detailed progress and upcoming milestones.

## Building from Source

Requirements:
- Rust 1.75 or later
- RocksDB development libraries

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install RocksDB (Ubuntu/Debian)
sudo apt-get install librocksdb-dev

# Install RocksDB (macOS)
brew install rocksdb

# Build BitQuan
cargo build --release --features rocksdb-backend
```

## License

Apache License 2.0

See [LICENSE](LICENSE) for details.

## Community

- GitHub: https://github.com/alphab/BitQuan
- Issues: https://github.com/alphab/BitQuan/issues
- Discussions: https://github.com/alphab/BitQuan/discussions

## Translations

- [Thai (ภาษาไทย)](docs/i18n/README.th.md)
- [English](docs/i18n/README.en.md)
- `todo.md` – Phase-by-phase master plan (Phase 0–13)

## Current Focus
1. Draft transaction and block data specifications (Phase 3)
2. Author BQIP drafts 0001–0004 aligned with the architectural decisions
3. Bootstrap the Rust baseline for core modules: `crypto/`, `consensus/`, `mempool/`, `p2p/`, `storage/`

## Contributing Workflow
- Review `docs/CONTRIBUTING.md` for the code review process and project standards
- Configure deterministic builds per `docs/REPRODUCIBILITY.md`
- (Optional) Enable pre-commit tooling hooks: `./scripts/install-hooks.sh`
- Submit signed commits (`git commit -S`) with every pull request

## Additional Security Resources
See the [security policy](SECURITY.md) for disclosure guidelines and contact information.
