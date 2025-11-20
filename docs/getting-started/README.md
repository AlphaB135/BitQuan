# Getting Started with BitQuan

Welcome to BitQuan! This section will help you get started with the post-quantum blockchain.

## 🚀 Quick Start

If you're new to BitQuan, start with the [Quick Start Guide](quick-start.md) to get up and running in 5 minutes.

## 📖 Essential Guides

- [Installation Guide](installation.md) - Detailed installation instructions
- [First Transaction](first-transaction.md) - Create your first transaction
- [Testnet Guide](testnet-guide.md) - Join testnet

## 🔧 For Node Operators

- [Node Operator Guide](../guides/node-operator.md) - Run a full node
- [Mining Guide](../guides/mining.md) - Start mining BitQuan

## 📚 Next Steps

After completing the getting started guides, explore:

- [Architecture Overview](../architecture/overview.md) - Understand system design
- [API Documentation](../api/) - Learn about RPC and SDK APIs
- [Security Best Practices](../security/best-practices.md) - Secure your setup

## ❓ Need Help?

- [Troubleshooting](../guides/troubleshooting.md) - Common issues and solutions
- [Community](https://github.com/AlphaB135/BitQuan/discussions) - Join our community
- [Issues](https://github.com/AlphaB135/BitQuan/issues) - Report bugs or request features

---

*Last updated: 2025-11-21*

## What is BitQuan?

BitQuan is a minimal proof-of-work blockchain designed for 50-year security resilience in the quantum era. It uses:

- **CRYSTALS-Dilithium3** - Post-quantum digital signatures
- **Bitcoin-inspired UTXO model** - Proven simplicity
- **ASERT difficulty adjustment** - Stable block times
- **Block weight accounting** - Fair fee market

## Quick Start Options

### 1. Run a Full Node

```bash
# Download latest release
wget https://github.com/AlphaB135/BitQuan/releases/latest/download/bitquan-node-linux-amd64

# Verify checksum
sha256sum -c bitquan-node-linux-amd64.sha256

# Run node
./bitquan-node-linux-amd64 run --network mainnet
```

### 2. Create a Wallet

```bash
# Generate new wallet
bitquan-wallet create --name my-wallet

# Get your address
bitquan-wallet address --wallet my-wallet

# Check balance
bitquan-wallet balance --wallet my-wallet
```

### 3. Join Testnet

```bash
# Start testnet node
bitquan-node run --network testnet --datadir ~/.bitquan/testnet

# Mine on testnet
bitquan-node mine --network testnet --threads 4
```

## Installation Methods

### Binary Release (Recommended)

Download pre-built binaries from [GitHub Releases](https://github.com/AlphaB135/BitQuan/releases).

Supported platforms:
- Linux (x86-64, ARM64)
- macOS (x86-64, Apple Silicon)
- Windows (x86-64)

### Build from Source

```bash
# Clone repository
git clone https://github.com/AlphaB135/BitQuan.git
cd BitQuan

# Build release binary
cargo build --release

# Binary location
./target/release/bitquan-node --version
```

See [Installation Guide](../guides/INSTALL.md) for detailed instructions.

### Docker

```bash
# Pull image
docker pull ghcr.io/alphab135/bitquan:latest

# Run node
docker run -d -p 28333:28333 -p 28332:28332 \
  -v ~/.bitquan:/data \
  ghcr.io/alphab135/bitquan:latest
```

## First Steps

### 1. Sync the Blockchain

```bash
# Start node and sync
bitquan-node run --network mainnet --datadir ~/.bitquan

# Check sync status
curl http://localhost:9090/metrics | grep sync_progress
```

### 2. Secure Your Node

```bash
# Generate JWT secret for RPC
bitquan-node jwt-keygen --output ~/.bitquan/jwt.secret

# Generate TLS certificate
bitquan-node generate-cert --output ~/.bitquan/certs

# Restart with security enabled
bitquan-node run \
  --jwt-secret ~/.bitquan/jwt.secret \
  --rpc-tls-cert ~/.bitquan/certs/cert.pem \
  --rpc-tls-key ~/.bitquan/certs/key.pem
```

### 3. Make Your First Transaction

```bash
# Create wallet
bitquan-wallet create --name main-wallet

# Get receiving address
bitquan-wallet address --wallet main-wallet

# Send transaction (after receiving funds)
bitquan-wallet send \
  --wallet main-wallet \
  --to bq1recipient... \
  --amount 10.0 \
  --fee 0.001
```

## Network Information

### Mainnet
- **Network ID**: `mainnet`
- **P2P Port**: 28333
- **RPC Port**: 28332
- **Block Time**: ~10 minutes
- **Block Reward**: See [Consensus Economics](/concepts/CONSENSUS_ECON.md)

### Testnet
- **Network ID**: `testnet`
- **P2P Port**: 18333
- **RPC Port**: 18332
- **Hybrid Mining**: SHA-256d + RandomX

## Next Steps

- **[CLI Reference](/cli/)** - Learn all commands
- **[Operations Guide](/ops/)** - Deploy to production
- **[Security](/security/)** - Best practices
- **[Development](/dev/)** - Build and contribute

## Getting Help

- **Documentation**: You're reading it! Browse via sidebar
- **GitHub Issues**: [Report bugs or ask questions](https://github.com/AlphaB135/BitQuan/issues)
- **Security**: See [Security Policy](../SECURITY.md) for vulnerability disclosure

## System Requirements

### Minimum
- **CPU**: 2 cores
- **RAM**: 4GB
- **Disk**: 50GB SSD
- **Network**: 10Mbps

### Recommended
- **CPU**: 4+ cores
- **RAM**: 8GB+
- **Disk**: 100GB+ NVMe SSD
- **Network**: 100Mbps+

## Frequently Asked Questions

### Is BitQuan quantum-resistant?

Yes! BitQuan uses CRYSTALS-Dilithium3 signatures, which are resistant to attacks by both classical and quantum computers. It's designed for 50+ year security.

### Can I mine BitQuan?

Yes! Mainnet supports hybrid mining:
- SHA-256d (ASIC miners) - Available from genesis
- RandomX (CPU miners) - Available from block 10,000+
- Ethash (GPU miners) - Available from block 10,000+

### Where can I get testnet coins?

Use the testnet faucet or mine on testnet. See [Testnet Guide](/testnet/README.md).

### Is BitQuan compatible with Bitcoin?

BitQuan uses a Bitcoin-inspired UTXO model but with post-quantum signatures. Addresses and transaction formats are different.

---

*Updated on: 2025-01-07*

**Ready to dive deeper?** Check out the [CLI Reference](/cli/) to master all BitQuan commands!
