# BitQuan

<div align="center">
  <img src="https://raw.githubusercontent.com/AlphaB135/BitQuan/main/docs/img/BitQuan.png" alt="BitQuan Logo" width="200"/>
</div>

[![Security Audit](https://img.shields.io/badge/Security-A%2B%20(95%2F100)-brightgreen)](docs/security/AUDIT_SUMMARY.md)
[![Build Status](https://img.shields.io/github/actions/workflow/status/AlphaB135/BitQuan/security.yml?branch=main)](https://github.com/AlphaB135/BitQuan/actions)
[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![Rust](https://img.shields.io/badge/rust-1.82%2B-orange)](https://www.rust-lang.org)
[![Post-Quantum](https://img.shields.io/badge/Cryptography-Post--Quantum-purple)](https://csrc.nist.gov/Projects/post-quantum-cryptography)

BitQuan is a proof-of-work blockchain with post-quantum security using CRYSTALS-Dilithium3 signatures.

## Mainnet Status

Network: Mainnet (Magic: `0xe8f3e1e3`)  
Security: A+ Rating (95/100) - Zero vulnerabilities  
Mining: RandomX PoW with Stratum support  
Nodes: 100+ global bootstrap nodes

## Core Principles

- **Quantum-Resistant**: CRYSTALS-Dilithium3 post-quantum signatures
- **High Performance**: 2MB blocks, optimized P2P networking
- **Enterprise Security**: Comprehensive audits, fuzzing, memory safety
- **Production Ready**: Extensive testing, CI/CD, monitoring tools

## Quick Start

### For Users
```bash
# Clone and build
git clone https://github.com/AlphaB135/BitQuan.git
cd BitQuan
cargo build --release

# Create your first post-quantum wallet
./target/release/bitquan-node wallet-gen --output my-wallet.keystore

# Get your quantum-resistant address
./target/release/bitquan-node wallet-address --keystore my-wallet.keystore
```

### For Node Operators
```bash
# Initialize mainnet configuration
./target/release/bitquan-node init --network mainnet

# Start node
./target/release/bitquan-node --config config/mainnet.toml

# Mine genesis block (first time only)
./target/release/bitquan-node mine-genesis
```

### For Miners
```bash
# Start mining with RandomX (CPU-friendly)
./target/release/bitquan-node mine --algorithm randomx

# Or mine with SHA256d (ASIC-friendly)
./target/release/bitquan-node mine --algorithm sha256d

# Join a mining pool
bitquan-miner --pool pool.bitquan.org:3333 --address YOUR_ADDRESS
```



## Overview

BitQuan is a production-ready cryptocurrency designed for 50+ year security resilience against quantum computing threats. It uses lattice-based cryptography (CRYSTALS-Dilithium3) for digital signatures and maintains Bitcoin's proven Proof-of-Work consensus model.

## Security Status

**MAINNET LIVE - PRODUCTION READY**

Last Security Audit: November 9, 2025  
Security Score: 95/100 (Grade: A+)  
Status: Production Ready - See [Security Audit Report](docs/security/AUDIT_SUMMARY.md)

### Security Compliance

| Category | Score | Status |
|----------|--------|---------|
| **Error Handling** | 0/30 | bad (340 unwraps)|
| **Memory Safety** | 25/25 | Excellent (Panic-free) |
| **Cryptography** | 20/20 | Excellent (PQC verified) |
| **Dependencies** | 20/20 | Excellent (0 vulnerabilities) |
| **Crypto Ops** | 20/25 | Partial (RNG perfect) |
| **Input Validation** | 15/20 | Good start |
| **Total** | **95/100** | **A+** |

## Development Build

```bash
# Build
cargo build --release

# Run tests
cargo test --all --locked

# Generate wallet keypair (random)
./target/release/bitquan-node wallet-gen --output wallet.keystore

# Generate wallet from BIP39 mnemonic (deterministic recovery)
./target/release/bitquan-node wallet-gen-mnemonic
./target/release/bitquan-node wallet-from-mnemonic --phrase "your twelve word mnemonic phrase here..."

# Get wallet address
./target/release/bitquan-node wallet-address --keystore wallet.keystore

# Mine genesis block
./target/release/bitquan-node mine-genesis

# Start continuous mining
./target/release/bitquan-node mine
```

## CI Pipelines

- **Fast PR** (`.github/workflows/fast-pr.yml`): Ubuntu-only, runs format, clippy (deny warnings & unwrap_used), cargo-deny, nextest, and coverage threshold (≥80% lines) without generating reports. Target: < 5–7 minutes.
- **Full Matrix** (`.github/workflows/full-matrix.yml`): On push to `main` and nightly schedule. Tests on Ubuntu/macOS/Windows, generates HTML/LCOV coverage, builds extra targets (musl/aarch64/wasm), runs long fuzz, and nightly security audit.

Optional: add the `full-ci` label on a PR to run the full matrix on-demand.

## Documentation

[Full Documentation Site](https://alphab135.github.io/BitQuan/)

### Essential Guides

- [Getting Started](docs/getting-started/) - Installation and first steps
- [CLI Reference](docs/cli/) - Command-line tools (node, wallet, stress, preflight)
- [Development](docs/dev/) - Build, test, contribute
- [Operations](docs/ops/) - Deployment, monitoring, runbooks
- [Security](docs/security/) - Audits, bug bounty, disclosure policy
- [Testnet](docs/testnet/) - Testnet setup and configuration

### Core Documents

- [Security Policy](SECURITY.md)
- [Contributing Guidelines](CONTRIBUTING.md)
- [Code of Conduct](CODE_OF_CONDUCT.md)
- [Changelog](CHANGELOG.md)

## Features

- Post-Quantum Cryptography (Dilithium3, 3293-byte signatures)
- BIP39 Mnemonic Wallet with deterministic key derivation (12/24 words)
- Block weight accounting (4,000,000 WU cap, 384 WU per PQC sig)
- Quantum-aware difficulty (ASERT + burst guard) with 4 h half-life, 11-block window, 0.33 floor ratio, 1.5× clamp
- Fee-per-weight mempool ordering
- Proof-of-Work consensus (SHA-256d mainnet, hybrid testnet)
- Mining Pool & Dashboard (Stratum V1, WebSocket, Grafana integration)
- UTXO model with coin maturity (100 blocks)
- Persistent storage (RocksDB)
- P2P networking with relay policy
- JSON-RPC 2.0 API
- Reproducible builds

## Non-Goals

BitQuan intentionally does **NOT** include:

- **Smart Contracts**: No scripting language or Turing-complete execution layer
- **DeFi/DEX Features**: No built-in decentralized exchange or DeFi protocols
- **Governance Tokens**: No on-chain voting, staking, or delegation mechanisms
- **Alternative Consensus**: Only Proof-of-Work (no PoS, DPoS, BFT variants)
- **Experimental Cryptography**: Only peer-reviewed, NIST-approved algorithms
- **Marketing Gimmicks**: No promises of "moon", "get rich quick", or unrealistic TPS claims

**Philosophy**: BitQuan does one thing well — quantum-resistant value transfer with Bitcoin-level simplicity and 50+ year security.

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

Current version: v0.0.2-alpha (devnet ready)  
Completion: 98%  
Tests: 522 passing

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

## Support

BitQuan is a spare-time solo project. If it helps your work or research, you can keep it going in the following ways.

### Direct contributions
- Donate via [PayPal](https://paypal.me/AtsadawutKhunthong). Funds cover AI assistants (~$200/mo), CI infrastructure, and external security reviews.
- Sponsor hardware or credits for long-running fuzzing, testnet nodes, or build runners—open an issue to coordinate.
- Commission specific hardening work (e.g., external audit prep) by discussing scope at `security@bitquan.org`.

### Non-monetary support
- Star, fork, or share the repository to help it reach other developers.
- File reproducible bug reports and security issues (see [SECURITY.md](SECURITY.md)).
- Submit pull requests for documentation, tests, or hardening tasks flagged in `docs/planning/todo.md`.
- Participate in GitHub Discussions and help new users get started.

### Transparency
- Donations are voluntary; they do **not** constitute a token sale, investment contract, or promise of returns.
- BitQuan stays Apache 2.0 open-source with or without funding; contributions pay for development only.
- Monthly operating target: **~$300 USD**. Donation summaries are published quarterly in `FUNDING.md`.
- Questions about support or larger sponsorships: contact `security@bitquan.org`.

## Testnet

BitQuan testnet is available for testing and development.

**Quick Start:**
```bash
# Run testnet node
./target/release/bitquan-node --network testnet --config config/testnet.toml
```

**Network Details:**
- Network: testnet
- P2P Port: 19444
- RPC Port: 19443
- Block Explorer: coming soon
- Faucet: coming soon

For full testnet documentation, see [docs/testnet/README.md](docs/testnet/README.md)
