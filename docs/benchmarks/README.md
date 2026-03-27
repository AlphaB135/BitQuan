# Benchmark Results

**Last Updated**: 2026-03-27
**Platform**: macOS ARM64 (Apple Silicon), 16 GB RAM
**Rust**: stable (1.79+)
**Build**: `cargo build --release`

---

## Cryptographic Operations

| Operation | Time | Notes |
|-----------|------|-------|
| Dilithium5 Key Generation | ~1.2 ms | One-time per wallet |
| Dilithium5 Sign | ~0.8 ms | Per transaction input |
| Dilithium5 Verify | ~0.5 ms | Per transaction input |
| SHA-256d (single hash) | ~0.001 ms | Block/tx hashing |
| Argon2id KDF (256 MiB) | ~1.5 s | Wallet encryption (tuned) |
| AES-256-GCM Encrypt (1 KB) | ~0.005 ms | Wallet data encryption |
| AES-256-GCM Decrypt (1 KB) | ~0.005 ms | Wallet data decryption |
| BIP-39 Mnemonic Generation | ~0.3 ms | Wallet creation |

## Signature Size Comparison

| Algorithm | Public Key | Signature | Combined |
|-----------|-----------|-----------|----------|
| **Dilithium5** | 1,952 B | 2,420 B | 4,372 B |
| ECDSA (secp256k1) | 33 B | 72 B | 105 B |
| Ed25519 | 32 B | 64 B | 96 B |
| **Ratio (Dilithium/ECDSA)** | 59x | 34x | 42x |

> Dilithium5 signatures are ~42x larger than ECDSA. This is the primary trade-off for quantum resistance. TPS is correspondingly lower due to block space consumption.

## Block Validation

| Operation | Time | Notes |
|-----------|------|-------|
| Block header validation | ~0.5 ms | Hash check + difficulty + PoW |
| Single transaction validation | ~1.5 ms | Signature verify + UTXO lookup + script |
| Full block (100 tx) | ~150 ms | Sequential validation |
| Full block (1000 tx) | ~1.5 s | Sequential validation |
| UTXO lookup (indexed) | ~0.01 ms | HashMap-based |
| UTXO lookup (unindexed) | ~5 ms | Linear scan (worst case) |

## Mining Performance

| Algorithm | Hardware | Hash Rate | Notes |
|-----------|----------|-----------|-------|
| SHA-256d | Apple M1 (1 thread) | ~2 MH/s | CPU only |
| SHA-256d | Apple M1 (8 threads) | ~14 MH/s | CPU only |
| SHA-256d | ASIC (Antminer S19) | ~95 TH/s | 6.8Mx faster than CPU |

## Memory Usage

| Component | Idle | Under Load | Notes |
|-----------|------|-----------|-------|
| Node (no mining) | ~30 MB | ~80 MB | P2P + RPC + storage |
| Node + Mining (1 thread) | ~35 MB | ~120 MB | SHA-256d only |
| Memory-mapped chain data | Variable | ~200 MB / 10K blocks | mmap storage backend |

## Build Performance

| Metric | Time |
|--------|------|
| `cargo build` (debug) | ~45 s |
| `cargo build --release` | ~3 min |
| `cargo test --workspace` | ~30 s |
| `cargo clippy --workspace` | ~15 s |
| Binary size (release) | ~8 MB |
| Binary size (stripped) | ~5 MB |

## How to Run Benchmarks

```bash
# Run all benchmarks
cargo bench

# Run specific benchmark
cargo bench -- "block_validation"

# Run with profiling
cargo bench -- --profile-time=10
```

## Notes

- All cryptographic benchmarks measured with `criterion` on Apple Silicon
- Block validation times are sequential; parallel validation is not yet implemented
- Mining benchmarks are CPU-only; GPU mining is not supported
- Memory usage measured with Activity Monitor on idle and under 100 tx/block load
- Build times measured on cold build (no cache)
