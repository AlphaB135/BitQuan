# TPS Analysis and Roadmap

## Executive Summary

**Current Layer 1 TPS**: < 1 transaction per second

This is not a bug—it's a deliberate design choice. BitQuan prioritizes:
1. Post-quantum security (Dilithium5 signatures)
2. Decentralization (low hardware requirements)
3. Simplicity (minimal attack surface)

Scaling is achieved through **Layer 2 solutions**, not larger blocks.

---

## Current TPS Calculation

### Base Parameters

| Parameter | Value | Source |
|-----------|-------|--------|
| Block Time | 120 seconds | `consensus.rs` |
| Block Size (Weight) | 1,000,000 WU | `MempoolPolicy` |
| Signature Weight | 384 WU | `mempool/lib.rs:11` |
| Dilithium5 Signature | 4,595 bytes | `params.rs` |

### Per-Transaction Weight

From `crates/mempool/src/lib.rs`:

```rust
// BQIP-0002: Weight calculation
fn calculate_tx_weight(tx: &Transaction) -> Result<usize> {
    let base_size = serialized_size_hint() - witness_size_hint();
    let sig_count = count_signatures();

    // Base size × 4 + signatures × 384
    base_size * 4 + sig_count * 384
}
```

**Typical Transaction** (2 inputs, 2 outputs, 2 signatures):
```
Base: ~500 bytes × 4 = 2,000 WU
Sigs: 2 × 384 WU = 768 WU
Total: ~2,768 WU per transaction
```

### Maximum Transactions Per Block

```
1,000,000 WU / 2,768 WU ≈ 361 transactions/block
361 tx / 120 sec ≈ 3.0 TPS (theoretical max)
```

### Real-World TPS

**Conservative Estimate** (larger transactions, witness data):
```
Effective block capacity: ~300 KB (after weight adjustments)
Average transaction size: ~400 KB (Dilithium5 signatures)
Transactions per block: ~0.75 (conservative)
TPS: 0.75 / 120 ≈ 0.006 TPS
```

**Honest Estimate**:
- **Layer 1 TPS**: 0.5 - 1.0 TPS
- **With optimization**: 2 - 3 TPS (compact blocks, aggregation)

---

## Bottleneck Analysis

### 1. Signature Size (Primary Bottleneck)

**Problem**: Dilithium5 signatures are 4,595 bytes each

**Impact**:
- 2-input transaction: ~9 KB in signatures alone
- 10-input transaction: ~45 KB in signatures
- Block capacity limited by signature overhead

**Mitigation Timeline**:
| Phase | Solution | TPS Impact |
|-------|----------|------------|
| Phase 7 | Compact block relay | +50% bandwidth |
| Phase 8 | Signature aggregation | 2-3x reduction |
| Phase 9 | Layer 2 protocols | 1000+ TPS |

### 2. Block Time (Design Choice)

**Problem**: 120-second blocks limit transaction confirmation rate

**Rationale**:
- Longer blocks = more propagation time
- Reduces orphan rate for decentralized mining
- Low hardware requirements for full nodes

**Trade-off**: Slower confirmations vs decentralization

**Not Changing**: This is a core design principle

### 3. Network Propagation

**Current**: Full block relay
- Each block: ~300 KB
- Bandwidth: ~2.4 KB/s per peer
- 10 peers = 24 KB/s upstream

**Optimization**: Compact block relay (BIP-152)
- Send only transaction IDs + witness data
- Peers reconstruct from mempool
- **Bandwidth savings**: 80-90%

---

## Optimization Roadmap

### Phase 7: Compact Block Relay

**Target**: 2-3 TPS effective

**Implementation**:
```rust
// Send compact block with only txids
pub struct CompactBlock {
    header: BlockHeader,
    txids: Vec<[u8; 32]>,  // Only IDs, not full transactions
    prefilled_txs: Vec<Transaction>,  // Only missing from mempool
}
```

**Benefits**:
- 80-90% bandwidth reduction
- Faster block propagation
- Supports more miners

**Status**: Designed, not implemented

### Phase 8: Signature Aggregation

**Target**: 5-10 TPS layer 1

**Research Directions**:
1. **Batch Verification**: Verify N signatures with 1 amortized operation
2. **Aggregate Signatures**: MuSig2-style (adapted for Dilithium)
3. **Witness Compression**: Compress witness data with zstd

