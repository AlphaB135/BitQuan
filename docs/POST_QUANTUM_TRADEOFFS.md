# Post-Quantum Cryptography Trade-offs

## Executive Summary

BitQuan uses **Dilithium5**, a NIST-standardized post-quantum signature scheme. This provides quantum resistance against future attacks from sufficiently powerful quantum computers running Shor's algorithm.

**The Cost**: Signatures are ~63x larger than Bitcoin's ECDSA signatures.

## Signature Size Comparison

| Scheme | Signature Size | Public Key Size |
|--------|---------------|-----------------|
| Bitcoin ECDSA (secp256k1) | ~73 bytes | 33 bytes (compressed) |
| BitQuan Dilithium5 | **4,595 bytes** | **1,315 bytes** |

### Breakdown: Dilithium5 (Mode 5)

From `crates/pqc-dilithium-seeded/src/params.rs`:

```rust
// For Mode 5 (highest security):
K = 8, L = 7, OMEGA = 75

SIGNBYTES = SEEDBYTES + L * POLYZ_PACKEDBYTES + POLYVECH_PACKEDBYTES
          = 32 + (7 * 640) + (75 + 8)
          = 32 + 4,480 + 83
          = 4,595 bytes
```

**Comparison**:
- BitQuan signature: 4,595 bytes
- Bitcoin signature: ~73 bytes
- **Ratio: ~63x larger** (close to the 70x claimed in Reddit roast)

## Storage Impact

### Per-Transaction
- Single input (1 signature): ~4.6 KB
- Typical transaction (2-3 inputs): ~9-14 KB
- Bitcoin transaction: ~250-500 bytes

### Per-Block (Assuming 1 MB block size limit)
- Bitcoin: ~4,000 transactions
- BitQuan: ~250-300 transactions (with same block size)

**Note**: BitQuan uses [BQIP-0002](./bqip/bqip-0002.md) (fee market) which defines weight units, not raw bytes. The actual block capacity depends on transaction weight.

## Why Dilithium5?

### Security Levels

NIST Post-Quantum Cryptography Standardization:

| Level | Security Strength | Dilithium Mode | Signature Size |
|-------|------------------|----------------|----------------|
| 1 | AES-128 | Mode 2 | ~2,400 bytes |
| 3 | AES-192 | Mode 3 | ~3,300 bytes |
| 5 | **AES-256** | **Mode 5** | **4,595 bytes** |

**BitQuan Choice**: Mode 5 for maximum security against quantum attacks.

### Alternatives Considered

| Scheme | Quantum Resistant? | Size | Status |
|--------|-------------------|------|--------|
| ECDSA (secp256k1) | ❌ No | 73 bytes | Bitcoin standard |
| Schnorr/Taproot | ❌ No | 64 bytes | Bitcoin upgrade |
| Dilithium2 | ✅ Yes | 2,400 bytes | Less secure |
| Dilithium5 | ✅ **Yes** | **4,595 bytes** | **NIST standard** |
| Falcon-512 | ✅ Yes | ~666 bytes | NIST standard (complex) |
| SPHINCS+ | ✅ Yes | ~8,000 bytes | Stateless (very large) |

### Trade-off Analysis

**Why not Falcon-512?** (Smaller signatures)
- More complex implementation
- Constant-time requirements harder to verify
- Fewer independent implementations
- **Decision**: Security simplicity over size optimization

**Why not SPHINCS+?** (Stateless, but large)
- Signatures are ~2x larger than Dilithium
- Stateless property is interesting for long-term storage
- **Decision**: Size is too large for practical use

## Mitigation Strategies

### 1. Pruning (BQIP-0003 - Proposal)

**State Pruning**: Drop old transaction data after validation
- Keep only block headers and UTXO set
- Reduces storage from terabytes to gigabytes
- Compatible with SPV (Simplified Payment Verification) clients

**Data Requirements** (Mainnet, 1 year):
```
Without pruning:
- 6 blocks/hour × 24 hours × 365 days = 52,560 blocks/year
- Assume 300 KB/block (Dilithium5 signatures)
- 52,560 × 300 KB ≈ 15.8 GB/year

With pruning (UTXO-only):
- UTXO set: ~5-10 GB (depends on adoption)
- Block headers: ~4 MB
- Total: < 20 GB regardless of chain length
```

### 2. Signature Aggregation (Research)

**Future Work**: Investigate aggregation schemes
- MuSig2-style aggregation (adapted for PQ)
- Batch verification for block validation
- Potential 2-3x reduction in effective size

**Timeline**: Phase 9 (Post-Mainnet)

### 3. Block Compression (Research)

**Zstd Compression**:
- Test compression on block data
- Expected: 30-40% size reduction
- Trade-off: CPU overhead vs bandwidth savings

**Compact Block Relay** (BIP-152 style):
- Send "compact blocks" with transaction IDs only
- Peers reconstruct from mempool
- Reduces bandwidth by ~80-90%

## TPS Implications

### Current Design

