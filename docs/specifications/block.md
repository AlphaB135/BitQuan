# Block Specification

Version: 0.0.1-alpha
Status: Draft

## Overview

BitQuan blocks follow Bitcoin-style Proof-of-Work consensus with post-quantum
signature support and block weight accounting for large PQC signatures.

## Block Structure

```rust
struct Block {
    header: BlockHeader,
    transactions: Vec<Transaction>,
}
```

## Block Header

```rust
struct BlockHeader {
    version: i32,           // Block version
    prev_block: [u8; 32],   // Previous block hash
    merkle_root: [u8; 32],  // Merkle root of transactions
    pqc_agg_hint: [u8; 32], // Witness merkle root
    time: u32,              // Block timestamp (Unix epoch)
    bits: u32,              // Target difficulty (compact format)
    nonce: u64,             // Proof-of-work nonce
}
```

### Version
- Version 1: Genesis and initial blocks
- Version 2: With witness support (future)

### Previous Block Hash
SHA256(SHA256(previous_block_header))

Genesis block: all zeros

### Merkle Root
Merkle tree of transaction IDs (without witness data).

```
merkle_root = merkle_tree_root([tx[0].txid, tx[1].txid, ..., tx[n].txid])
```

### PQC Aggregate Hint
Merkle tree of witness transaction IDs (including witness data).

```
pqc_agg_hint = merkle_tree_root([tx[0].wtxid, tx[1].wtxid, ..., tx[n].wtxid])
```

### Timestamp
- Unix timestamp in seconds
- Must be > median of last 11 blocks (MTP)
- Must be < current time + 2 hours

### Bits (Difficulty Target)
Compact representation of target difficulty.

```
target = coefficient * 2^(8 * (exponent - 3))
bits = (exponent << 24) | coefficient
```

### Nonce
64-bit proof-of-work nonce. Block hash must be <= target.

## Block Hash

```
block_hash = SHA256(SHA256(
    version ||
    prev_block ||
    merkle_root ||
    pqc_agg_hint ||
    time ||
    bits ||
    nonce
))
```

Block is valid if: `block_hash <= target`

## Block Validation

### Header Validation
1. Version must be known (1 or 2)
2. Previous block must exist (except genesis)
3. Timestamp > MTP and < now + 2 hours
4. Bits matches difficulty algorithm
5. Block hash <= target
6. Merkle root matches transaction tree
7. PQC hint matches witness tree

### Block Validation
1. First transaction is coinbase
2. Only first transaction is coinbase
3. Block weight <= 4,000,000 WU (see block-weight.md)
4. All transactions valid
5. No duplicate transactions
6. Coinbase value <= subsidy + fees

### Contextual Validation
1. Block builds on best chain or valid fork
2. All inputs reference existing UTXOs
3. No double-spends
4. Difficulty matches retarget algorithm
5. Timestamp ordering valid

## Genesis Block

```
version: 1
prev_block: 0000000000000000000000000000000000000000000000000000000000000000
merkle_root: (calculated from coinbase)
pqc_agg_hint: (calculated from coinbase)
time: 1729944000 (Oct 26, 2025 12:00:00 UTC)
bits: 0x207fffff (very low difficulty)
nonce: (to be determined by mining)
```

Genesis coinbase message:
```
"The Quantum Age Begins - 26 Oct 2025. Ownerless. Verifiable. For everyone."
```

## Block Propagation

### Compact Blocks
Header-first propagation for bandwidth efficiency.

1. Node receives header
2. Validates header PoW
3. Requests missing transactions via inv/getdata
4. Validates full block
5. Relays to peers

### Block Relay Policy
- Blocks with valid PoW are relayed immediately
- Invalid blocks are not relayed
- Orphan blocks stored temporarily (max 100)
- Blocks older than 24 hours are not requested

## Merkle Tree Construction

```python
def merkle_root(hashes):
    if len(hashes) == 0:
        return bytes(32)
    if len(hashes) == 1:
        return hashes[0]

    tree = hashes[:]
    while len(tree) > 1:
        next_level = []
        for i in range(0, len(tree), 2):
            if i + 1 < len(tree):
                next_level.append(sha256(sha256(tree[i] + tree[i+1])))
            else:
                # Odd number: hash with itself
                next_level.append(sha256(sha256(tree[i] + tree[i])))
        tree = next_level

    return tree[0]
```

## Size Limits

- Max block size: 4,000,000 bytes (raw)
- Max block weight: 4,000,000 WU (see block-weight.md)
- Max transactions per block: 65,535
- Max sigops per block: 80,000

## Difficulty Adjustment

See BQIP-0003 for ASERT retarget algorithm.

Target adjustment every block based on:
- Target block time: 600 seconds (10 minutes)
- Half-life: 86,400 seconds (1 day)
- Minimum difficulty: 0x207fffff

## Examples

### Block Header (hex)

```
01000000  // version
0000...00 // prev_block (32 bytes)
abcd...ef // merkle_root (32 bytes)
1234...56 // pqc_agg_hint (32 bytes)
e8030000  // time
ffff7f20  // bits
4200000000000000 // nonce
```

### Block JSON

```json
{
  "header": {
    "version": 1,
    "prev_block": "0000000000000000000000000000000000000000000000000000000000000000",
    "merkle_root": "abcdef...",
    "pqc_agg_hint": "123456...",
    "time": 1729944000,
    "bits": "0x207fffff",
    "nonce": 42
  },
  "transactions": [
    {
      "version": 2,
      "inputs": [...],
      "outputs": [...],
      "lock_time": 0,
      "witnesses": []
    }
  ]
}
```

## Header-First Sync

Nodes synchronize blockchain in two phases:

1. Header sync: Download all headers (small, fast)
2. Block sync: Download full blocks (large, slow)

Benefits:
- Early detection of best chain
- Bandwidth efficiency
- Parallel block download
- SPV wallet support

## References

- Bitcoin block format
- BIP141: Segregated Witness
- BIP152: Compact Block Relay
