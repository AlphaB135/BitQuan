# BitQuan Public Testnet

Welcome to the BitQuan public testnet! This network is designed for testing and development of the BitQuan blockchain.

## Network Information

| Parameter | Value |
|-----------|-------|
| **Network ID** | testnet |
| **P2P Port** | 18444 |
| **RPC Port** | 18443 |
| **Genesis Hash** | TBD (generated on first block) |
| **Difficulty** | 0x1d00ffff (easier than mainnet) |
| **Block Time** | 600 seconds (10 minutes) |
| **Block Reward** | 50 BQ (halving every 210,000 blocks) |

## Quick Start

### Running a Testnet Node

```bash
# Clone the repository
git clone https://github.com/AlphaB135/BitQuan.git
cd BitQuan

# Build the node
cargo build --release

# Run testnet node
./target/release/bitquan-node --network testnet --config config/testnet.toml
```

### Connecting to Bootstrap Nodes

The testnet has bootstrap nodes for peer discovery:
- `node1.bitquan.dev:18444`
- `node2.bitquan.dev:18444`

### Getting Testnet Coins

Visit the testnet faucet:
**https://faucet.bitquan.dev**

Provide your testnet address and receive testnet BQ for development.

## Network Services

### Block Explorer
**URL**: https://explorer.bitquan.dev

View blocks, transactions, and network statistics.

### Faucet
**URL**: https://faucet.bitquan.dev

Get free testnet coins for development (rate limited).

### RPC Endpoint
**URL**: https://rpc.bitquan.dev:18443 (if public RPC is enabled)

## Mining on Testnet

Mining difficulty is lower than mainnet for easier testing:

```bash
# Start mining (requires node to be running)
bitquan-node mine --address <your-testnet-address>
```

**Note**: You can mine on testnet without connecting to peers (solo mining enabled).

## Configuration

Testnet configuration file: `config/testnet.toml`

Key settings:
- `p2p_port = 18444`
- `rpc_port = 18443`
- `difficulty_bits = "0x1d00ffff"`
- `max_block_weight = 4000000`

## Genesis Block

Genesis block parameters in `data/testnet/genesis.json`:
- **Timestamp**: November 4, 2024
- **Initial Reward**: 50 BQ
- **Coinbase Message**: "BitQuan Testnet Genesis - Nov 2024 - Quantum-Resistant Future"

## Consensus Parameters

| Parameter | Value | Purpose |
|-----------|-------|---------|
| ASERT Half-Life | 2 days | Difficulty adjustment smoothing |
| BurstGuard Threshold | 1.5x | Sudden hashrate spike protection |
| Coinbase Maturity | 100 blocks | Blocks before mined coins spendable |
| Max Block Weight | 4M WU | Maximum block size limit |

## Wallet Operations

### Create Wallet
```bash
bitquan-node wallet create --network testnet
```

### Get Address
```bash
bitquan-node wallet address
```

### Check Balance
```bash
bitquan-node wallet balance
```

### Send Transaction
```bash
bitquan-node wallet send --to <address> --amount <amount>
```

## RPC API

Connect to testnet RPC:

```bash
curl -X POST http://localhost:18443 \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <jwt-token>" \
  -d '{"method": "getblockcount", "params": []}'
```

## Development Features

### Fast Sync
Enabled for quick testnet synchronization.

### Mining Without Peers
Solo mining allowed for development.

### Lower Difficulty
Easier mining for testing block generation.

### Periodic Resets
Testnet may be reset periodically (configurable).

## Network Status

Check network status:

```bash
# Get peer count
bitquan-node getpeerinfo

# Get blockchain info
bitquan-node getblockchaininfo

# Get mempool info
bitquan-node getmempoolinfo
```

## Testnet Characteristics

### ✅ What Testnet Has
- Post-quantum signatures (Dilithium3)
- ASERT difficulty adjustment
- BurstGuard protection
- Full consensus rules
- P2P networking
- RPC interface
- Wallet functionality

### ⚠️ What Testnet Differs From Mainnet
- Lower mining difficulty
- Faster block generation (for testing)
- May be reset periodically
- Free coins from faucet
- Less security (for testing only)

## Security Notice

**⚠️ TESTNET COINS HAVE NO VALUE**

- Do not use testnet for production
- Testnet may be reset without notice
- Private keys may be compromised
- Use mainnet for real value transfer

## Common Issues

### Cannot Connect to Bootstrap Nodes
Check your firewall settings and ensure port 18444 is open.

### RPC Connection Refused
Ensure the node is running and RPC port 18443 is accessible.

### Mining Not Working
Check that mining address is valid and node is synced.

## Support

- **GitHub**: https://github.com/AlphaB135/BitQuan
- **Documentation**: https://docs.bitquan.dev
- **Issues**: https://github.com/AlphaB135/BitQuan/issues
- **Discord**: https://discord.gg/bitquan (if available)

## For Developers

### Running Tests Against Testnet
```bash
# Integration tests
cargo test --test integration_tests -- --test-threads=1

# Testnet smoke tests
cargo test --test testnet_smoke
```

### Debugging
```bash
# Run with debug logging
bitquan-node --network testnet --log-level debug
```

### Reset Local Testnet Data
```bash
rm -rf data/testnet/chainstate
rm -rf data/testnet/node.log
```

## Testnet Milestones

- [x] Genesis block created
- [x] Bootstrap nodes configured
- [ ] Faucet deployed
- [ ] Block explorer deployed
- [ ] Public RPC endpoint enabled
- [ ] 1000 blocks mined
- [ ] 100 active nodes

## Roadmap

1. **Phase 1**: Internal testing (current)
2. **Phase 2**: Limited public testing
3. **Phase 3**: Full public testnet
4. **Phase 4**: Stress testing and optimization
5. **Phase 5**: Mainnet launch preparation

---

**Join the testnet and help test BitQuan!**

*Last Updated: November 4, 2024*
