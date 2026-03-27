# Third-Party Security Audit Package

**Last Updated**: 2026-03-27
**Version**: v0.1.0-pre
**Status**: Audit-ready

---

## 1. Project Overview

BitQuan is a post-quantum blockchain using UTXO model, Proof-of-Work consensus, and CRYSTALS-Dilithium3 digital signatures for 50+ year quantum resistance. Written in Rust with `unsafe_code = "forbid"` enforced at workspace level.

**Key differentiator**: First blockchain to use NIST-standardized post-quantum signatures (Dilithium3) in production consensus.

## 2. Architecture

```
┌─────────────────────────────────────────────┐
│                  bitquan-node               │
│         (orchestrator + CLI binary)          │
├───────┬──────┬──────┬──────┬──────┬────────┤
│ types │crypto│consen│store │ net  │  rpc   │
│       │      │sus   │      │      │        │
│  data │  PQC │ rules│ chain│  P2P │ JSON   │
│ structs│ KDF │valid.│ UTXO │ msg  │ server │
│  ser. │ encr │ PoW  │ blocs│ proto│ wallet │
├───────┴──────┴──────┴──────┴──────┴────────┤
│              mempool  │  wallet  │ faucet   │
└─────────────────────────────────────────────┘
```

**Dependency flow** (unidirectional, no circular deps):
```
types <- crypto <- consensus <- mempool <- node
types <- storage <- consensus <- rpc <- node
```

**Crates** (14 total):

| Crate | Purpose | LOC (approx) |
|-------|---------|-------------|
| `bitquan-types` | Core data structures, serialization | ~2,000 |
| `bq-crypto` | PQC signatures, KDF, encryption, wallet crypto | ~5,000 |
| `bitquan-consensus` | Block/tx validation, PoW, difficulty | ~4,000 |
| `bitquan-storage` | Chain store, UTXO index, block persistence | ~3,000 |
| `bitquan-network` | P2P messaging, peer management | ~3,000 |
| `bitquan-mempool` | Transaction pool, prioritization | ~1,500 |
| `bitquan-rpc` | JSON-RPC server, JWT auth | ~2,500 |
| `bitquan-wallet` | Wallet operations, key management | ~2,000 |
| `bitquan-node` | Main binary, miner, metrics | ~5,000 |
| `bitquan-cli` | TUI client (ratatui) | ~1,500 |
| `bq-sdk` | Developer SDK | ~500 |
| `faucet` | Testnet faucet service | ~500 |
| `bq-preflight` | Release preflight checks | ~300 |
| `bq-stress` | Load testing tool | ~500 |

## 3. Threat Surface

### High Priority (Direct Attack Vectors)

| Surface | Location | Attack Types |
|---------|----------|-------------|
| P2P message parsing | `crates/network/src/lib.rs` | Malformed messages, DoS, protocol exploitation |
| Block/tx validation | `crates/consensus/src/lib.rs` | Invalid blocks, double-spend, overflow |
| RPC endpoint | `crates/rpc/src/` | Injection, auth bypass, info disclosure |
| JWT authentication | `crates/rpc/src/auth.rs` | Token forgery, timing attacks |
| Wallet encryption | `crates/crypto/src/wallet/` | Brute-force, side-channel, memory disclosure |
| KDF parameters | `crates/crypto/src/wallet/kdf.rs` | Weak params, timing oracle |
| Mining/submission | `crates/node/src/block_submit.rs` | Invalid block injection |

### Medium Priority

| Surface | Location | Attack Types |
|---------|----------|-------------|
| Peer management | `crates/network/src/peers.rs` | Eclipse attack, Sybil |
| Mempool | `crates/mempool/src/` | Spam, DoS via tx flood |
| Storage | `crates/storage/src/` | Data corruption, chain manipulation |
| Mnemonic generation | `crates/crypto/src/mnemonic.rs` | Weak entropy |

