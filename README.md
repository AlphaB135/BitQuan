# BitQuan
<div align="center">
  <img src="https://raw.githubusercontent.com/AlphaB135/BitQuan/main/docs/img/BitQuan.png" alt="BitQuan Logo" width="200"/>
</div>

[![CI](https://img.shields.io/github/actions/workflow/status/AlphaB135/BitQuan/ci.yml?branch=main&label=CI)](https://github.com/AlphaB135/BitQuan/actions/workflows/ci.yml)
[![Local CI](https://img.shields.io/badge/Pre--Testnet%20Audit-9%2F9%20Passed%20(100%25)-brightgreen)](#project-status)
[![Release](https://img.shields.io/github/v/release/AlphaB135/BitQuan?label=Release&color=blue)](https://github.com/AlphaB135/BitQuan/releases/tag/v0.1.0-testnet)
[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![Rust](https://img.shields.io/badge/rust-stable-orange)](https://www.rust-lang.org)
[![Post-Quantum](https://img.shields.io/badge/Cryptography-Dilithium5-purple)](https://csrc.nist.gov/Projects/post-quantum-cryptography)

A sovereign proof-of-work blockchain with true post-quantum security using **CRYSTALS-Dilithium5** (NIST Level 5) signatures, ASERT difficulty adjustment, Nordic treasury tokenomics, and complete BIP-39 mnemonic wallet infrastructure.

---

## 🌐 Live Testnet Web Ecosystem

| Service | Live URL | Description |
| :--- | :--- | :--- |
| 👛 **Web Wallet** | [http://140.245.127.249/wallet/](http://140.245.127.249/wallet/) | Real Dilithium5 Keypair Generator (BIP-39 12, 24, and 512-word streams) |
| 🚰 **Testnet Faucet** | [http://140.245.127.249/faucet/](http://140.245.127.249/faucet/) | Instant 10.00 BQ Testnet Dispenser |
| 🛡️ **Audit Scorecard** | [http://140.245.127.249/session-security-audit.html](http://140.245.127.249/session-security-audit.html) | Live 9/9 Test Suite Verification Matrix (100% Pass Rate) |
| 📊 **Grafana Telemetry** | [http://140.245.127.249:3030/](http://140.245.127.249:3030/) | Real-time node metrics and network telemetry |

---

## 📦 Latest Release: `v0.1.0-testnet`

Pre-compiled binary packages and cryptographic checksums are available on the [GitHub Releases page](https://github.com/AlphaB135/BitQuan/releases/tag/v0.1.0-testnet):

```bash
# Download and extract Linux x86_64 release
wget https://github.com/AlphaB135/BitQuan/releases/download/v0.1.0-testnet/bitquan-v0.1.0-testnet-linux-x86_64.tar.gz
tar -xzf bitquan-v0.1.0-testnet-linux-x86_64.tar.gz
cd bitquan-v0.1.0-testnet

# Run node
./bitquan-node run --config testnet.toml
```

---

## 🚀 Quick Start

### 1. Run via Docker Compose (Non-Mining Relay Mesh)

Run a local 3-node simulation mesh with CPU and memory protection:

```bash
git clone https://github.com/AlphaB135/BitQuan.git && cd BitQuan
docker compose -f docker-compose.cluster.yml up -d
```

### 2. Build from Source

```bash
# Clone repository
git clone https://github.com/AlphaB135/BitQuan.git
cd BitQuan

# Build release binary (requires Rust 1.82+)
cargo build --release

# Run comprehensive test suite
cargo test --all
```

### 3. Generate Post-Quantum Wallets (BIP-39 Mnemonic)

```bash
# 12-Word Standard Mnemonic (Deterministic)
./target/release/bitquan-node wallet-gen-mnemonic --words 12 --show-mnemonic

# 24-Word High-Security Mnemonic
./target/release/bitquan-node wallet-gen-mnemonic --words 24 --show-mnemonic

# Restore Wallet from Mnemonic Phrase
./target/release/bitquan-node wallet-from-mnemonic --mnemonic "afford van hundred shaft mad school copy deny crumble blanket elder fish"
```

---

## 📊 Project Status & Security Hardening

**Current Phase**: Public Testnet (`v0.1.0-testnet`)

| Component | Status | Verification & Metrics |
| :--- | :---: | :--- |
| **PQC Cryptography** | Complete | CRYSTALS-Dilithium5 (6,533.6 TPS signature verification throughput) |
| **Consensus & ASERT** | Hardened | 9 arithmetic overflow vectors eliminated using checked/saturating math |
| **P2P Networking** | Hardened | TOCTOU handshake locks + sync backpressure queue (50-block memory cap) |
| **Mempool Integrity** | Hardened | Atomic spent outpoint rollback on multi-input invalid transactions |
| **RPC Daemon** | Hardened | Role-based JWT authentication + generate block limits + sanitized errors |
| **Tokenomics** | Complete | BQIP-0004 Nordic Treasury model with halving decay & uncle block rewards |
| **Pre-Testnet Suite** | **9/9 (100%)** | Full pass: Chaos tests (5/5), Mempool (29/29), Network (131/131), Wallet (43/43) |

---

## 🛡️ Post-Quantum Trade-offs

BitQuan implements NIST Level 5 CRYSTALS-Dilithium5 signatures, ensuring 50+ year security against quantum computing threats.

| Metric | Bitcoin (ECDSA) | BitQuan (Dilithium5) | Ratio |
| :--- | :--- | :--- | :--- |
| **Public Key Size** | 33 bytes | 2,592 bytes | **78x** |
| **Signature Size** | ~72 bytes | 4,595 bytes | **63x** |
| **Secret Key Size** | 32 bytes | 4,864 bytes | **152x** |
| **Quantum Resistance** | ❌ Vulnerable (Shor's Algorithm) | ✅ Immune (Lattice-based) | NIST Level 5 |

---

## 🏛️ Core Principles

- **Quantum-Proof Consensus**: CRYSTALS-Dilithium5 signatures with SHA-256d Proof-of-Work.
- **BIP-39 Mnemonic Standards**: 12, 24, and 512-word deterministic mnemonic derivation.
- **Fixed Hard Cap**: 21,000,000 BQ total supply with 18-decimal precision (`u128` qbits).
- **Simple & Resilient**: Pure value transfer, zero smart contract complexity, state pruning enabled.
- **Open Source**: Apache 2.0 license, fully auditable codebase.

---

## 📂 Repository Structure

```
BitQuan/
├── crates/
│   ├── consensus/       # ASERT difficulty & block validation rules
│   ├── crypto/          # Dilithium5, Argon2id, AES-256-GCM primitives
│   ├── mempool/         # Atomic double-spend protected transaction pool
│   ├── network/         # Tokio async P2P network with Noise protocol
│   ├── node/            # Node entrypoint, CLI commands & BIP-39 engine
│   ├── rpc/             # JSON-RPC server with JWT authentication
│   ├── storage/         # RocksDB high-performance persistence layer
│   ├── types/           # Core transaction & block data structures
│   ├── wallet/          # Deterministic wallet derivation & keystore encryption
│   └── faucet/          # Testnet faucet web service
├── scripts/             # API daemons, orchestrator suites, and BIP-39 wordlists
├── docs/                # Technical guides, BQIPs, and test specifications
└── docker-compose.cluster.yml # Multi-node simulation mesh
```

---

## 🤝 Contributing & Security Disclosure

- **Issues & Discussions**: [GitHub Issues](https://github.com/AlphaB135/BitQuan/issues)
- **Security Contact**: `bitquan.dev@proton.me`
- **License**: Apache License 2.0 (see [LICENSE](LICENSE))

---

*BitQuan Testnet is active for testing and verification purposes. Testnet coins have no monetary value.*
