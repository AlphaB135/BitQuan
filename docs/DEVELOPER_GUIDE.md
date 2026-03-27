# BitQuan Developer Guide

**Last Updated**: 2026-03-27

## Prerequisites

- Rust stable 1.79+ (`rustup default stable`)
- macOS or Linux (Windows not tested)
- For RandomX mining: C compiler (clang/gcc)

## Build from Source

```bash
# Clone
git clone https://github.com/AlphaB135/BitQuan.git
cd BitQuan

# Standard build (SHA-256d mining only)
cargo build --release

# With RandomX support (adds ~50MB deps)
cargo build --release --features randomx

# Run tests
cargo test --workspace

# Run with lint
cargo clippy --workspace -- -D warnings
```

Binary location: `./target/release/bitquan-node`

## Quick Start: Local Devnet

```bash
# 1. Generate wallet
./target/release/bitquan-node wallet-gen \
  --network devnet \
  --datadir ./data/devnet

# Note the address and file path from output.

# 2. Mine genesis block
./target/release/bitquan-node mine-genesis \
  --network devnet \
  --datadir ./data/devnet

# 3. Start node with RPC and mining
./target/release/bitquan-node run \
  --config config/devnet.toml \
  --rpc-bind 0.0.0.0:18332 \
  --p2p-bind 0.0.0.0:18444

# 4. Check balance in another terminal
./target/release/bitquan-node wallet-balance \
  --network devnet \
  --datadir ./data/devnet

# 5. Send a transaction
./target/release/bitquan-node wallet-send \
  --network devnet \
  --datadir ./data/devnet \
  --to <RECIPIENT_ADDRESS> \
  --amount 1000
```

## CLI Commands

| Command | Description |
|---------|-------------|
| `run` | Start the node (config file, RPC, P2P) |
| `mine-genesis` | Mine the genesis block |
| `mine-once` | Mine a single block (demo miner) |
| `mine` | Continuous mining with storage |
| `wallet-gen` | Generate a post-quantum keypair (Dilithium5) |
| `wallet-gen-mnemonic` | Generate wallet from BIP-39 mnemonic |
| `wallet-restore` | Restore wallet from mnemonic |
| `wallet-send` | Send a transaction |
| `wallet-address` | Show wallet address |
| `wallet-sign` | Sign a transaction |
| `wallet-verify` | Verify a transaction signature |
| `wallet-backup` | Backup wallet keystore |
| `check-block` | Validate a block file |
| `address-validate` | Validate a BitQuan address |
| `rng` | Generate random bytes (test utility) |

Run `./target/release/bitquan-node --help` for full argument list.

## RPC API

See [RPC Reference](./RPC_REFERENCE.md) for all endpoints with request/response examples.

Quick start with RPC:

```bash
# Authenticate
TOKEN=$(curl -s -X POST http://localhost:18332/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","password":"your_password"}' | jq -r '.token')

# Get block count
curl -X POST http://localhost:18332 \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"getblockcount","params":[],"id":1}'
```

## Project Structure

```
BitQuan/
├── crates/
│   ├── bitquan-types/     # Core data structures (Block, Tx, Address)
│   ├── bq-crypto/         # PQC signatures, KDF, encryption, wallet crypto
│   ├── bitquan-consensus/ # Block/tx validation, PoW, ASERT difficulty
│   ├── bitquan-storage/   # Chain store, UTXO index (RocksDB)
│   ├── bitquan-network/   # P2P messaging, peer management
│   ├── bitquan-mempool/   # Transaction pool, prioritization
│   ├── bitquan-rpc/       # JSON-RPC server, JWT auth
│   ├── bitquan-wallet/    # Wallet operations, key management
│   ├── bitquan-node/      # Main binary, miner, metrics
│   ├── bitquan-cli/       # TUI client (ratatui)
│   ├── bq-sdk/            # Developer SDK
│   └── faucet/            # Testnet faucet service
├── config/                # Network configs (mainnet, testnet, devnet)
├── docs/                  # Documentation
├── fuzz/                  # Fuzzing targets
├── monitoring/            # Prometheus + Grafana stack
├── scripts/               # Build, test, audit scripts
└── tests/                 # Integration tests
```

## Using the SDK

```rust
use bq_sdk::wallet::Wallet;
use bq_sdk::address::Address;

// Create wallet
let wallet = Wallet::generate(NetworkId::Testnet)?;

// Get address
let addr = wallet.address()?;
println!("Address: {}", addr);

// Sign transaction
let signed_tx = wallet.sign_transaction(&unsigned_tx, &password)?;

// Verify signature
let valid = bq_sdk::crypto::verify_signature(&tx, &signature, &public_key)?;
```

See `crates/bq-sdk/` for full API documentation.

## Code Style

- `unsafe_code = "forbid"` enforced at workspace level
- Clippy with `-D warnings` (all warnings are errors)
- No `unwrap()` in production code paths
- No `TODO`/`FIXME` in production code
- All public APIs documented

Run `./scripts/pre-commit.sh` before committing.

## Testing

```bash
# Unit tests
cargo test --workspace

# Integration tests
cargo test --test '*' -- --test-threads=1

# Fuzz targets (requires cargo-fuzz)
cd fuzz && cargo fuzz run <target>

# Stress test
cargo run -p bq-stress
```

## Network Identifiers

| Network | ID | P2P Port | RPC Port |
|---------|----|----------|----------|
| Mainnet | `mainnet` | 18444 | 18443 |
| Testnet | `testnet` | 19444 | 19443 |
| Devnet | `devnet` | 18444 | 18332 |
| Regtest | `regtest` | 18445 | 18446 |
