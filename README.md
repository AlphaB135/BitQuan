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
  <a href="./badges/audit.svg"><img alt="Audit Status" src="./badges/audit.svg"></a>
</p>

**Badge Legend:**
- **CI**: Continuous Integration build status
- **License**: Apache 2.0 open-source license
- **Audit**: External security audit status (pass/fail/pending)


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

### Mainnet

For mainnet deployment, see **[MAINNET_ANNOUNCEMENT.md](./docs/MAINNET_ANNOUNCEMENT.md)** for:
- Official release binaries and checksums
- Network endpoints and DNS seeds
- Mining pool configuration
- Security best practices

### Development Build

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

### Hybrid Mining (Testnet Only)

BitQuan supports hybrid Proof-of-Work with multiple algorithms on testnet/devnet. **Mainnet strictly uses SHA-256d only.**

```bash
# Build with RandomX support
cargo build --release --features randomx

# Hybrid mining with weighted algorithm selection
./target/release/bitquan-node mine \
  --network devnet \
  --pow hybrid \
  --hybrid-weights "sha256d:1,randomx:2" \
  --threads 4 \
  --limit-blocks 10

# Pure RandomX mining (testnet/devnet)
./target/release/bitquan-node mine \
  --network testnet \
  --pow randomx \
  --threads 8

# Mainnet rejects RandomX (security gate)
./target/release/bitquan-node mine \
  --network mainnet \
  --pow hybrid  # ❌ Returns error: "RandomX disabled on mainnet"
```

**Hybrid Mining Features:**
- ✅ Weighted round-robin algorithm selection
- ✅ Per-algorithm difficulty tracking (ASERT)
- ✅ Prometheus metrics (`pow_mined_blocks_total`, `pow_hashrate_gauge`)
- ✅ Mainnet safety: RandomX consensus-level rejection
- ✅ Feature-gated: no dependencies unless `--features randomx`

See [docs/testnet/README.md](docs/testnet/README.md) for detailed hybrid mining instructions.

### Stratum Mining Server (Pool Mode)

BitQuan includes a Stratum V1-compatible mining server for connecting external miners:

```bash
# Start Stratum server on testnet
./target/release/bitquan-node stratum-server \
  --network testnet \
  --stratum-bind 0.0.0.0:3333 \
  --stratum-allow "127.0.0.1,192.168.0.0/16" \
  --stratum-diff 1.0

# Connect external miners (example with cgminer)
cgminer -o stratum+tcp://your-server:3333 -u miner1 -p x

# Monitor Stratum metrics
curl http://localhost:9090/metrics | grep stratum
```

**Stratum Features:**
- ✅ JSON-RPC protocol (mining.subscribe, mining.authorize, mining.submit)
- ✅ Per-miner session tracking and statistics
- ✅ Prometheus metrics (connections, shares, difficulty)
- ✅ Algorithm support: SHA-256d (mainnet) + RandomX (testnet)
- ✅ IP allowlist for access control

```

## RPC Server with TLS + JWT Authentication

BitQuan RPC server supports secure HTTPS with JWT authentication:

### Start RPC Server

```bash
# Production (mainnet) - requires CA-signed certificate
bitquan-node p2p-server \
  --datadir ./data/chainstate \
  --rpc-listen 0.0.0.0:8332 \
  --rpc-tls-cert /etc/letsencrypt/live/node.example.com/fullchain.pem \
  --rpc-tls-key /etc/letsencrypt/live/node.example.com/privkey.pem \
  --jwt-config jwt.toml

# Development - generate and use self-signed certificate
bitquan-node generate-cert --output ./certs
bitquan-node p2p-server \
  --datadir ./data/chainstate \
  --rpc-listen 127.0.0.1:8332 \
  --rpc-tls-cert ./certs/cert.pem \
  --rpc-tls-key ./certs/key.pem \
  --rpc-allow-insecure \
  --jwt-secret "dev-secret-change-in-production"
