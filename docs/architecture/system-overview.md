# BitQuan System Architecture

## Overview

BitQuan is a minimal proof-of-work blockchain with post-quantum cryptography (Dilithium) signatures and a public UTXO ledger. The system follows Bitcoin's design principles while integrating PQC primitives.

## System Components

```
┌──────────────────────────────────────────────────────────────┐
│                     BitQuan Node                             │
├──────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌────────────┐  ┌────────────┐  ┌────────────┐            │
│  │    RPC     │  │    CLI     │  │   Mining   │            │
│  │  Server    │  │  Interface │  │   Module   │            │
│  └─────┬──────┘  └─────┬──────┘  └─────┬──────┘            │
│        │               │               │                    │
│        └───────────────┼───────────────┘                    │
│                        │                                    │
│                ┌───────▼────────┐                           │
│                │  Node Core     │                           │
│                │  (bitquan-node)│                           │
│                └───────┬────────┘                           │
│                        │                                    │
│        ┌───────────────┼───────────────┐                    │
│        │               │               │                    │
│  ┌─────▼─────┐  ┌─────▼─────┐  ┌─────▼─────┐              │
│  │  Network  │  │ Consensus │  │  Mempool  │              │
│  │   (P2P)   │  │  Engine   │  │  Manager  │              │
│  └─────┬─────┘  └─────┬─────┘  └─────┬─────┘              │
│        │               │               │                    │
│        └───────────────┼───────────────┘                    │
│                        │                                    │
│                ┌───────▼────────┐                           │
│                │  Storage Layer │                           │
│                │   (RocksDB)    │                           │
│                └────────────────┘                           │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

## Layer Architecture

### Layer 1: Interface Layer

**RPC Server** (`bitquan-rpc`)
- JSON-RPC 2.0 interface
- Methods: getblockcount, getblockchaininfo, getmininginfo
- Mining API: getwork, submitwork, getblocktemplate
- Rate limiting and allow-list

**CLI Interface** (`bitquan-node`)
- Node management (start, stop, status)
- Wallet operations (create, balance, send)
- Mining controls (start, stop, difficulty)
- Network selection (mainnet, testnet, devnet, regtest)

**Mining Module**
- Block template generation
- PoW hashing (SHA-256d)
- Work submission validation
- Difficulty adjustment (ASERT)

### Layer 2: Core Layer

**Node Core** (`bitquan-node`)
- Chain state management
- Block validation pipeline
- Transaction relay coordination
- UTXO set management

**Network (P2P)** (`bitquan-network`)
- Version handshake (net-magic)
- Inv/getdata relay
- Block propagation
- Peer ban scoring
- DoS protection

**Consensus Engine** (`bitquan-consensus`)
- Block validation
- Transaction validation
- PoW verification (SHA-256d)
- Difficulty retarget (ASERT)
- Fork choice (longest chain)
- Reorg handling (max depth: 100)

**Mempool Manager** (`bitquan-mempool`)
- Transaction pool
- Fee-per-weight ordering (qbits/WU)
- Eviction policy (lowest fee first)
- Protected fee rate (≥10 qbits/WU)
- Size limit (300 MB default)

### Layer 3: Data Layer

**Storage Layer** (`bitquan-storage`)
- RocksDB backend
- Column families:
  - `utxo`: UTXO set (txid:vout → TxOut)
  - `blocks`: Block data (hash → Block)
  - `index`: Block index (height → hash)
  - `meta`: Chain metadata
- Compaction hints
- Salvage tool

**Crypto Primitives** (`bq-crypto`)
- Dilithium3 (baseline PQC)
- Falcon512 (optional)
- SPHINCS+ (future)
- KDF/RNG (domain-separated)

**Types** (`bitquan-types`)
- Transaction structure
- Block structure
- UTXO types
- Signature types (SigAlgorithm enum)

## Data Flow

### Block Validation Pipeline

```
New Block → Network → Consensus → Validation → Storage → Update UTXO
    │          │          │            │           │            │
    │          │          │            │           │            └─→ Chain Tip
    │          │          │            │           └─→ Write to DB
    │          │          │            └─→ Check PoW, Weight, Signatures
    │          │          └─→ Fork Choice (longest chain)
    │          └─→ Peer relay
    └─→ Deserialize
```

### Transaction Flow

```
New TX → Mempool → Fee Check → Validation → Pool → Block Template
   │        │          │           │          │           │
   │        │          │           │          │           └─→ Mining
   │        │          │           │          └─→ Sorted by fee/WU
   │        │          │           └─→ Sig verify, UTXO check
   │        │          └─→ Fee ≥ 10 qbits/WU
   │        └─→ Size check
   └─→ Deserialize
