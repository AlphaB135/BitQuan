# BitQuan
<div align="center">
  <img src="https://raw.githubusercontent.com/AlphaB135/BitQuan/main/docs/img/BitQuan.png" alt="BitQuan Logo" width="200"/>
</div>

[![CI](https://img.shields.io/github/actions/workflow/status/AlphaB135/BitQuan/ci.yml?branch=main&label=CI)](https://github.com/AlphaB135/BitQuan/actions/workflows/ci.yml)
[![Integration Tests](https://img.shields.io/github/actions/workflow/status/AlphaB135/BitQuan/integration-tests.yml?branch=main&label=Integration)](https://github.com/AlphaB135/BitQuan/actions/workflows/integration-tests.yml)
[![RPC Tests](https://img.shields.io/github/actions/workflow/status/AlphaB135/BitQuan/rpc-tests.yml?branch=main&label=RPC)](https://github.com/AlphaB135/BitQuan/actions/workflows/rpc-tests.yml)
[![Fast PR](https://img.shields.io/github/actions/workflow/status/AlphaB135/BitQuan/fast-pr.yml?label=PR%20Checks)](https://github.com/AlphaB135/BitQuan/actions/workflows/fast-pr.yml)
[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![Rust](https://img.shields.io/badge/rust-stable-orange)](https://www.rust-lang.org)
[![Post-Quantum](https://img.shields.io/badge/Cryptography-Dilithium5-purple)](https://csrc.nist.gov/Projects/post-quantum-cryptography)

A proof-of-work blockchain with post-quantum security using **CRYSTALS-Dilithium5** signatures.

## Project Status

**Current Phase**: Pre-Testnet Development (March 2026)

| Component | Status | Notes |
|-----------|--------|-------|
| Core Protocol | Complete | ASERT difficulty, P2P sync |
| Cryptography | Complete | Dilithium5 signatures |
| Data Integrity | Fixed | C1-C7 vulnerabilities resolved |
| Documentation | Complete | Production guides, BQIPs |
| Unit Tests | 600+ passing | 142+ consensus tests |
| Internal Audit | Complete | Security hardening done |
| External Audit | Pending | Q3 2026 target |
| Testnet | Q2 2026 | Public launch |
| Mainnet | Q4 2026 | Post-audit |

**Recent Fixes**: Data integrity (C1-C7), P2P sync, ASERT difficulty, unwrap elimination

## Documentation Index

### Core Specifications
| Document | Description |
|----------|-------------|
| [POST_QUANTUM_TRADEOFFS.md](docs/POST_QUANTUM_TRADEOFFS.md) | Honest analysis of Dilithium5 size trade-offs |
| [BQIP-0003_WALLET_STANDARDS.md](docs/BQIP-0003_WALLET_STANDARDS.md) | PQC PSBT, address format, SDK patterns |
| [BQIP-0004_L2_INTEGRATION.md](docs/BQIP-0004_L2_INTEGRATION.md) | Witness model, ZK-Rollup roadmap |

### Operations
| Document | Description |
|----------|-------------|
| [PRODUCTION_DEPLOYMENT.md](docs/PRODUCTION_DEPLOYMENT.md) | Hardware, network, security, monitoring, backup |
| [SDK_DESIGN.md](docs/SDK_DESIGN.md) | Rust SDK, TypeScript bindings, CLI tools |

### Community
| Document | Description |
|----------|-------------|
| [REDDIT_ROAST_RESPONSE.md](docs/REDDIT_ROAST_RESPONSE.md) | Honest response to criticism, what's fixed |

### Full Documentation
[Complete Documentation Site](https://alphab135.github.io/BitQuan/)

## Core Principles

- **Proven Consensus**: Longest VALID chain rule, no checkpoints, no governance
- **Quantum-Resistant**: CRYSTALS-Dilithium5 post-quantum signatures (NIST-approved)
- **Simple & Secure**: No smart contracts, no DeFi, just value transfer
- **Proof-of-Work**: SHA-256d mining with RandomX support
- **Precision**: 18-decimal precision (1 BQ = 10^18 qbits) stored as `u128`
- **Hard Supply**: 21,000,000 BQ limit
- **Memory Safety**: 14 unsafe blocks (all justified with SAFETY comments), minimal unwrap() in production code
- **Open Source**: Apache 2.0, fully auditable, no backdoors
- **Async-Powered**: High-performance network layer with DoS protection

## Post-Quantum Trade-offs

**Honest Assessment**: BitQuan uses Dilithium5 signatures which are ~63x larger than Bitcoin's ECDSA.

| Metric | Bitcoin | BitQuan | Ratio |
|--------|---------|---------|-------|
| Signature Size | ~73 bytes | 4,595 bytes | **63x** |
| Layer 1 TPS | ~7 | < 1 | By design |
| Quantum Security | Vulnerable | Dilithium5 | NIST standard |

**This is a deliberate trade-off**: We prioritize quantum security today over layer 1 efficiency.

- **Why**: Emergency hard forks for PQ migration are risky and disruptive
- **Solution**: Layer 2 protocols for scaling (payment channels, rollups)
- **Mitigation**: State pruning keeps full node storage manageable

**Full Analysis**: See [Post-Quantum Trade-offs](docs/POST_QUANTUM_TRADEOFFS.md) | [TPS Analysis](docs/TPS_ANALYSIS.md)

## Quick Start

### Build from Source

```bash
# Clone repository
git clone https://github.com/AlphaB135/BitQuan.git
cd BitQuan

# Build (requires Rust 1.82+)
cargo build --release

# Run tests
cargo test --all

# Run lints
cargo clippy -- -D warnings
```

### Generate Wallet

```bash
# Create new wallet with Dilithium5 keys
./target/release/bitquan-node wallet-gen --output wallet.keystore

# Get your address
./target/release/bitquan-node wallet-address --keystore wallet.keystore
# Output: bq1qyqsq9q5z5khxv8y2w3...
```

### Start Node

```bash
# Initialize configuration
./target/release/bitquan-node init --network testnet

# Start node
./target/release/bitquan-node run --config config/testnet.toml
```

### Mining

```bash
# SHA-256d mining (default)
./target/release/bitquan-node mine --pow hashcash --network testnet

# Mock mining for testing (instant blocks)
./target/release/bitquan-node mine --pow mock --network devnet
```

**Full Guide**: [SDK_DESIGN.md](docs/SDK_DESIGN.md) | [PRODUCTION_DEPLOYMENT.md](docs/PRODUCTION_DEPLOYMENT.md)



## Overview

BitQuan is a cryptocurrency designed for 50+ year security resilience against quantum computing threats. It implements a proven consensus model with post-quantum cryptographic signatures, maintaining simplicity while ensuring long-term security against quantum attacks.

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
- [CLI Tools](docs/guides/) - Command-line tools and guides
- [Development](docs/dev/) - Build, test, contribute
- [Operations](docs/ops/) - Deployment, monitoring, runbooks
- [Security](docs/security/) - Audits, bug bounty, disclosure policy
- [Installation Guide](docs/INSTALL_GUIDE.md) - Mainnet installation

### Core Documents

- [Security Policy](SECURITY.md)
- [Contributing Guidelines](CONTRIBUTING.md)
- [Code of Conduct](CODE_OF_CONDUCT.md)
- [Changelog](CHANGELOG.md)

## Features

- **Post-Quantum Cryptography**: CRYSTALS-Dilithium5 signatures (NIST-approved)
- **Proven Consensus**: Longest chain rule, no governance, no checkpoints
- **Proof-of-Work Mining**: SHA-256d (primary) with RandomX (experimental) for CPU/GPU mining
- **BIP39 Wallet Support**: 12/24 word mnemonic phrases with deterministic recovery
- **UTXO Model**: Transaction model with `u128` values (18 decimals) and 100-block coin maturity
- **Block Weight System**: 4MB blocks with 384 weight units per PQC signature
- **Difficulty Adjustment**: ASERT algorithm with integer fixed-point arithmetic
- **Async P2P Networking**: High-performance async network layer with DoS protection
- **JSON-RPC API**: Standard RPC interface
- **Stratum Mining Pool Support**: Stratum V1 protocol for pool mining
- **Memory Safety**: 14 unsafe blocks (all justified with SAFETY comments)

## Async Network Layer

BitQuan uses an async network layer powered by tokio for:

- **Slowloris Attack Protection**: 30-second total timeout per message
- **Scalability**: Handle 100,000+ concurrent connections
- **Efficiency**: 4KB per connection vs 8MB with threads

### Architecture

```
Tokio Runtime
├─ P2P Server (accept loop)
│  └─ Per-peer handlers (spawned tasks)
├─ RPC Server (async)
└─ Mining (spawn_blocking thread pool)
```

### Benefits

- **Memory**: 2000x improvement (4MB vs 8GB for 1000 peers)
- **Security**: Immune to Slowloris attacks
- **Performance**: Non-blocking I/O throughout

## Non-Goals

BitQuan intentionally does **NOT** include:

- **Smart Contracts**: No scripting language
- **DeFi Features**: No DEX, staking, governance
- **Alternative Consensus**: Only PoW (no PoS, DPoS)
- **Experimental Crypto**: Only NIST-approved algorithms

**Philosophy**: Quantum-resistant value transfer with 50+ year security horizon.

## Roadmap

```
Q1 2026: Security hardening (complete)
Q2 2026: Public testnet launch
Q3 2026: External security audit
Q4 2026: Mainnet launch (post-audit)
2027+:    Layer 2 development (ZK-Rollup)
```

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

### Audit Status

| Phase | Status | Date |
|-------|--------|------|
| Internal Audit | Complete | Feb 2026 |
| Code Hardening | Complete | Feb 2026 |
| External Audit | Planned | Q3 2026 |

**Resolved Issues**: C1-C7 data integrity, P2P sync, unwrap elimination

### Responsible Disclosure

**Do NOT open public issues for security vulnerabilities.**

Email: security@bitquan.org

Response SLA:
- Acknowledgment: 24 hours
- Initial assessment: 72 hours
- Critical fix: 7 days

See [SECURITY.md](SECURITY.md) for full policy.

### Security Features

- **Post-Quantum**: Dilithium5 (NIST FIPS 205)
- **No Backdoors**: No admin keys, no hidden switches
- **Reproducible Builds**: Deterministic compilation
- **Signed Releases**: GPG-signed commits and binaries
- **Memory Safety**: Rust + 14 justified unsafe blocks

## Development Status

Current version: v1.0-audit-20260204 (pre-mainnet)
Tests: 600+ tests passing (unit + integration + E2E stress)
Recent Updates:
- P2P TCP socket I/O implementation complete
- Reward maturity integration tests (100-block maturity)
- Noise Protocol encryption for P2P (ephemeral keys - V1)
- GHOST Protocol with uncle block validation and tokenomics
- Phase 4 ASERT difficulty (120s block time, 14,400s half-life)
- Data integrity hardening (C1-C7 vulnerabilities resolved)
- Code cleanup and documentation improvements

See [docs/archive/](docs/archive/) for historical audits and planning documents.

## Building from Source

Requirements:
- Rust 1.82.0 or later (stable)
- RocksDB development libraries (optional, bundled by default)

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone repository
git clone https://github.com/AlphaB135/BitQuan.git
cd BitQuan

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

### How to Help

| Area | Skills Needed | Priority |
|------|---------------|----------|
| Code review | Rust, blockchain | **High** |
| Security audit | Cryptography | **Critical** |
| Test coverage | Rust testing | **High** |
| Documentation | Technical writing | Medium |
| Layer 2 research | ZK proofs | Medium |

### Code Style

```bash
# Format code
cargo fmt --all

# Check lints (must pass)
cargo clippy --all-targets --all-features -- -D warnings

# Run all tests (must pass)
cargo test --all --locked

# Check for unwrap in production
cargo clippy -- -D clippy::unwrap_used
```

### Pull Request Process

1. Fork the repository
2. Create feature branch (`git checkout -b feature/my-feature`)
3. Make changes following [SECURITY_STANDARDS.md](docs/SECURITY_STANDARDS.md)
4. Run tests and lints locally
5. Sign commits with GPG (`git commit -S`)
6. Open PR against `main` branch
7. Wait for CI to pass (Fast PR < 7 min)
8. Address review feedback

**Guidelines**: [CONTRIBUTING.md](CONTRIBUTING.md) | [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md)

### Pre-commit Hooks

```bash
./scripts/install-hooks.sh
```

## Support

BitQuan is a spare-time open-source project.

### Ways to Help
- Star and share the repository
- Report bugs via [GitHub Issues](https://github.com/AlphaB135/BitQuan/issues)
- Submit pull requests (see [Contributing](#contributing))
- Donate via [PayPal](https://paypal.me/AtsadawutKhunthong) for CI/audit costs

### Contact
- **Issues**: https://github.com/AlphaB135/BitQuan/issues
- **Discussions**: https://github.com/AlphaB135/BitQuan/discussions
- **Security**: security@bitquan.org
- **Email**: security@bitquan.org

---

*BitQuan is NOT launched. No mainnet exists. All coins are testnet-only with NO VALUE.*