```

### JWT User Management

```bash
# Hash a password for jwt.toml
bitquan-node hash-password

# Add a user
bitquan-node jwt-user-add --config jwt.toml --username alice --role admin

# List all users
bitquan-node jwt-user-list --config jwt.toml

# Remove a user
bitquan-node jwt-user-remove --config jwt.toml --username alice
```

### RPC Usage Examples

```bash
# 1. Health check (no auth required)
curl -i https://127.0.0.1:8332/health -k

# 2. Login to get JWT token
curl -X POST https://127.0.0.1:8332/auth/login -k \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","password":"yourpass"}'

# Response: {"access_token":"eyJ...","token_type":"Bearer","expires_in":3600}

# 3. Call RPC method with JWT token
curl -X POST https://127.0.0.1:8332/rpc -k \
  -H "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..." \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"getblockcount","params":[],"id":1}'

# 4. Refresh expired token
curl -X POST https://127.0.0.1:8332/auth/refresh -k \
  -H "Content-Type: application/json" \
  -d '{"refresh_token":"eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."}'
```

### Configuration Options

Tune RPC behavior with CLI flags (defaults shown):

**Security**:
- `--rpc-tls-cert=<path>` – TLS certificate file (PEM format)
- `--rpc-tls-key=<path>` – TLS private key file (PEM format)
- `--rpc-allow-insecure` – Allow HTTP or self-signed certs (dev only)
- `--jwt-config=<path>` – JWT configuration file
- `--jwt-secret=<secret>` – Alternative to jwt-config file

**Rate Limiting**:
- `--rpc-max-body=1048576` – maximum JSON-RPC request size (bytes)
- `--rpc-rl-burst=20` – per-IP token bucket burst size
- `--rpc-rl-refill-per-sec=10` – token refill rate per second
- `--rpc-conn-cooldown-ms=10` – per-connection cooldown between requests

**Security Headers**:
- `--rpc-max-header=8192` – maximum HTTP header size (bytes)
- `--rpc-header-timeout-ms=1000` – header read timeout

**Proxy Support**:
- `--rpc-trust-proxy` – trust X-Forwarded-For header
- `--rpc-trusted-cidr=<cidrs>` – comma-separated trusted proxy CIDRs

See [docs/cli/](docs/cli/) for the complete CLI reference.

## Pre-Launch Validation

BitQuan includes comprehensive preflight validation to ensure mainnet readiness:

```bash
# Run full preflight validation
scripts/preflight/preflight.sh --network mainnet --release-tag v1.0.0-rc1

