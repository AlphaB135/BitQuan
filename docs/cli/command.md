# BitQuan CLI Reference

The `bitquan-node` binary bundles wallet tooling, mining demos, address utilities, and
networking scaffolding for the BitQuan prototype. Every invocation follows:

```bash
bitquan-node <COMMAND> [OPTIONS]
```

Run `bitquan-node help <COMMAND>` to inspect flags for a specific subcommand.

## Address & Script Utilities

| Command | What it does | Typical usage |
|---------|--------------|----------------|
| `script-from-address` | Emits the scriptPubKey (hex) for a Bech32m address. Writes metadata to `stderr` so pipelines can safely consume stdout. | `./target/release/bitquan-node script-from-address --address bq1... > script.hex` |
| `validateaddress` | Verifies checksum/HRP, prints normalized form, public-key hash, and derived script. | `./target/release/bitquan-node validateaddress --address bq1...` |

See also [`docs/address-and-script.md`](../address-and-script.md) for a walkthrough that pairs these commands.

## Wallet & Signing

| Command | Purpose |
|---------|---------|
| `wallet-gen` | Create a Dilithium3 keypair and encrypted keystore. |
| `wallet-gen-mnemonic` | Generate a BIP39 mnemonic phrase for wallet recovery. |
| `wallet-from-mnemonic` | Restore a wallet from a BIP39 mnemonic phrase. |
| `wallet-restore` | Restore a wallet keystore from backup. |
| `wallet-address` | Show the Bech32m address and pubkey hash from a keystore. |
| `wallet-sign` | Produce a Dilithium signature over a hex-encoded message. |
| `wallet-verify` | Verify a signature against a public key (placeholder implementation). |
| `wallet-send` | Construct, sign, and submit a basic transaction from a keystore. |

Combined example:

```bash
# Generate new wallet
./target/release/bitquan-node wallet-gen --output wallet.keystore

# Or generate from BIP39 mnemonic
./target/release/bitquan-node wallet-gen-mnemonic
./target/release/bitquan-node wallet-from-mnemonic --phrase "your twelve word mnemonic phrase here..."

# Get address and sign
./target/release/bitquan-node wallet-address --keystore wallet.keystore
./target/release/bitquan-node wallet-sign --keystore wallet.keystore --message deadbeef
```

## Mining & Consensus

| Command | Purpose |
|---------|---------|
| `mine-genesis` | Brute-force the devnet genesis block. |
| `mine-once` | Mine a single block template to illustrate PoW inner loop. |
| `mine` | Continuous CPU mining demo with configurable threads and payout script. |
| `run` | Launch prototype node loop (prints configured endpoints). |

Example payout flow:

```bash
SCRIPT_HEX=$(./target/release/bitquan-node script-from-address --address bq1...)
./target/release/bitquan-node mine --payout-script-hex "$SCRIPT_HEX" --threads 4
```

## Blockchain Inspection

| Command | Purpose |
|---------|---------|
| `balance` | Scan the local chainstate to sum UTXOs for a script or address. |
| `check-block` | Validate a serialized block from disk (placeholder). |
| `verify-db` | Verify RocksDB integrity and optionally create a backup. |
| `rng` | Display random bytes derived from the consensus RNG (debug). |
| `build-tx` | Generate a JSON template for a simple 1-in/1-out transaction. |

### Database Verification

The `verify-db` command helps ensure database integrity:

```bash
# Basic verification
./target/release/bitquan-node verify-db --path data/chaindata

# Verify with backup
./target/release/bitquan-node verify-db \
  --path data/chaindata \
  --backup \
  --backup-path backups/$(date +%Y%m%d-%H%M%S)

# Verify and rebuild indices if corruption detected
./target/release/bitquan-node verify-db \
  --path data/chaindata \
  --rebuild
```

**Options:**
- `--path` (string, default: `data/chaindata`) – database directory
- `--backup` (flag) – create backup before verification
- `--backup-path` (string) – backup directory (required if `--backup`)
- `--rebuild` (flag) – rebuild indices if corruption is detected

## Networking & P2P

| Command | Purpose |
|---------|---------|
| `p2p-demo` | Local handshake demo running client/server in a single process. |
| `p2p-server` | Bind a listening node for inbound peers (optionally with RPC when built). |
| `p2p-connect` | Dial a remote peer and perform a basic handshake. |
| `rpc-serve` | Start the JSON-RPC server with TLS and JWT authentication. |
| `jwt-keygen` | Generate a JWT secret key for RPC authentication. |

### RPC Server Setup

```bash
# Generate JWT secret
./target/release/bitquan-node jwt-keygen --output data/jwt.secret

# Start RPC server
./target/release/bitquan-node rpc-serve \
  --bind 127.0.0.1:28332 \
  --jwt-secret data/jwt.secret \
  --tls-cert certs/server.crt \
  --tls-key certs/server.key
```

## Getting Help

- `bitquan-node --help` — global options and the command list.
- `bitquan-node help <command>` — detailed flags for any subcommand.
- `cargo run -p bitquan-node -- --help` — regenerate this reference from source.
