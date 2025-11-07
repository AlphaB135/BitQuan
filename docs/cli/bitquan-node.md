# bitquan-node Command Reference

**Last Updated: 2025-01-07**

The `bitquan-node` binary is the core full node implementation for BitQuan. It handles blockchain operations, wallet management, mining, and P2P networking.

## Usage

```bash
bitquan-node <COMMAND> [OPTIONS]
```

Run `bitquan-node help <COMMAND>` for detailed flags on any subcommand.

## Address & Script Utilities

| Command | Description | Example |
|---------|-------------|---------|
| `script-from-address` | Generate scriptPubKey (hex) from Bech32m address | `bitquan-node script-from-address --address bq1... > script.hex` |
| `validateaddress` | Verify address checksum/HRP and show metadata | `bitquan-node validateaddress --address bq1...` |

See also: [Address and Script Guide](../concepts/address-and-script.md)

## Wallet & Signing

| Command | Purpose |
|---------|---------|
| `wallet-gen` | Create Dilithium3 keypair and encrypted keystore |
| `wallet-gen-mnemonic` | Generate BIP39 mnemonic for recovery |
| `wallet-from-mnemonic` | Restore wallet from BIP39 mnemonic |
| `wallet-restore` | Restore wallet from backup |
| `wallet-address` | Show Bech32m address and pubkey hash |
| `wallet-sign` | Sign message with Dilithium signature |
| `wallet-verify` | Verify signature against public key |
| `wallet-send` | Construct, sign, and broadcast transaction |

### Wallet Examples

```bash
# Generate new wallet
bitquan-node wallet-gen --output wallet.keystore

# Generate from mnemonic
bitquan-node wallet-gen-mnemonic
bitquan-node wallet-from-mnemonic --phrase "your twelve word phrase..."

# Get address and sign
bitquan-node wallet-address --keystore wallet.keystore
bitquan-node wallet-sign --keystore wallet.keystore --message deadbeef
```

## Mining & Consensus

| Command | Purpose |
|---------|---------|
| `mine-genesis` | Generate devnet genesis block via PoW |
| `mine-once` | Mine single block (demo) |
| `mine` | Continuous CPU mining with configurable threads |
| `run` | Launch full node (prints endpoints) |

### Mining Example

```bash
# Get payout script from address
SCRIPT_HEX=$(bitquan-node script-from-address --address bq1...)

# Start mining to that address
bitquan-node mine --payout-script-hex "$SCRIPT_HEX" --threads 4
```

## Blockchain Inspection

| Command | Purpose |
|---------|---------|
| `balance` | Query UTXO balance for address/script |
| `check-block` | Validate serialized block |
| `verify-db` | Verify and repair RocksDB integrity |
| `rng` | Show consensus RNG output (debug) |
| `build-tx` | Generate transaction JSON template |

### Database Verification

```bash
# Basic verification
bitquan-node verify-db --path data/chaindata

# With backup
bitquan-node verify-db \
  --path data/chaindata \
  --backup \
  --backup-path backups/$(date +%Y%m%d-%H%M%S)

# Rebuild indices on corruption
bitquan-node verify-db --path data/chaindata --rebuild
```

**Options:**
- `--path` - Database directory (default: `data/chaindata`)
- `--backup` - Create backup before verification
- `--backup-path` - Backup destination (required with `--backup`)
- `--rebuild` - Rebuild indices if corrupted

## Networking & P2P

| Command | Purpose |
|---------|---------|
| `p2p-demo` | Local handshake demo (single process) |
| `p2p-server` | Start listening node for inbound peers |
| `p2p-connect` | Connect to remote peer |
| `rpc-serve` | Start JSON-RPC server with TLS/JWT |
| `jwt-keygen` | Generate JWT secret for RPC auth |

### RPC Server Setup

```bash
# Generate JWT secret
bitquan-node jwt-keygen --output data/jwt.secret

# Start RPC server
bitquan-node rpc-serve \
  --bind 127.0.0.1:28332 \
  --jwt-secret data/jwt.secret \
  --tls-cert certs/server.crt \
  --tls-key certs/server.key
```

## See Also

- [bitquan-wallet](./bitquan-wallet.md) - Dedicated wallet tool
- [Operations Guide](../ops/RUNBOOK.md) - Production deployment
- [RPC API](../rpc/) - RPC endpoint documentation

---

*Updated on: 2025-01-07*