# View validation report
cat preflight_report.md
```

**Validation Checks:**
- ✅ Genesis hash verification
- ✅ DNS seeds reachability (≥60% threshold)
- ✅ Build reproducibility
- ✅ RPC security guards (401/408/429/431)
- ✅ Metrics endpoint availability
- ✅ PoW parameters locked for mainnet
- ✅ TLS/JWT configuration

See [docs/ops/PRELAUNCH_CHECKLIST.md](docs/ops/PRELAUNCH_CHECKLIST.md) for complete validation criteria.

---

## 📚 Documentation

**[📖 Full Documentation Site →](https://alphab135.github.io/BitQuan/)**

Browse complete documentation with search, navigation, and mobile support.

### Quick Links

📚 **[Complete Documentation Index](docs/INDEX.md)** - Central hub for all documentation

**Essential Guides:**
- **[🚀 Getting Started](docs/getting-started/)** - Installation and first steps
- **[⚙️ CLI Reference](docs/cli/)** - Command-line tools (node, wallet, stress, preflight)
- **[🛠️ Development](docs/dev/)** - Build, test, contribute
- **[🖥️ Operations](docs/ops/)** - Deployment, monitoring, runbooks
- **[🔒 Security](docs/security/)** - Audits, bug bounty, disclosure policy
- **[🌐 Testnet](docs/testnet/)** - Testnet setup and configuration

**Core Documents:**
- [Security Policy](SECURITY.md)
- [Contributing Guidelines](CONTRIBUTING.md)
- [Code of Conduct](CODE_OF_CONDUCT.md)
- [Changelog](CHANGELOG.md)

## Features

- **Post-Quantum Cryptography** (Dilithium3, 3293-byte signatures)
- **BIP39 Mnemonic Wallet** with deterministic key derivation (12/24 words)
- Block weight accounting (4,000,000 WU cap, 384 WU per PQC sig)
- Quantum-aware difficulty (ASERT + burst guard) with 4 h half-life, 11-block window, 0.33 floor ratio, 1.5× clamp
- Fee-per-weight mempool ordering
- Proof-of-Work consensus (SHA-256d mainnet, hybrid testnet)
- **Mining Pool & Dashboard** (Stratum V1, WebSocket, Grafana integration)
- UTXO model with coin maturity (100 blocks)
- Persistent storage (RocksDB)
- P2P networking with relay policy
- JSON-RPC 2.0 API
- Reproducible builds

### Mining Pool & Dashboard

BitQuan includes a full-featured mining pool for testnet operations:

- **Stratum V1 Server**: External miner support (cgminer, xmrig compatible)
- **Real Block Templates**: Automatic refresh every 30 seconds with HybridMiner integration
- **Variable Difficulty (Vardiff)**: Adaptive difficulty adjustment per miner
- **WebSocket Dashboard**: Real-time pool statistics and miner monitoring
- **Prometheus Metrics**: Full observability with Grafana integration
- **Share Verification**: Real PoW verification using SHA-256d or RandomX

### Reward Engine & Pool Persistence (Phase 4)

BitQuan now includes complete reward calculation and pool accounting:

- **Bitcoin-Style Halving**: 50 BQ initial reward, halvings every 210,000 blocks
- **Chain Persistence**: SQLite-backed block and miner tracking
- **Reward Calculation**: Automatic base reward + transaction fees
- **Pool Database**: Miner accounts, block history, payout records
- **RPC Endpoints**: `getpoolstats`, `getminerstats`, `createpayout`
- **Metrics Integration**: Real-time reward and balance tracking

See [POOL_OPERATIONS.md](docs/POOL_OPERATIONS.md) for pool lifecycle and payout details.

## Non-Goals

BitQuan intentionally does **NOT** include:

- ❌ **Smart Contracts**: No scripting language or Turing-complete execution layer
- ❌ **DeFi/DEX Features**: No built-in decentralized exchange or DeFi protocols
- ❌ **Governance Tokens**: No on-chain voting, staking, or delegation mechanisms
- ❌ **Alternative Consensus**: Only Proof-of-Work (no PoS, DPoS, BFT variants)
- ❌ **Experimental Cryptography**: Only peer-reviewed, NIST-approved algorithms
- ❌ **Marketing Gimmicks**: No promises of "moon", "get rich quick", or unrealistic TPS claims

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
See [Release Notes v0.0.2-alpha](docs/releases/RELEASE_NOTES_v0.0.2-alpha.md) for security hardening details.

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

## Try the BitQuan Public Testnet

BitQuan testnet is now available for testing and development!

**Quick Start (Public services: coming soon):**
```bash
# Run testnet node
./target/release/bitquan-node --network testnet --config config/testnet.toml

# Get testnet coins (coming soon)
# Visit: https://faucet.bitquan.dev
```

**Network Details:**
- Network: testnet
- P2P Port: 19444
- RPC Port: 19443
- Block Explorer: coming soon
- Faucet: coming soon

> Note: These ports are offset from Bitcoin testnet defaults (18444/18443) to avoid conflicts when running both on the same host.

For full testnet documentation, see [docs/TESTNET_README.md](docs/TESTNET_README.md)
