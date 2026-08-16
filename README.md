# BitQuan — Post-Quantum Layer-1 Blockchain

		"Talk is cheap. Show me the code."
		— Linus Torvalds

BitQuan is a proof-of-work blockchain that doesn't fuck around with quantum vulnerability. While everyone else is busy adding more buzzwords to their whitepapers, we're using **CRYSTALS-Dilithium5** (NIST FIPS 205 / Level 5) because when Shor's algorithm breaks ECDSA, you'll wish you had.

No smart contracts. No DAOs. No governance tokens. No pre-mine. No bullshit.

Just a blockchain that does one thing: **verifiable value transfer that survives quantum computing**.

---

## What's Different?

Most blockchains will tell you they're "quantum-resistant" while still using ECDSA. We actually did the work:

| Metric | Bitcoin (ECDSA) | BitQuan (Dilithium5) | Trade-off |
|---|---|---|---|
| Public Key | 33 bytes | 2,592 bytes | ~78x larger |
| Signature | ~72 bytes | 4,595 bytes | ~63x larger |
| Secret Key | 32 bytes | 4,864 bytes | ~152x larger |
| Quantum Resistance | **Broken by Shor's Algorithm** | **Secure (NIST Level 5)** | You choose |

Yes, signatures are huge. Yes, blocks are bigger. But when quantum computers mature, you won't need a hard fork that breaks every wallet, exchange, and smart contract ever deployed.

We pay the cost **now** so you don't pay it **later** when it's too late.

---

## Technical Specs (The Stuff That Actually Matters)

- **Consensus**: Proof-of-Work (SHA-256d) + ASERT difficulty adjustment
- **Signature Scheme**: CRYSTALS-Dilithium5 (lattice-based, NIST Level 5)
- **Block Time**: 120 seconds (2 minutes)
- **Block Size**: 4 MB base (weight-based like Bitcoin SegWit)
- **Total Supply**: 21,000,000 BQ (fixed hard cap, no inflation)
- **Decimal Places**: 18 (u128 qbits — no floating-point bullshit)
- **Mnemonic**: BIP-39 (12/24 words — your grandma can back it up)

---

## Security Status

This codebase has been through **3 rounds of penetration testing** with 27 vulnerabilities found and fixed:
- **Round 1**: 15 bugs (TOCTOU races, memory exhaustion, integer overflows)
- **Round 2**: 10 NEW bugs (wallet cache race, CORS exposure, eclipse attacks)
- **Round 3**: 2 MORE bugs (atomic ordering, integer overflow in cache)

Then we attacked it with **real-world CVE techniques** from Bitcoin Core and Ethereum:
- CVE-2025-54604: Resource exhaustion → ✅ DEFENDED
- CVE-2026-34219: Integer overflow → ✅ DEFENDED
- Eclipse attacks → ✅ DEFENDED (subnet diversity enforced)
- Rate limit bypass → ✅ DEFENDED (50/50 concurrent requests blocked)
- Serialization bomb → ✅ DEFENDED (nested JSON rejected)

**Live Attack Results**:
- 10,000+ attack requests
- 60 seconds maximum load (50 workers)
- RPC fuzzing with random payloads
- **Result**: Node survived. No crashes. No memory leaks.

See detailed reports:
- `REAL_WORLD_ATTACK_REPORT.md` — CVE-based attacks
- `LIVE_ATTACK_RESULTS.md` — Stress testing results
- `TIMING_ATTACK_ANALYSIS.md` — Why constant-time validation isn't needed

---

## Building from Source

If you can't build from source, you can't verify anything. Here's how:

```bash
# Prerequisites: Rust 1.82.0+, Clang/LLVM
git clone https://github.com/AlphaB135/BitQuan.git
cd BitQuan

# Build release binaries
cargo build --release

# Run the full test suite
cargo test --workspace

# Binaries are in target/release/:
# - bitquan-node: Full node + wallet + miner
# - bitquan-cli: Command-line wallet
```

If the build fails, you probably need to install Clang. If the tests fail, open an issue with the full output.

---

## Running a Node

### Docker (Easiest)

3-node non-mining mesh with strict resource limits:

```bash
docker compose -f docker-compose.cluster.yml up -d
```

### Bare Metal (Fastest)

```bash
./target/release/bitquan-node run \
  --config config/testnet.toml \
  --datadir ~/.bitquan/testnet \
  --rpc-bind 127.0.0.1:19443 \
  --p2p-bind 0.0.0.0:19444
```

---

## Wallet Operations

### Generate BIP-39 Mnemonic

```bash
# 12-word recovery phrase
./target/release/bitquan-node wallet-gen-mnemonic \
  --words 12 \
  --password "YourStrongPassword" \
  --show-mnemonic

# 24-word recovery phrase (more secure)
./target/release/bitquan-node wallet-gen-mnemonic \
  --words 24 \
  --password "YourStrongPassword" \
  --show-mnemonic
```

