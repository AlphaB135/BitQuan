# BitQuan Examples

Step-by-step tutorials and code examples for common BitQuan operations.

## About These Examples

Each example includes:
- **Prerequisites** - What you need before starting
- **Step-by-step instructions** - Exact commands to run
- **Expected output** - What you should see
- **Common errors** - Typical problems and solutions

**Network:** All examples use `devnet` by default for safe testing.

## Quick Examples

### Create Wallet
```bash
./target/release/bitquan-node wallet-gen --output my-wallet.keystore
```
[Full Guide](create-wallet.md)

### Start Node
```bash
./target/release/bitquan-node --network devnet
```
[Full Guide](run-node.md)

### Mine Blocks
```bash
./target/release/bitquan-node mine --pow mock --datadir ./data/chainstate
```
[Full Guide](mine-blocks.md)

### Check Balance
```bash
./target/release/bitquan-node balance \
  --address <your-address> \
  --datadir ./data/chainstate
```
[Full Guide](run-node.md)

### Send Transaction
```bash
./target/release/bitquan-node wallet-send \
  --keystore my-wallet.keystore \
  --to <recipient-address> \
  --amount 1000000000000000000 \
  --datadir ./data/chainstate
```
[Full Guide](send-transaction.md)

## Available Examples

### Getting Started

| Example | Description | Time |
|---------|-------------|------|
| [Create Wallet](create-wallet.md) | Generate post-quantum wallet | 5 min |
| [Run Node](run-node.md) | Start BitQuan node | 10 min |
| [Mine Blocks](mine-blocks.md) | Mine blocks on devnet | 5 min |

### Transactions

| Example | Description | Time |
|---------|-------------|------|
| [Send Transaction](send-transaction.md) | Send coins to another address | 10 min |

### Advanced

| Example | Description | Time |
|---------|-------------|------|
| [RPC Calls](rpc-calls.md) | JSON-RPC API usage | 15 min |

## Example Conventions

### Commands

Commands are shown in code blocks:

```bash
# This is a comment explaining what the command does
./target/release/bitquan-node command --option value
```

### Placeholders

Replace `<placeholder>` with your actual values:

- `<your-address>` - Your BitQuan address
- `<recipient-address>` - Recipient's address
- `/path/to/file` - File path
- `<password>` - Your password

### Network Flags

All examples use `--network devnet` for safety. To use different networks:

| Network | Flag |
|---------|------|
| Devnet (default) | `--network devnet` |
| Testnet | `--network testnet` |
| Mainnet | `--network mainnet` |

**WARNING:** Never use mainnet addresses/keys for testing!

## Prerequisites

### Build from Source

```bash
# Clone repository
git clone https://github.com/AlphaB135/BitQuan.git
cd BitQuan

# Build release binary
cargo build --release

# Verify installation
./target/release/bitquan-node --version
```

### System Requirements

| Component | Minimum | Recommended |
|-----------|---------|-------------|
| OS | Linux/macOS/Windows | Linux (Ubuntu 22.04+) |
| RAM | 4GB | 8GB+ |
| Disk | 10GB | 50GB+ |
| CPU | 2 cores | 4+ cores |

### Dependencies

```bash
# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install additional dependencies (Ubuntu/Debian)
sudo apt-get install build-essential libssl-dev pkg-config

# Install additional dependencies (macOS)
xcode-select --install
```

## Common Commands Reference

### Wallet Commands

| Command | Purpose |
|---------|---------|
| `wallet-gen` | Create new wallet |
| `wallet-address` | Get wallet address |
| `wallet-send` | Send transaction |
| `balance` | Check balance |

### Node Commands

| Command | Purpose |
|---------|---------|
| (no command) | Start node |
| `mine` | Start mining |
| `info` | Show node info |

### Options

| Option | Purpose |
|--------|---------|
| `--network <net>` | Select network |
| `--datadir <path>` | Data directory |
| `--config <file>` | Config file |
| `--help` | Show help |

## Troubleshooting

### Command Not Found

```bash
# Error: bitquan-node: command not found

# Solution: Use full path
./target/release/bitquan-node --version

# Or add to PATH
export PATH="$PATH:$(pwd)/target/release"
```

### Permission Denied

```bash
# Error: Permission denied

# Solution: Make executable
chmod +x target/release/bitquan-node

# Or use rust run
cargo run --release -- bin bitquan-node --help
```

### Port Already in Use

```bash
# Error: Address already in use

# Solution: Kill existing process
pkill bitquan-node

# Or use different port
./target/release/bitquan-node --p2p-port 18445
```

## Best Practices

### Security

1. **Never share** keystore files or mnemonics
2. **Use strong passwords** (8+ characters, mixed case)
3. **Backup wallets** before any operation
4. **Test on devnet first** before using mainnet
5. **Verify addresses** before sending transactions

### Development

1. **Use devnet** for all testing
2. **Clean data directory** when needed: `rm -rf ./data/chainstate`
3. **Check logs** for errors: `tail -f bitquan.log`
4. **Run full tests** after code changes: `cargo test`

### Operations

1. **Monitor disk space** - blockchain grows over time
2. **Regular backups** - backup keystore and chainstate
3. **Update regularly** - keep BitQuan updated
4. **Monitor resources** - CPU, memory, disk usage

## Next Steps

After completing these examples:

1. **Read full documentation:**
   - [Getting Started](../getting-started/)
   - [Guides](../guides/)
   - [API Reference](../api/rpc/API_REFERENCE.md)

2. **Join the community:**
   - [GitHub Discussions](https://github.com/AlphaB135/BitQuan/discussions)
   - [GitHub Issues](https://github.com/AlphaB135/BitQuan/issues)

3. **Contribute:**
   - [Contributing Guide](../guides/CONTRIBUTING.md)
   - Report bugs
   - Suggest improvements

## Need Help?

- [Troubleshooting](../troubleshooting/) - Common problems and solutions
- [FAQ](../troubleshooting/faq.md) - Frequently asked questions
- [GitHub Issues](https://github.com/AlphaB135/BitQuan/issues) - Report bugs
