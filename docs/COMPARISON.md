# BitQuan vs Other Blockchains

**Last Updated**: 2026-03-27

## Feature Comparison

| Feature | BitQuan | Bitcoin | Monero | Kaspa |
|---------|---------|---------|--------|-------|
| **Consensus** | PoW (SHA-256d + hybrid) | PoW (SHA-256d) | PoW (RandomX) | PoW (kHeavyHash) |
| **Signatures** | CRYSTALS-Dilithium5 | ECDSA (secp256k1) | Ed25519 + MLSAG | ECDSA (secp256k1) |
| **Quantum Resistance** | Native (NIST PQC) | None | None | None |
| **UTXO Model** | Yes | Yes | No (ring signatures) | UTXO |
| **Block Time** | ~10 min (target) | ~10 min | ~2 min | ~1 sec |
| **Difficulty Adjustment** | ASERT + BurstGuard | DAA (every 2016 blocks) | LWMA | BGRT |
| **Smart Contracts** | No | No (Script limited) | No | No |
| **Privacy** | Transparent | Transparent | Built-in (Ring CT) | Transparent |
| **Language** | Rust | C++ | C++ | Go + Rust |
| **Max Supply** | 21M BQ | 21M BTC | Unlimited (tail) | Unlimited |
| **Halving** | 210K blocks | 210K blocks | Smooth emission | No halving |

## Security Model Comparison

| Aspect | BitQuan | Bitcoin | Monero | Kaspa |
|--------|---------|---------|--------|-------|
| **Signature Security** | 256-bit (Dilithium5, PQ level 5) | 128-bit (ECDSA) | 128-bit (Ed25519) | 128-bit (ECDSA) |
| **Quantum Attack** | Requires ~10,000+ qubits | Requires ~4,000 qubits | Requires ~4,000 qubits | Requires ~4,000 qubits |
| **KDF** | Argon2id (memory-hard) | SHA-512 (CPU-only) | No wallet KDF standard | N/A |
| **Memory Safety** | Rust (no unsafe) | C++ (manual) | C++ (manual) | Go + Rust |
| **Formal Verification** | No | No | No | No |
| **Third-Party Audit** | Pending | Multiple (trailofbits, etc.) | Multiple | Quarkslab |

## Performance Comparison (Theoretical)

| Metric | BitQuan | Bitcoin | Monero | Kaspa |
|--------|---------|---------|--------|-------|
| **Theoretical TPS** | ~7 | ~7 | ~5 | ~32 (with pruning) |
| **Block Size** | 4MW (weighted) | 4MW | Dynamic (penalty) | Dynamic |
| **Signature Size** | ~2.4 KB (Dilithium5) | ~72 B (ECDSA) | ~2.5 KB (Ring CT) | ~72 B |
| **Finality** | Probabilistic (6 conf) | Probabilistic (6 conf) | Probabilistic (10 conf) | Probabilistic |
| **Sync Time (full)** | TBD | ~2 days | ~3 days | ~hours (with pruning) |

## BitQuan's Unique Advantages

1. **Post-Quantum by Default**: First blockchain using NIST-standardized Dilithium5 signatures in consensus. No migration needed when quantum computers arrive.

2. **Memory-Safe Codebase**: Written in Rust with `unsafe_code = "forbid"` at workspace level. No buffer overflows, no use-after-free.

3. **Stronger Wallet Encryption**: Argon2id with 256 MiB memory + AES-256-GCM + memory locking + brute-force protection. Bitcoin uses SHA-512 only.

4. **BurstGuard**: Novel spike protection in ASERT difficulty adjustment prevents difficulty manipulation through burst mining.

## Trade-offs

| Trade-off | Impact |
|-----------|--------|
| Large signature size (~2.4 KB) | More data per tx, lower tx density per block |
| No privacy features | All transactions are transparent (unlike Monero) |
| Single mining algorithm primary | SHA-256d means Bitcoin ASIC miners dominate (mitigated by hybrid mode on testnet) |
| Young project | No battle-tested track record (unlike Bitcoin's 15+ years) |
| No smart contracts | Limited programmability compared to Ethereum-like chains |

## When to Choose BitQuan

- You need long-term quantum resistance (50+ year security horizon)
- You want a UTXO model with strong wallet encryption
- You value memory-safe implementation (Rust, no unsafe)
- You need a simple, Bitcoin-like protocol without complexity

## When to Choose Alternatives

- **Bitcoin**: Maximum decentralization, proven track record, largest network effect
- **Monero**: Privacy is the primary requirement
- **Kaspa**: High throughput with fast confirmations needed
- **Ethereum**: Smart contracts and DeFi applications needed