```

### Mining Flow

```
getwork → Template → Add Coinbase → Merkle Root → PoW Search → submitwork
    │         │            │             │             │            │
    │         │            │             │             │            └─→ Propagate
    │         │            │             │             └─→ Hash < target
    │         │            │             └─→ Commit to TXs
    │         │            └─→ Reward + fees
    │         └─→ Select TXs by fee density
    └─→ Get best tip, difficulty
```

## Consensus Rules

### Block Validation

1. Header validation
   - PoW: SHA-256d(header) < target
   - Parent exists in chain
   - Timestamp > median(last 11)
   - Version ≥ minimum

2. Weight validation
   - Block weight ≤ 4,000,000 WU
   - Weight = base_size*4 + sig_count*384

3. Transaction validation
   - No double-spends
   - All inputs exist in UTXO set
   - Sum(outputs) ≤ Sum(inputs) + coinbase_reward
   - Valid PQC signatures (Dilithium3)

4. Merkle root validation
   - merkle_root = MerkleTree(txids)

### Difficulty Adjustment (ASERT)

```
target_next = target_anchor * 2^exponent

where:
  exponent = (time_delta - block_time * height_delta) / half_life
  block_time = 600 seconds (10 minutes)
  half_life = 86400 seconds (1 day)

Bounds:
  1.0 ≤ target ≤ 2^208 * 65535
```

### Fee Market (BQIP-0002)

**Weight Formula:**
```
tx_weight = base_size * 4 + sig_count * 384

where:
  base_size = serialized size (bytes)
  sig_count = total signatures across all witnesses
  384 = SIGNATURE_WEIGHT (WU per PQC sig)
```

**Mempool Policy:**
- Minimum fee rate: 10 qbits/WU
- Sort by: fee / weight (descending)
- Eviction: remove lowest fee/weight first
- Max size: 300 MB (configurable)

**Block Template:**
- Select TXs by fee density (greedy)
- Enforce weight limit: ≤ 4,000,000 WU
- Include coinbase (reward + fees)

## Security Considerations

### Network Security

- Net-magic per network (mainnet/testnet/devnet/regtest)
- Replay protection (chain-id in TX digest)
- Peer ban scoring (malformed messages)
- Inv/getdata rate limits
- Max message size: 32 MB

### Consensus Security

- Max reorg depth: 100 blocks (safety limit)
- Checkpoint blocks (future)
- PoW difficulty floor (prevent zero-work)
- Weight limits prevent DOS
- Signature verification (PQC)

### Storage Security

- UTXO set integrity (hashed)
- Block index consistency
- RocksDB compaction
- Salvage tools for corruption

## Performance Characteristics

### Throughput

- Block time: 10 minutes (600s)
- Block weight: 4,000,000 WU
- Estimated TPS: ~10-20 (depends on sig count)

### Storage

- UTXO set: ~1-10 GB (depends on usage)
- Block data: ~50 GB/year (full blocks)
- Index: ~1 GB/year

### Validation

- PoW: SHA-256d (fast, hardware-accelerated)
- Signatures: Dilithium3 (~1000 sigs/sec per core)
- Merkle verification: O(log n)

## Network Topology

```
        Node A (Full Node)
           │
           ├─→ Node B (Miner)
           ├─→ Node C (Full Node)
           └─→ Node D (Light Client - future)
```

**Node Types:**
- Full Node: validates all blocks/TXs, stores full chain
- Miner: full node + mining (PoW search)
- Light Client: (future) SPV-like, PQC merkle proofs

## Deployment

### Networks

1. **Mainnet** (production)
   - Net-magic: 0xBQA1
   - Chain-id: 1
   - Genesis: TBD

2. **Testnet** (public testing)
   - Net-magic: 0xBQT1
   - Chain-id: 2
   - Free coins (faucet)

3. **Devnet** (development)
   - Net-magic: 0xBQD1
   - Chain-id: 3
   - Fast blocks (60s)

4. **Regtest** (local testing)
   - Net-magic: 0xBQR1
   - Chain-id: 4
   - Instant mining

## Future Extensions

- Compact blocks (BIP152-like)
- Schnorr batch verification (if supported by PQC)
- Lightning Network (second layer)
- Merkle proofs (light clients)
- Cross-chain bridges (with PQC)

## References

- BQIP-0001: PQC Signature Standard
- BQIP-0002: Block Weight and Fee Market
- BQIP-0003: Difficulty Retarget (ASERT)
- BQIP-0004: Governance and Activation
- Bitcoin: https://bitcoin.org/bitcoin.pdf
- Dilithium: https://pq-crystals.org/dilithium/
