# BitQuan

## Genesis Manifesto

BitQuan is a minimal proof-of-work blockchain designed for 50-year security
resilience in the quantum era. It follows Bitcoin's simplicity but upgrades
its cryptography, ensuring verifiable, tamper-resistant value transfer beyond
the limits of classical computation.

**Core Principles:**
- **Quantum-Resistant**: CRYSTALS-Dilithium3 post-quantum signatures
- **Minimalist Design**: Only essential features, Bitcoin-inspired UTXO model
- **Long-Term Security**: Built for multi-decade cryptographic durability
- **Production-Ready**: Comprehensive tests, audit tooling, CI/CD safety checks

---

<p align="center">
  <a href="./docs/README.md">
    <img src="./docs/img/BitQuan.png" alt="BitQuan logo" width="200"/>
  </a>
</p>

<h1 align="center">BitQuan</h1>

<p align="center"><strong>A minimal Proof-of-Work blockchain with Dilithium PQC and a public UTXO ledger.</strong></p>

<p align="center">
  <a href="https://github.com/AlphaB135/BitQuan/actions"><img alt="CI" src="https://github.com/AlphaB135/BitQuan/workflows/CI/badge.svg"></a>
  <a href="LICENSE"><img alt="License" src="https://img.shields.io/badge/license-Apache%202.0-blue.svg"></a>
</p>


---

## Table of Contents

- [Overview](#overview)
- [Quick Start](#quick-start)
- [Documentation](#documentation)
- [Features](#features)
- [Repository Structure](#repository-structure)
- [Contributing](#contributing)

---

## Overview

BitQuan is a cryptocurrency designed for 50+ year security resilience against quantum computing threats. It uses lattice-based cryptography (Dilithium) for digital signatures and maintains Bitcoin's proven Proof-of-Work consensus model with block weight accounting for large PQC signatures.

## Security Status

**Last Security Audit:** November 2025 (Self-audit + AI-assisted)  
**Status:** ✅ Hardened (Tasks I, J, K complete)

### Recent Security Improvements
- ✅ Integer overflow protection (Nov 2025)
- ✅ Replay attack prevention (Nov 2025)
- ✅ Entropy audit complete (Nov 2025)
- ⏳ External security audit: Pending (pre-mainnet)

### Security Features
- Post-quantum signatures (Dilithium3)
- Checked arithmetic in all production-critical paths
- Network-bound signatures (replay protection)
- CSPRNG-backed key generation and encryption
- Reproducible builds and documented security policy

See [SECURITY.md](SECURITY.md) for the vulnerability disclosure process.

## Quick Start

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

# Verify database integrity
./target/release/bitquan-node verify-db --path data/chaindata --backup

# Check balance (requires script pubkey hex)
./target/release/bitquan-node balance --script-hex <SCRIPT_HEX>

# Mine genesis block
./target/release/bitquan-node mine-genesis

# Start continuous mining
./target/release/bitquan-node mine

# Dev/test fast mining with mock PoW (nonce=0 shortcut)
./target/debug/bitquan-node mine --network devnet --pow mock --limit-blocks 20

# Mainnet forbids mock PoW (returns an error)
./target/debug/bitquan-node mine --network mainnet --pow mock
```

## RPC Health & Testing

```bash
# Health check (no auth required)
curl -i http://127.0.0.1:8332/health

# Unauthorized request (expects 401)
curl -i \
  -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","method":"getblockcount","params":[],"id":1}' \
  http://127.0.0.1:8332/

# Authorized request (replace user/pass accordingly)
curl -i \
  -H 'Content-Type: application/json' \
  -H 'Authorization: Basic $(printf "alice:secret" | base64)' \
  -d '{"jsonrpc":"2.0","method":"getblockcount","params":[],"id":1}' \
  http://127.0.0.1:8332/
```

Tune behaviour with CLI flags (defaults shown):

- `--rpc-max-body=1048576` – maximum JSON-RPC request size (bytes)
- `--rpc-rl-burst=20` / `--rpc-rl-refill-per-sec=10` – per-IP token bucket guard
- `--rpc-conn-cooldown-ms=10` – per-connection cooldown between requests

See [docs/command.md](docs/command.md) for the complete CLI reference.

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
- [Address ↔︎ Script Guide](docs/address-and-script.md)
- [RPC Testing Guide](docs/rpc/testing.md)
- [Command Reference](docs/command.md)
- [Roadmap](ROADMAP.md)

## Features

- **Post-Quantum Cryptography** (Dilithium3, 3293-byte signatures)
- **BIP39 Mnemonic Wallet** with deterministic key derivation (12/24 words)
- Block weight accounting (4,000,000 WU cap, 384 WU per PQC sig)
- Quantum-aware difficulty (ASERT + burst guard) with 4 h half-life, 11-block window, 0.33 floor ratio, 1.5× clamp
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
