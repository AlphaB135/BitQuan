# BitQuan Quick Start Guide

Welcome to BitQuan - A post-quantum secure blockchain!

## Prerequisites

- Rust 1.75+ (`rustup install stable`)
- C compiler (for RocksDB)
- Git

## Installation

```bash
# Clone repository
git clone https://github.com/yourusername/BitQuan.git
cd BitQuan

# Build with RocksDB support
cargo build --release --features rocksdb-backend

# Verify installation
./target/release/bitquan-node --version
```

## Quick Start: Run a Local Node

### 1. Generate a Wallet

```bash
# Generate Dilithium3 keypair
./target/release/bitquan-node wallet-gen \
  --algo dilithium3 \
  --output wallet.keystore

# View your address
./target/release/bitquan-node wallet-address \
  --keystore wallet.keystore
```

Your address will start with `q1...` (Bech32m format)

### 2. Mine Genesis Block

```bash
# Quick mine (easy difficulty)
./scripts/generate_genesis.sh

# Or manually with custom parameters
./target/release/bitquan-node mine-once \
  --max-tries 1000000 \
  --payout-script-hex "YOUR_SCRIPT_HEX" \
  --bits 0x207fffff
```

### 3. Start Mining

```bash
# Continuous mining with 4 threads
./target/release/bitquan-node mine \
  --datadir ./data/chainstate \
  --payout-script-hex "YOUR_SCRIPT_HEX" \
  --bits 0x207fffff \
  --threads 4
```

### 4. Run P2P Node

```bash
# Start P2P server (listens for connections)
./target/release/bitquan-node p2p-server \
  --listen 127.0.0.1:8333 \
  --max-peers 125 \
  --datadir ./data/chainstate

# In another terminal, connect as peer
./target/release/bitquan-node p2p-connect \
  --peer 127.0.0.1:8333 \
  --height 0
```

### 5. Check Balance

```bash
# Check balance for a script
./target/release/bitquan-node balance \
  --datadir ./data/chainstate \
  --script-hex "YOUR_SCRIPT_HEX"
```

## RPC API Usage

### Start RPC Server (TODO: Coming soon)

```bash
./target/release/bitquan-node rpc-server \
  --listen 127.0.0.1:8332
```

### Python Client Example

```python
from scripts.rpc_client import BitQuanRPC

client = BitQuanRPC("http://127.0.0.1:8332")

# Get blockchain info
info = client.getblockchaininfo()
print(f"Height: {info['blocks']}")

# Get mining info
mining = client.getmininginfo()
print(f"Difficulty: {mining['difficulty']}")
```

## Building a Transaction

```bash
# Build a transaction (manual)
./target/release/bitquan-node build-tx \
  --prev-txid "PREV_TX_ID" \
  --prev-vout 0 \
  --value 1000000000 \
  --to-script-hex "RECIPIENT_SCRIPT"
```

## Advanced: Multi-Node Setup

### Node 1 (Miner)
```bash
# Terminal 1: Mine blocks
./target/release/bitquan-node mine \
  --datadir ./data/node1 \
  --payout-script-hex "YOUR_SCRIPT" \
  --threads 2

# Terminal 2: P2P server
./target/release/bitquan-node p2p-server \
  --listen 0.0.0.0:8333 \
  --datadir ./data/node1
```

### Node 2 (Peer)
```bash
# Terminal 1: P2P server
./target/release/bitquan-node p2p-server \
  --listen 0.0.0.0:8334 \
  --datadir ./data/node2

# Terminal 2: Connect to miner
./target/release/bitquan-node p2p-connect \
  --peer 127.0.0.1:8333
```

## Troubleshooting

### Build Issues

```bash
# Clean and rebuild
cargo clean
cargo build --release --features rocksdb-backend

# Check RocksDB installation (macOS)
brew install rocksdb

# Check RocksDB installation (Ubuntu)
sudo apt-get install librocksdb-dev
```

### Mining Too Slow

```bash
# Increase difficulty (easier)
--bits 0x207fffff  # Very easy
--bits 0x206fffff  # Easier
--bits 0x205fffff  # Even easier

# Use more threads
--threads 8  # Use 8 CPU cores
```

### P2P Connection Fails

```bash
# Check firewall
sudo ufw allow 8333/tcp

# Check if port is in use
lsof -i :8333

# Try different port
--listen 127.0.0.1:18333
```

## Configuration Files

Create `config/bitquan.toml`:

```toml
[network]
listen = "0.0.0.0:8333"
max_peers = 125

[mining]
threads = 4
default_bits = 0x207fffff

[storage]
datadir = "./data/chainstate"

[rpc]
listen = "127.0.0.1:8332"
enabled = true
```

## Testing

```bash
# Run all tests
cargo test --all --features rocksdb-backend

# Run specific test suite
cargo test --package bitquan-consensus
cargo test --package bitquan-network

# Run with verbose output
cargo test -- --nocapture
```

## Next Steps

1. Read the [Architecture Documentation](docs/architecture.md)
2. Review [BQIP Proposals](docs/bqips/)
3. Check [Contributing Guidelines](CONTRIBUTING.md)
4. Join the community (Discord/Telegram)

## Security Notice

⚠️ **IMPORTANT**: This is experimental software. Do not use for production or real value until:

1. ✅ Security audit completed
2. ✅ Testnet fully operational (3+ months)
3. ✅ Community consensus on mainnet launch
4. ✅ Reproducible builds verified by multiple parties

## Resources

- Documentation: `docs/`
- API Reference: `docs/api/`
- Specifications: `docs/spec/`
- Examples: `examples/`
- Community: [Discord](https://discord.gg/bitquan) | [Telegram](https://t.me/bitquan)

## Support

- GitHub Issues: Report bugs and feature requests
- Discussions: Technical questions and proposals
- Security: security@bitquan.org (PGP key in `docs/security/`)

---

**Happy Quantum-Safe Mining! 🚀🔐**