### Low Priority

| Surface | Location | Attack Types |
|---------|----------|-------------|
| Metrics endpoint | `crates/node/src/metrics.rs` | Info disclosure |
| Faucet | `crates/faucet/src/` | Abuse (testnet only) |
| CLI | `crates/bitquan-cli/` | Input handling |

## 4. Cryptographic Stack

| Component | Algorithm | Library | Standard |
|-----------|-----------|---------|----------|
| Signatures | CRYSTALS-Dilithium3 | `pqc-dilithium-seeded` | NIST FIPS 204 |
| KDF | Argon2id | `argon2` crate | RFC 9106 |
| Encryption | AES-256-GCM | `aes-gcm` crate | NIST SP 800-38D |
| Hashing | SHA-256d | `sha2` crate | NIST FIPS 180-4 |
| Address | blake3 | `blake3` crate | BLAKE3 spec |
| Mnemonic | BIP-39 | `bip39` crate | BIP-39 standard |
| RNG | `getrandom` (OS) | `getrandom` crate | OS CSPRNG |

## 5. What to Focus On

### Critical
1. **Dilithium signature implementation** (`crates/pqc-dilithium-seeded/`) — Custom FFI/seeded implementation. Verify correctness against NIST reference.
2. **Wallet encryption flow** (`crates/crypto/src/wallet/`) — KDF params, constant-time comparisons, memory zeroization.
3. **Consensus validation** (`crates/consensus/src/lib.rs`) — Timestamp validation, difficulty adjustment, block reward calculation.

### Important
4. **P2P network isolation** — Can a malicious peer cause resource exhaustion?
5. **RPC authentication** — JWT implementation, rate limiting, input sanitization.
6. **Mempool behavior under load** — Can it be spammed to DoS the node?

### Nice to Have
7. **Reproducible builds** — Do builds produce identical binaries?
8. **Fuzzing coverage** — Are all parser entry points fuzzed?

## 6. Existing Security Measures

| Measure | Status | Evidence |
|---------|--------|----------|
| `unsafe_code = "forbid"` | Enforced | `Cargo.toml` workspace lints |
| Constant-time crypto comparisons | Done | `subtle::ConstantTimeEq` in crypto crate |
| Memory locking (mlock) | Done | `crates/crypto/src/wallet/encryption.rs` |
| Memory zeroization | Done | `zeroize` crate on all sensitive types |
| Clippy strict mode | Done | `-D warnings` in CI |
| Cargo audit | Done | `.github/workflows/security-scan.yml` |
| Cargo deny | Done | `deny.toml` with license + advisory checks |
| Fuzzing | Done | `fuzz/` directory, 24+ hour campaigns |
| Bug bounty | Active | `docs/security/BUG_BOUNTY.md` |
| Self-audit reports | 20+ files | `docs/security/audit-reports/`, `docs/security/audits/` |

## 7. Build & Test Instructions

```bash
# Prerequisites
rustup default stable
cargo install cargo-audit cargo-deny

# Build
cargo build --release

# Test
cargo test

# Lint
cargo clippy -- -D warnings

# Security audit (all-in-one)
./scripts/audit.sh

# Fuzz targets
cd fuzz && cargo fuzz run <target>
```

## 8. Known Limitations

1. **No formal verification** — Code is tested but not formally verified
2. **Single-node testnet** — No large-scale network testing yet
3. **Benchmark data incomplete** — `docs/benchmarks/` is placeholder
4. **No hardware security module (HSM) support** — Keys stored in software only
5. **PQC hybrid mode not yet implemented** — Pure Dilithium, no ECDSA fallback

## 9. Contact & Disclosure

- **Security contact**: bitquan.dev@proton.me
- **Bug bounty**: See `docs/security/BUG_BOUNTY.md`
- **Coordinated disclosure**: 30-day window per bug bounty policy
- **PGP key**: See `docs/security/keys/`