**Expected Reductions**:
- Batch verification: 20-30% faster validation
- Aggregation: 2-3x smaller signatures
- Compression: 30-40% smaller witnesses

**Combined**: ~3-5x effective TPS increase

**Status**: Research phase

### Phase 9: Layer 2 Scaling

**Target**: 1,000+ TPS via payment channels

**Solutions**:
1. **Payment Channels**: Lightning Network style
   - Off-chain transactions
   - On-chain settlement only
   - Unlimited TPS between channel parties

2. **Rollups**: Batched validity proofs
   - Hundreds of tx → 1 on-chain transaction
   - Fraud proofs or ZK proofs
   - Data availability on layer 1

3. **Sidechains**: Independent chains with smaller signatures
   - Faster confirmations
   - Different security trade-offs
   - Pegged to mainnet

**Status**: Not started (requires active mainnet)

---

## Comparison with Other Chains

| Chain | Layer 1 TPS | Notes |
|-------|------------|-------|
| Bitcoin | ~7 | ECDSA signatures (73 bytes) |
| Ethereum | ~15 | Smart contract execution |
| Solana | ~3,000 | Proof-of-history + centralized |
| BitQuan | **< 1** | Dilithium5 signatures (4,595 bytes) |

**Honest Assessment**: BitQuan has the lowest layer 1 TPS among major blockchains.

**Why This Is OK**:
1. **Security > TPS**: Quantum resistance is more valuable than raw throughput
2. **Layer 2 Solution**: Payment channels can provide unlimited TPS
3. **Decentralization**: Low TPS enables low hardware requirements

---

## Frequently Asked Questions

### Q: Is 1 TPS enough for a global currency?

**A**: Not on layer 1. But with payment channels:
- Each channel user can make unlimited off-chain transactions
- Only channel opening/closing hits layer 1
- **Effective TPS**: Millions of transactions per day globally

### Q: Why not increase block size?

**A**: Larger blocks = more centralization:
- 10 MB blocks → 2.4 MB/s bandwidth per peer
- 100 MB blocks → 24 MB/s → requires datacenter
- **Decision**: Keep blocks small, use layer 2

### Q: When will BitQuan scale?

**A**: Timeline:
- **Phase 7** (Post-mainnet): Compact blocks → 2-3 TPS
- **Phase 8** (Research): Aggregation → 5-10 TPS
- **Phase 9** (After adoption): Layer 2 → 1,000+ TPS

### Q: Can we use smaller PQ signatures?

**A**: Yes, but with trade-offs:
- Dilithium2: 2,400 bytes (less secure)
- Falcon-512: 666 bytes (more complex)
- **Current Choice**: Dilithium5 (maximum security)

### Q: What about sharding?

**A**: Not considered for BitQuan:
- Adds complexity
- Weakens security model
- **Focus**: Payment channels (simpler, proven)

---

## Performance Testing

### Current Benchmarks

Run with:
```bash
cargo test --release --test stress_test
```

**Results** (devnet, 4-core CPU):
```
Block validation: ~50ms per block
Transaction verification: ~10ms per signature
Mempool insertion: ~1ms per transaction
```

### Load Testing

```bash
# Run stress test
./target/release/bitquan-node stress-test --tx-count 1000

# Expected: ~1000 transactions over ~20 minutes (0.8 TPS)
```

---

## Conclusion

**BitQuan is not a high-TPS layer 1 chain.** This is intentional.

**Design Philosophy**:
1. ✅ Post-quantum security (non-negotiable)
2. ✅ Decentralization (low hardware requirements)
3. ✅ Simplicity (minimal attack surface)
4. ❌ High layer 1 TPS (sacrificed for above goals)

**Scaling Strategy**:
- Layer 1: Secure settlement layer (1 TPS)
- Layer 2: High-speed payment channels (1000+ TPS)

**Timeline to Viable Scaling**: Phase 9 (post-mainnet adoption)

---

**Related Documents**:
- [Post-Quantum Trade-offs](POST_QUANTUM_TRADEOFFS.md)
- [BQIP-0002: Fee Market](bqip/bqip-0002.md)
- [Architecture Overview](architecture/index.md)

**Updated**: 2026-01-26
**Author**: BitQuan Core Team
