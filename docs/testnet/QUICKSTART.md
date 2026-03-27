# BitQuan Testnet Quickstart

Join the BitQuan testnet in under 5 minutes.

## Option 1: Docker (Recommended)

```bash
# Clone and start
git clone https://github.com/AlphaB135/BitQuan.git
cd BitQuan
docker-compose up -d

# Check node status
curl http://localhost:19443/health

# View metrics dashboard
open http://localhost:3000  # Grafana (admin / admin123)
```

That's it. Your node is mining on testnet.

## Option 2: Build from Source

### Prerequisites

- Rust stable 1.79+
- macOS or Linux

### Steps

```bash
# 1. Clone
git clone https://github.com/AlphaB135/BitQuan.git
cd BitQuan

# 2. Build
cargo build --release

# 3. Start node (mining + RPC)
./target/release/bitquan-node \
  --network testnet \
  --datadir ./data/testnet \
  --rpc \
  --mine \
  --threads 2

# 4. Create wallet in another terminal
./target/release/bitquan-node wallet-gen \
  --network testnet \
  --datadir ./data/testnet

# 5. Check balance
./target/release/bitquan-node wallet-balance \
  --network testnet \
  --datadir ./data/testnet

# 6. Send a transaction
./target/release/bitquan-node wallet-send \
  --network testnet \
  --datadir ./data/testnet \
  --to <RECIPIENT_ADDRESS> \
  --amount 1000
```

### One-Command Start

```bash
./scripts/testnet-start.sh
```

This script builds (if needed), starts the node, and shows status.

## Option 3: Pre-built Binary

```bash
# Download latest release
curl -L -o bitquan-node.tar.gz \
  https://github.com/AlphaB135/BitQuan/releases/latest/download/bitquan-linux-x86_64.tar.gz
tar xzf bitquan-node.tar.gz

# Run
./bitquan-node --network testnet --mine
```

## Ports

| Service | Port | Description |
|---------|------|-------------|
| P2P | 19444 | Peer-to-peer networking |
| RPC | 19443 | JSON-RPC API |
| Grafana | 3000 | Monitoring dashboard |
| Prometheus | 9090 | Metrics scraping |

## Testnet Faucet

Get free testnet tokens:

```bash
curl -X POST http://localhost:8080/request \
  -H "Content-Type: application/json" \
  -d '{"address": "your_testnet_address", "amount": 10000}'
```

Or visit the faucet web UI (when available).

## Common Issues

**Port already in use**: Another process is using 19444/19443. Kill it or change ports in `config/testnet.toml`.

**Cannot connect to peers**: Bootstrap nodes may be down. Use `--allow-mining-without-peers` flag.

**Build fails**: Ensure Rust 1.79+ with `rustup update stable`.

## Next Steps

- [RPC Reference](../RPC_REFERENCE.md) — All available commands
- [Developer Guide](../DEVELOPER_GUIDE.md) — Build on top of BitQuan
- [Mining Guide](../TESTNET_README.md) — Hybrid mining (SHA-256d + RandomX + Ethash)
