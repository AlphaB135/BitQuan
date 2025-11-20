# BitQuan CLI Reference

**Last Updated: 2025-01-07**

This section contains complete command-line interface documentation for all BitQuan binaries.

## Available Commands

### Core Binaries

- **[bitquan-node](./bitquan-node.md)** - Full node operations (start, stop, sync, RPC)
- **[bitquan-wallet](./bitquan-wallet.md)** - Wallet management (create, send, balance, keys)

### Testing & Operations

- **[bq-stress](./bq-stress.md)** - Network stress testing and load generation
- **[bq-preflight](./bq-preflight.md)** - Pre-deployment validation and health checks

## Quick Examples

```bash
# Start a full node
bitquan-node --network mainnet --datadir ~/.bitquan

# Create a new wallet
bitquan-wallet create --name my-wallet

# Run preflight checks
bq-preflight --config config/mainnet.toml

# Stress test network
bq-stress --tps 100 --duration 60s
```

## Common Options

Most BitQuan CLI tools support these common flags:

- `--config <path>` - Configuration file path
- `--network <network>` - Network selection (mainnet, testnet, regtest)
- `--datadir <path>` - Data directory path
- `--log-level <level>` - Logging verbosity (error, warn, info, debug, trace)
- `--help` - Show help message
- `--version` - Show version information

## Configuration

For detailed configuration options, see:
- [Development Guide](../dev/) - Build and development setup
- [Operations Guide](../ops/) - Production deployment

---

*Updated on: 2025-01-07*