**Block Time**: 2 minutes (120 seconds)
**Block Size**: 1 MB (soft limit, via weight units)

**Theoretical Max TPS**:
```
Assume 300 KB average block size:
300 KB / 4.6 KB per signature ≈ 65 signatures/block
65 signatures / 120 seconds ≈ 0.5 signatures/second

With optimized transactions (batching, compression):
~100-200 signatures/block
100 / 120 ≈ 0.8 TPS (base layer)
```

**Realistic TPS**: 0.5-1 TPS (layer 1)

### Layer 2 Scaling

BitQuan expects most activity to occur on Layer 2:
- Payment channels ( Lightning Network style)
- Sidechains with smaller signatures
- Rollups with batched validity proofs

**Target**: 1,000+ TPS via Layer 2 solutions

## Honest Assessment

### What BitQuan Does Well
1. **Quantum Security**: Dilithium5 is NIST-standardized and well-audited
2. **Simplicity**: Clean implementation, fewer moving parts
3. **Long-term Viability**: No need for hard forks to fix quantum vulnerabilities

### Current Limitations
1. **TPS**: < 1 TPS on layer 1 (by design)
2. **Storage**: Requires pruning for full nodes
3. **Adoption Barrier**: Large transactions may deter initial users

### Comparison with Other Blockchains

| Feature | Bitcoin | Ethereum | QRL | BitQuan |
|---------|---------|----------|-----|---------|
| **Signature Scheme** | ECDSA/secp256k1 | ECDSA + ECOTS | XMSS (Lattice) | Dilithium5 |
| **Quantum Resistant** | ❌ No | ❌ No (yet) | ✅ Yes | ✅ Yes |
| **Signature Size** | 71-73 bytes | 65-73 bytes | ~3,000 bytes | 4,595 bytes |
| **Avg Transaction Size** | ~250 bytes | ~200 bytes | ~5,000 bytes | ~7,500 bytes |
| **TPS (Layer 1)** | ~7 | ~15-30 | ~1-2 | ~0.5-1 |
| **Block Time** | 10 min | 12 sec | 15 sec | 2 min |
| **Consensus** | PoW (SHA-256) | PoS | PoW (RandomX) | PoW (SHA-256d) |

#### Bitcoin
- **Pros**: Largest network, most battle-tested, high liquidity
- **Cons**: Vulnerable to quantum attacks via Shor's algorithm
- **Future**: May require hard fork to post-quantum signatures

#### Ethereum
- **Pros**: Smart contracts, large ecosystem, active development
- **Cons**: Also vulnerable to quantum attacks; transition to PoS doesn't solve this
- **Future**: Account abstraction may enable PQ upgrades

#### QRL (Quantum Resistant Ledger)
- **Pros**: First-mover in quantum resistance, uses XMSS signatures
- **Cons**: Smaller network, less adoption, XMSS is stateful (complex key management)
- **Signature Approach**: XMSS (eXtended Merkle Signature Scheme) - requires state tracking

#### BitQuan
- **Pros**: NIST-standardized Dilithium5 (stateless), simpler key management than QRL
- **Cons**: Largest signatures among PQ chains, lowest TPS
- **Trade-off**: Maximum security (Mode 5) over efficiency

### Key Takeaway
BitQuan prioritizes **future-proof security** over **present efficiency**. While Bitcoin and Ethereum offer better TPS and smaller transactions today, they face existential risk from quantum computers. BitQuan accepts the "post-quantum tax" (30x larger transactions) as the cost of long-term viability.

### Roadmap to Improvement
| Phase | Focus | TPS Target | Storage |
|-------|-------|------------|---------|
| Phase 1-6 (Current) | Core functionality | < 1 TPS | Pruning (BQIP-0003) |
| Phase 7 | Compact block relay | 2-3 TPS | -20% bandwidth |
| Phase 8 | Signature aggregation | 5-10 TPS | -30% signatures |
| Phase 9 | Layer 2 protocols | 1,000+ TPS | Minimal |

## Conclusion

**The "Post-Quantum Tax" is Real**: BitQuan transactions are 63x larger than Bitcoin's.

**This is a Deliberate Trade-off**:
- ✅ Quantum security today (vs emergency hard fork later)
- ✅ NIST-standardized algorithm (vs experimental schemes)
- ✅ Conservative security level (Mode 5 vs Mode 2)
- ❌ Larger transactions (4.6 KB vs 73 bytes)
- ❌ Lower layer 1 TPS (< 1 vs ~7)
- ❌ Higher storage requirements (mitigated via pruning)

**Philosophy**: Security > Efficiency. It's easier to optimize later than to fix a broken security model.

---

**Related Documents**:
- [BQIP-0002: Fee Market](./bqip/bqip-0002.md)
- [BQIP-0003: Pruning Strategy](./bqip/bqip-0003.md) (proposal)
- [Architecture Overview](./architecture/index.md)

**Updated**: 2026-03-17
**Author**: BitQuan Core Team
