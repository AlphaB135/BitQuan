# BitQuan

<div align="center">
  <img src="https://raw.githubusercontent.com/AlphaB135/BitQuan/main/docs/img/BitQuan.png" alt="BitQuan Logo" width="200"/>
</div>

[![Build Status](https://img.shields.io/github/actions/workflow/status/AlphaB135/BitQuan/security.yml?branch=main)](https://github.com/AlphaB135/BitQuan/actions)
[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![Rust](https://img.shields.io/badge/rust-stable-orange)](https://www.rust-lang.org)
[![Post-Quantum](https://img.shields.io/badge/Cryptography-Dilithium5-purple)](https://csrc.nist.gov/Projects/post-quantum-cryptography)

A proof-of-work blockchain with post-quantum security using **CRYSTALS-Dilithium5** signatures.

## TESTNET COMING SOON

**BitQuan is preparing for public testnet launch.**

| Milestone | Status |
|-----------|--------|
| Code Complete | Done |
| Internal Security Audit | Done |
| External Security Audit | In Progress |
| **Public Testnet** | **Coming Q1 2026** |
| Mainnet | After successful testnet + audit |

- **Code**: 100% complete, all 778 tests passing
- **Security**: Internal audit complete, external pending
- **Testnet**: Launching soon - follow for announcements
- **Mainnet**: NOT LAUNCHED (requires testnet validation + external audit)

**WARNING**: No real value - testnet coins are for testing only

## Project Status

| Component | Status | Target |
|-----------|--------|--------|
| **Code Completion** | 100% | Done |
| **Unit Tests** | 778 passing | Done |
| **Internal Security Audit** | Complete | Done |
| **External Security Audit** | In Progress | Q1 2026 |
| **Public Testnet** | Preparing | Q1 2026 |
| **Mainnet Launch** | Pending | After testnet |
| **Version** | v1.0-audit-20251122 | - |

**Current Phase**: Pre-Testnet (final preparations)

**Network Magic**: `0xe8f3e1e3` (mainnet, reserved)
**Mining**: SHA-256d (hashcash) primary, RandomX optional

## Core Principles

- **Proven Consensus**: Longest VALID chain rule, no checkpoints, no governance
- **Quantum-Resistant**: CRYSTALS-Dilithium5 post-quantum signatures (NIST-approved)
- **Simple & Secure**: No smart contracts, no DeFi, just value transfer
- **Proof-of-Work**: SHA-256d mining with RandomX support
- **Precision**: 18-decimal precision (1 BQ = 10^18 qbits) stored as `u128`
- **Hard Supply**: 21,000,000 BQ limit
- **Memory Safety**: ~15 unsafe blocks (all justified with SAFETY comments), minimal unwrap() in production code
- **Open Source**: Apache 2.0, fully auditable, no backdoors
- **Async-Powered**: High-performance network layer with DoS protection

## Post-Quantum Trade-offs

**Honest Assessment**: BitQuan uses Dilithium5 signatures which are ~63x larger than Bitcoin's ECDSA.

| Metric | Bitcoin | BitQuan | Ratio |
|--------|---------|---------|-------|
| Signature Size | ~73 bytes | 4,595 bytes | **63x** |
| Layer 1 TPS | ~7 | < 1 | By design |
| Quantum Security | ❌ Vulnerable | ✅ Dilithium5 | NIST standard |

**This is a deliberate trade-off**: We prioritize quantum security today over layer 1 efficiency.

- **Why**: Emergency hard forks for PQ migration are risky and disruptive
- **Solution**: Layer 2 protocols for scaling (payment channels, rollups)
- **Mitigation**: State pruning keeps full node storage manageable

**Full Analysis**: See [Post-Quantum Trade-offs](docs/post-quantum-trade-offs.md) | [TPS Analysis](docs/tps-analysis.md)

## Quick Start

**NOTE**: These commands are for TESTING ONLY. No mainnet is running. Coins on testnet/devnet have NO VALUE.

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
# Start mining with SHA-256d (default, ASIC-friendly)
./target/release/bitquan-node mine --pow hashcash

# Or mine with RandomX (experimental, CPU-friendly)
./target/release/bitquan-node mine --pow randomx

# For testing with instant blocks
./target/release/bitquan-node mine --pow mock
```



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
- **Memory Safety**: ~15 unsafe blocks (all justified with SAFETY comments)

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

- **Smart Contracts**: No scripting language or Turing-complete execution layer
- **DeFi/DEX Features**: No built-in decentralized exchange or DeFi protocols
- **Governance Tokens**: No on-chain voting, staking, or delegation mechanisms
- **Alternative Consensus**: Only Proof-of-Work (no PoS, DPoS, BFT variants)
- **Experimental Cryptography**: Only peer-reviewed, NIST-approved algorithms
- **Marketing Gimmicks**: No promises of "moon", "get rich quick", or unrealistic TPS claims

**Philosophy**: BitQuan does one thing well — quantum-resistant value transfer with simplicity and 50+ year security.

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
- Comprehensive security audits completed
- Production readiness: 100% (code complete, mainnet pending)
  - All security fixes applied (PR #80)
  - All logging migrated (PR #83)
  - All tests passing (200+ tests)
  - Post-quantum crypto (Dilithium5)
  - External security audit (pending)

Report security vulnerabilities to: security@bitquan.org

See [SECURITY.md](SECURITY.md) for disclosure policy and response SLAs.

## Development Status

Current version: v1.0-audit-20251122 (pre-mainnet)
Tests: 90+ unit tests + 10+ integration tests + E2E stress test validated (all passing)
Recent Updates:
- P2P TCP socket I/O implementation complete
- Reward maturity integration tests (100-block maturity)
- Noise Protocol encryption for P2P (ephemeral keys - V1)
- Code cleanup and documentation improvements
- **Major Refactor**: Migrated all values to `u128` (18 decimals)
- **Genesis Verified**: Validated genesis block generation with new precision
- **E2E Validated**: Full transaction flow tested (116 blocks, tx confirmed)

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

## Testnet - Coming Soon

BitQuan public testnet is **launching Q1 2026**.

**What to Expect:**
- Free testnet coins from faucet
- Full node sync testing
- Mining with real difficulty adjustments
- Transaction broadcasting and confirmation
- Wallet integration testing

**Pre-Launch Checklist:**
- [x] Core node implementation
- [x] Consensus rules finalized
- [x] P2P networking stable
- [x] RPC API complete
- [x] Internal security audit
- [ ] External security audit (in progress)
- [ ] Seed nodes deployed
- [ ] Faucet service ready
- [ ] Block explorer ready

**Stay Updated:**
- Watch this repo for release announcements
- Join [GitHub Discussions](https://github.com/AlphaB135/BitQuan/discussions)