### Restore from Mnemonic

```bash
./target/release/bitquan-node wallet-from-mnemonic \
  --mnemonic "word1 word2 ... word12" \
  --password "YourStrongPassword" \
  --output wallet.keystore
```

### Generate Raw Dilithium5 Keypair

```bash
./target/release/bitquan-node wallet-gen \
  --network testnet \
  --algo dilithium5 \
  --password "YourStrongPassword" \
  --output testnet-wallet.keystore
```

**IMPORTANT**: Keystore files contain encrypted private keys. Back them up. Don't commit them to git.

---

## Live Testnet

Public infrastructure running on Oracle Cloud:

- **Web Explorer**: http://140.245.127.249/
- **Web Wallet**: http://140.245.127.249/wallet/
- **Faucet**: http://140.245.127.249/faucet/
- **Security Audit**: http://140.245.127.249/session-security-audit.html
- **Grafana Telemetry**: http://140.245.127.249:3030/

---

## Repository Structure

```
BitQuan/
├── crates/
│   ├── consensus/          # ASERT + block validation
│   ├── crypto/             # Dilithium5 + Argon2id + AES-256-GCM
│   ├── mempool/            # Double-spend prevention
│   ├── network/            # P2P (Tokio + Noise protocol)
│   ├── node/               # Node daemon + CLI
│   ├── rpc/                # JSON-RPC 2.0 + JWT auth
│   ├── storage/            # RocksDB persistence
│   ├── types/              # Core data structures
│   ├── wallet/             # BIP-39 + keystores
│   └── faucet/             # Testnet faucet
├── config/                 # Network configs
├── scripts/                # Test automation
├── docs/                   # Specs + BQIPs
└── docker-compose.cluster.yml
```

---

## Testing

We don't do "manual testing" here. Everything is automated:

```bash
# Run all tests
./scripts/run-all-tests.sh

# Or run specific test suites
cargo test --package bitquan-consensus
cargo test --package bitquan-network
cargo test --package bitquan-mempool
```

**Current Test Coverage**:
- Consensus arithmetic (checked/saturating math)
- P2P concurrency (TOCTOU prevention)
- Mempool double-spend atomicity
- RPC rate limiting + JWT auth
- PQC signature verification (6,533.6 TPS benchmark)

---

## What We DON'T Have (And Why)

**No Smart Contracts**: Smart contracts on Layer-1 are a security nightmare. Every bug is permanent. Every exploit is irreversible. If you want programmability, build Layer-2.

**No Pre-mine**: The creator doesn't get free coins. Mine them like everyone else.

**No Governance Tokens**: Governance tokens are just pre-mines with extra steps.

**No DAOs**: If you need a DAO, deploy it on Layer-2 or another chain. BitQuan is for value transfer, not on-chain governance theater.

**No ICO/IEO/IDO**: We're not selling you tokens. Mine them or buy them from someone who did.

---

## Known Limitations

We're honest about what we can't fix:

1. **51% Attack**: Inherent to all PoW chains. Mitigated by network size.
2. **Sybil Attack**: Mitigated with subnet diversity (max 8 peers per /24 or /48).
3. **Quantum Attacks on PoW**: SHA-256d is vulnerable to Grover's algorithm (but needs a LOT of qubits).
4. **Large Signatures**: Dilithium5 signatures are 4,595 bytes. That's the trade-off.

---

## Contributing

Read `CONTRIBUTING.md` first. PRs without tests will be rejected. Code without comments explaining **why** (not what) will be rejected. Patches that break existing tests without justification will be rejected.

**Coding Standards**:
- Rust 1.82.0+ (2021 edition)
- `cargo fmt` before commit
- `cargo clippy` must pass
- No `unwrap()` or `expect()` in production code
- Saturating/checked arithmetic for all math
- Security-sensitive code needs comments explaining threat model

---

## License

Apache License 2.0 — because MIT is too permissive and GPL is too restrictive.

See `LICENSE` file.

---

## Contact

- **Issues**: GitHub Issues (preferred)
- **Security**: See `SECURITY.md`
- **Releases**: https://github.com/AlphaB135/BitQuan/releases

---

## Final Thoughts

Most blockchain projects are solving problems that don't exist while ignoring problems that do.

Quantum computers **will** break ECDSA. It's not an "if," it's a "when."

You can either:
1. Wait for it to happen and panic-migrate later
2. Build with post-quantum crypto from day one

We chose #2. The code is open source. Audit it yourself.

If you find bugs, report them. If you find vulnerabilities, report them privately (see `SECURITY.md`).

And if you think large signatures are a problem, remember: **broken cryptography is a bigger problem**.

---

*"Code doesn't lie. Whitepapers do."*
