# BitQuan v0.0.1-alpha - Devnet Ready

## 🎉 First Alpha Release

BitQuan v0.0.1-alpha is the first public release featuring post-quantum cryptographic signatures (Dilithium3) with Bitcoin-inspired PoW consensus.

### ✨ Features

- ✅ **Post-Quantum Cryptography** (Dilithium3, 3293-byte signatures)
- ✅ **Block weight accounting** (4M WU cap, 384 WU per PQC sig)
- ✅ **ASERT difficulty adjustment** (per-block, 1-day half-life)
- ✅ **Fee-per-weight mempool** ordering
- ✅ **UTXO model** with coin maturity (100 blocks)
- ✅ **Network replay protection** (chain-id in sighash)
- ✅ **P2P networking** with relay policy
- ✅ **JSON-RPC 2.0 API**
- ✅ **Reproducible builds**

### 📊 Statistics

- **Tests:** 129 passing (100% pass rate)
- **Code Quality:** Clean (clippy -D warnings)
- **Fuzz Targets:** 4 (transaction, block, script, mempool)
- **Documentation:** Complete spec + BQIPs
- **Completion:** 96%

### 🔒 Security

- No backdoors, admin keys, or hidden switches
- GPG-signed commits required
- Network magic per chain (mainnet/testnet/devnet/regtest)
- Ban-score P2P protection (100 points threshold)
- Golden vector tests for sighash stability
- SLSA provenance attestations

### 🏗️ Architecture

**Crates:**
- `bitquan-types` - Core data structures (Transaction, Block, UTXO)
- `bitquan-consensus` - Validation rules (PoW, ASERT, block weight)
- `bitquan-crypto` - PQC primitives (Dilithium, SHA-256)
- `bitquan-mempool` - Fee-per-weight transaction pool
- `bitquan-network` - P2P protocol (inv/getdata, relay)
- `bitquan-storage` - RocksDB persistence
- `bitquan-rpc` - JSON-RPC 2.0 server
- `bitquan-node` - Binary entrypoint

### 📦 Installation

#### Option 1: Build from Source (Recommended)

```bash
# Clone repository
git clone https://github.com/AlphaB135/BitQuan.git
cd BitQuan
git checkout v0.0.1-alpha

# Build release binary
cargo build --release --locked

# Binary location
./target/release/bitquan-node --version
```

#### Option 2: Download Pre-built Binaries

Download for your platform from the Assets section below:
- `bitquan-node-linux-x64.tar.gz` - Linux x86_64
- `bitquan-node-macos-x64.tar.gz` - macOS Intel
- `bitquan-node-macos-arm64.tar.gz` - macOS Apple Silicon
- `bitquan-node-windows-x64.zip` - Windows x86_64

**Verify checksums:**
```bash
# SHA256
sha256sum -c bitquan-node-*.sha256

# SHA512
sha512sum -c bitquan-node-*.sha512
```

### 🚀 Quick Start

```bash
# Generate wallet
./bitquan-node wallet-gen --output wallet.keystore

# Get address
./bitquan-node wallet-address --keystore wallet.keystore

# Mine genesis block
./bitquan-node mine-genesis

# Start continuous mining
./bitquan-node mine
```

See [command.txt](https://github.com/AlphaB135/BitQuan/blob/main/command.txt) for complete CLI reference.

### ⚠️ Status: Alpha (Devnet)

This is an early alpha release for developers and testers. **Not recommended for production use.**

- ✅ Core protocol implemented
- ✅ All tests passing
- ⚠️ Limited peer testing
- ⚠️ No external security audit yet
- ⚠️ API may change in beta

### 🔍 Verification

**Reproducible Build:**
See [REPRODUCIBILITY.md](https://github.com/AlphaB135/BitQuan/blob/main/REPRODUCIBILITY.md)

**SLSA Provenance:**
All release artifacts include SLSA build provenance attestations.

**SBOM:**
Software Bill of Materials (CycloneDX format) included as `sbom.json`

### 📝 Changelog

See [CHANGELOG.md](https://github.com/AlphaB135/BitQuan/blob/main/CHANGELOG.md) for detailed changes.

### 🐛 Known Issues

None critical. See [Issues](https://github.com/AlphaB135/BitQuan/issues) for tracking.

### 🤝 Contributing

We welcome contributions! Please read:
- [CONTRIBUTING.md](https://github.com/AlphaB135/BitQuan/blob/main/CONTRIBUTING.md) - Guidelines
- [CODE_OF_CONDUCT.md](https://github.com/AlphaB135/BitQuan/blob/main/CODE_OF_CONDUCT.md) - Community standards
- [SECURITY.md](https://github.com/AlphaB135/BitQuan/blob/main/SECURITY.md) - Security policy

### 📞 Community

- **Issues:** https://github.com/AlphaB135/BitQuan/issues
- **Discussions:** https://github.com/AlphaB135/BitQuan/discussions
- **Security:** security@bitquan.org

### 📄 License

Apache License 2.0 - See [LICENSE](https://github.com/AlphaB135/BitQuan/blob/main/LICENSE)

---

**Release Date:** 2025-10-27  
**Git Commit:** `0f47798`  
**Rust Version:** 1.79+
