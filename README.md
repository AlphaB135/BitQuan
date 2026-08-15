# BitQuan

BitQuan is a sovereign, proof-of-work Layer-1 blockchain built in Rust, designed for long-term resistance against quantum computing attacks. It uses CRYSTALS-Dilithium5 (NIST FIPS 205 / Level 5) for transaction signatures, SHA-256d for proof-of-work consensus, and the ASERT difficulty adjustment algorithm.

There are no smart contracts, no governance tokens, no DAOs, and no pre-mines. It is designed solely for resilient, verifiable value transfer.

---

## Technical Specifications

- **Consensus**: Proof-of-Work (SHA-256d) with ASERT difficulty adjustment
- **Signature Scheme**: CRYSTALS-Dilithium5 (Lattice-based, NIST Level 5)
- **Public Key Size**: 2,592 bytes
- **Signature Size**: 4,595 bytes
- **Secret Key Size**: 4,864 bytes
- **Block Time Target**: 120 seconds
- **Base Block Size**: 4 MB
- **Total Supply**: 21,000,000 BQ (Fixed hard cap, 18 decimal places, `u128` qbits)
- **Treasury Model**: Nordic dev/treasury split with decaying allocation (BQIP-0004)
- **Mnemonic Standard**: BIP-39 deterministic key derivation (12, 24, and 512-word streams)

---

## Design Rationale and Trade-offs

The primary technical trade-off in BitQuan is signature size versus quantum immunity:

| Metric | Bitcoin (ECDSA) | BitQuan (Dilithium5) | Overhead |
|---|---|---|---|
| Public Key | 33 bytes | 2,592 bytes | ~78x |
| Signature | ~72 bytes | 4,595 bytes | ~63x |
| Secret Key | 32 bytes | 4,864 bytes | ~152x |
| Quantum Resistance | Broken by Shor's Algorithm | Secure (Lattice problem hardness) | NIST Level 5 |

Lattice-based cryptography requires significantly larger keys and signatures. Rather than waiting for a rushed, backwards-incompatible hard fork when quantum hardware matures, BitQuan adopts large post-quantum keys natively at the base layer.

---

## Live Testnet Services

Public testnet infrastructure running on dedicated seed and relay nodes:

- Web Ledger & Explorer: http://140.245.127.249/
- Web Wallet (Dilithium5 + BIP-39): http://140.245.127.249/wallet/
- Faucet: http://140.245.127.249/faucet/
- Security Audit Verification: http://140.245.127.249/session-security-audit.html
- Grafana Telemetry: http://140.245.127.249:3030/

---

## Building from Source

### Prerequisites

- Rust 1.82.0 or newer
- Clang / LLVM (for Dilithium5 C reference bindings)
- Linux (x86_64, aarch64) or macOS

```bash
# Clone the repository
git clone https://github.com/AlphaB135/BitQuan.git
cd BitQuan

# Compile release binaries
cargo build --release

# Run the test suite
cargo test --workspace
```

The compiled binaries will be located in `target/release/`:
- `bitquan-node`: Main node daemon, RPC server, miner, and wallet manager
- `bitquan-cli`: Command-line wallet and RPC client

---

## Running a Node

### Quick Start with Docker Compose

A pre-configured 3-node non-mining mesh is provided. It runs as pure relay and validation daemons with strict CPU (0.25) and memory (512MB) limits:

```bash
docker compose -f docker-compose.cluster.yml up -d
```

### Standalone Node

```bash
./target/release/bitquan-node run \
  --config config/testnet.toml \
  --datadir ~/.bitquan/testnet \
  --rpc-bind 127.0.0.1:19443 \
  --p2p-bind 0.0.0.0:19444
```

---

## Wallet Operations

### 1. Generate a BIP-39 Mnemonic Wallet

```bash
# Generate a standard 12-word recovery phrase
./target/release/bitquan-node wallet-gen-mnemonic --words 12 --password "YourStrongPassword" --show-mnemonic

# Generate a 24-word recovery phrase
./target/release/bitquan-node wallet-gen-mnemonic --words 24 --password "YourStrongPassword" --show-mnemonic
```

### 2. Restore a Wallet from Mnemonic

```bash
./target/release/bitquan-node wallet-from-mnemonic \
  --mnemonic "word1 word2 word3 word4 word5 word6 word7 word8 word9 word10 word11 word12" \
  --password "YourStrongPassword" \
  --output wallet.keystore
```

### 3. Generate Raw Dilithium5 Keypair

```bash
./target/release/bitquan-node wallet-gen \
  --network testnet \
  --algo dilithium5 \
  --password "YourStrongPassword" \
  --output testnet-wallet.keystore
```

---

## Test Suite and Security Hardening

The codebase includes full automated test suites covering consensus arithmetic, network concurrency, mempool state validation, and cryptographic throughput.

To execute all test suites:

```bash
./scripts/run-all-tests.sh
```

### Current Verification Status

1. Consensus Arithmetic: All arithmetic operations on subsidies, fees, and timestamps use checked or saturating math to eliminate integer overflow crashes.
2. P2P Concurrency: Double-checked locks prevent connection limit race conditions (TOCTOU); inbound sync queue is capped at 50 blocks to prevent memory exhaustion.
3. Mempool Validation: Multi-input double-spend checks are verified atomically before committing outpoints to memory.
4. RPC Daemon: Role-based access control with JWT verification. Block generation calls are rate-capped to 100 blocks per request.
5. PQC Performance: CRYSTALS-Dilithium5 signature verification throughput benchmarks at 6,533.6 TPS on modern x86_64 hardware.

---

## Repository Layout

```
BitQuan/
├── crates/
│   ├── consensus/          # ASERT difficulty engine and block validation
│   ├── crypto/             # Dilithium5, Argon2id, and AES-256-GCM
│   ├── mempool/            # Transaction validation and double-spend pool
│   ├── network/            # Async P2P networking (Tokio + Noise protocol)
│   ├── node/               # Reference node daemon and CLI entrypoint
│   ├── rpc/                # JSON-RPC 2.0 engine with JWT auth
│   ├── storage/            # RocksDB key-value persistence layer
│   ├── types/              # Block, transaction, script, and address types
│   ├── wallet/             # BIP-39 mnemonic derivation and keystores
│   ├── bq-sdk/             # Developer SDK
│   └── faucet/             # Testnet faucet microservice
├── config/                 # Network configuration files (testnet.toml)
├── scripts/                # Test orchestrators and API bridge servers
├── docs/                   # Specifications, BQIPs, and test matrices
└── docker-compose.cluster.yml # Multi-node containerized testnet mesh
```

---

## Releases and Downloads

Pre-compiled release tarballs and SHA256 verification hashes for Linux x86_64 are available on the GitHub Releases page:

- Release: https://github.com/AlphaB135/BitQuan/releases/tag/v0.1.0-testnet

---

## License

This project is licensed under the Apache License 2.0. See the `LICENSE` file for details.
