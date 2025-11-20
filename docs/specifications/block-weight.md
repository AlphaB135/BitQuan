# Block Weight Specification

Version: 0.0.1-alpha  
Status: Draft

## Overview

Block weight accounts for the increased size of post-quantum signatures,
ensuring fair block capacity and fee market dynamics.

## Motivation

Post-quantum signatures (Dilithium3) are ~3,000 bytes each, compared to
~70 bytes for ECDSA. Without weight adjustment, PQC transactions would
dominate block space and create unfair fee dynamics.

## Weight Formula

```
block_weight = base_size + (signature_count * SIGNATURE_WEIGHT)

where:
  base_size = size of block without witness data (bytes)
  signature_count = total signatures in all transactions
  SIGNATURE_WEIGHT = 384 weight units (WU)
```

### Constants

```rust
const SIGNATURE_WEIGHT: usize = 384;        // WU per signature
const MAX_BLOCK_WEIGHT: usize = 4_000_000;  // WU
const WITNESS_SCALE_FACTOR: usize = 4;      // Bitcoin compatibility
```

## Rationale

### SIGNATURE_WEIGHT = 384

Dilithium3 signatures are ~3,000 bytes. With witness scale factor of 4:
```
3000 / 4 = 750 WU (raw witness cost)
```

We use 384 WU (50% discount) to encourage PQC adoption while maintaining
block space fairness.

Comparison:
- Bitcoin ECDSA: ~70 bytes / 4 = 17.5 WU
- Dilithium3: ~3000 bytes, discounted to 384 WU
- Ratio: 384 / 17.5 = ~22x (reasonable for PQC)

### MAX_BLOCK_WEIGHT = 4,000,000

Matches Bitcoin's block weight limit for ecosystem familiarity.

Maximum transactions per block (estimated):
```
Average tx: 2 inputs, 2 outputs, 2 signatures
Base size: ~200 bytes
Weight: 200 + (2 * 384) = 968 WU
Max txs: 4,000,000 / 968 = ~4,132 transactions
```

## Weight Calculation

### Transaction Weight

```rust
fn tx_weight(tx: &Transaction) -> usize {
    let base_size = tx.serialize_without_witness().len();
    let sig_count = tx.witnesses.iter()
        .map(|w| w.signatures.len())
        .sum();
    
    base_size * WITNESS_SCALE_FACTOR + sig_count * SIGNATURE_WEIGHT
}
```

### Block Weight

```rust
fn block_weight(block: &Block) -> usize {
    block.transactions.iter()
        .map(|tx| tx_weight(tx))
        .sum()
}
```

## Validation Rules

1. Block weight MUST be <= MAX_BLOCK_WEIGHT
2. Each transaction weight MUST be <= MAX_BLOCK_WEIGHT
3. Coinbase transaction counts toward weight
4. Empty blocks (coinbase only) are valid

## Mempool Policy

### Fee-Per-Weight Ordering

Transactions in mempool ordered by:
```
priority = fee / tx_weight(tx)
```

Higher fee-per-weight = higher priority.

### Minimum Fee Rate

Default: 1 qbit per weight unit (1 qbit/WU)

```
min_fee = tx_weight(tx) * 1 qbit
```

### Replacement Policy

Transaction replacement (RBF) requires:
1. New tx fee >= old tx fee + (new_weight * 1 qbit/WU)
2. New tx weight <= old tx weight * 2

### Eviction Policy

When mempool full (300 MB default):
1. Remove lowest fee-per-weight transactions
2. Maintain minimum fee rate floor
3. Never evict transactions with >= 10 qbit/WU

## Block Template Construction

Miners select transactions by:

```python
def select_transactions(mempool, max_weight):
    selected = []
    total_weight = 0
    total_fee = 0
    
    # Sort by fee per weight (descending)
    sorted_txs = sorted(mempool, 
                       key=lambda tx: tx.fee / tx.weight,
                       reverse=True)
    
    for tx in sorted_txs:
        if total_weight + tx.weight <= max_weight:
            selected.append(tx)
            total_weight += tx.weight
            total_fee += tx.fee
    
    return selected, total_fee
```

## Examples

### Example 1: Simple Transfer

```
Transaction:
- 1 input (Dilithium3 signature)
- 2 outputs
- Base size: 150 bytes
- Signatures: 1

Weight calculation:
weight = (150 * 4) + (1 * 384)
       = 600 + 384
       = 984 WU

Min fee: 984 qbits (0.00000984 BQ)
```

### Example 2: Multi-Input Transaction

```
Transaction:
- 3 inputs (3 Dilithium3 signatures)
- 2 outputs
- Base size: 300 bytes
- Signatures: 3

Weight calculation:
weight = (300 * 4) + (3 * 384)
       = 1200 + 1152
       = 2352 WU

Min fee: 2,352 qbits (0.00002352 BQ)
```

### Example 3: Maximum Block

```
Block with 4,000 txs:
- Each tx: 200 bytes base, 2 signatures
- Each tx weight: (200 * 4) + (2 * 384) = 1,568 WU
- Total weight: 4,000 * 1,568 = 6,272,000 WU

INVALID: Exceeds 4,000,000 WU cap

Maximum transactions:
4,000,000 / 1,568 = ~2,551 transactions
```

## Fee Market Dynamics

### Supply and Demand

Block space supply:
```
blocks_per_day = 144 (10 min blocks)
weight_per_day = 144 * 4,000,000 = 576,000,000 WU
```

Average transaction cost:
```
avg_tx_weight = 1,500 WU
min_daily_txs = 576,000,000 / 1,500 = 384,000 transactions
```

### Fee Estimation

Recommended fee rates:
- Low priority: 1 qbit/WU (next few blocks)
- Medium priority: 5 qbit/WU (next block)
- High priority: 10 qbit/WU (immediate)

## Comparison with Bitcoin

| Metric | Bitcoin | BitQuan |
|--------|---------|---------|
| Block weight limit | 4,000,000 WU | 4,000,000 WU |
| Signature weight | ~17.5 WU | 384 WU |
| Avg tx weight | ~560 WU | ~1,500 WU |
| Max txs/block | ~7,000 | ~2,500 |
| Sig size | ~70 bytes | ~3,000 bytes |

## Implementation Notes

### Weight Caching

Transaction weight should be cached after first calculation:

```rust
struct Transaction {
    // ... fields ...
    #[serde(skip)]
    cached_weight: Option<usize>,
}

impl Transaction {
    fn weight(&mut self) -> usize {
        if let Some(w) = self.cached_weight {
            return w;
        }
        let w = self.calculate_weight();
        self.cached_weight = Some(w);
        w
    }
}
```

### Consensus Critical

Weight calculation is consensus-critical. All implementations must
produce identical results.

Test vectors required for:
- Single signature transaction
- Multi-signature transaction  
- Maximum weight transaction
- Edge cases (empty witness, etc.)

## References

- BIP141: Segregated Witness (weight concept)
- BIP144: Segregated Witness (serialization)
- NIST FIPS 204: Dilithium signature sizes
