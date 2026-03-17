# BQIP-0004: Witness & Layer 2 Integration Planning

```
BQIP: 0004-L2
Title: Witness Model and Layer 2 Integration Planning
Author: BitQuan Maintainers
Status: Draft
Type: Standards Track (Informational)
Created: 2026-03-17
```

## Abstract

This document analyzes Layer 2 scaling options for BitQuan, a post-quantum blockchain using Dilithium5 signatures. We examine the unique challenges posed by large signature sizes (4,595 bytes), evaluate L2 architecture options, and provide recommendations for the optimal scaling strategy.

---

## Table of Contents

1. [Witness Model for Dilithium](#1-witness-model-for-dilithium)
2. [Layer 2 Architecture Options](#2-layer-2-architecture-options)
3. [Cross-Chain Bridge Design](#3-cross-chain-bridge-design)
4. [Recommendation](#4-recommendation)
5. [Implementation Roadmap](#5-implementation-roadmap)

---

## 1. Witness Model for Dilithium

### 1.1 Current Witness Structure

BitQuan's witness structure is designed for post-quantum signatures:

```rust
/// Witness container for PQC signatures
pub struct Witness {
    /// Signatures included in this witness
    pub signatures: Vec<SignaturePayload>,
}

/// Signature payload supporting multiple algorithms
pub enum SignaturePayload {
    /// Dilithium5 signature (4,595 bytes)
    Dilithium5 {
        public_key: [u8; 1952],
        signature: [u8; 4595],
    },
    /// ECDSA fallback for hybrid mode (71-73 bytes)
    ECDSA {
        public_key: [u8; 33],
        signature: Vec<u8>, // 71-73 bytes DER encoded
    },
}
```

### 1.2 Size Analysis

| Component | Bitcoin (ECDSA) | BitQuan (Dilithium5) | Ratio |
|-----------|-----------------|---------------------|-------|
| Public Key | 33 bytes | 1,952 bytes | 59x |
| Signature | 71-73 bytes | 4,595 bytes | 63x |
| Single Input Witness | ~107 bytes | ~6,547 bytes | 61x |
| 2-of-3 Multisig | ~320 bytes | ~14,500 bytes | 45x |

### 1.3 Script vs Signature Separation

```
┌────────────────────────────────────────────────────────────────────┐
│                    BitQuan Transaction Structure                   │
├────────────────────────────────────────────────────────────────────┤
│                                                                    │
│  Transaction                                                       │
│  └── Inputs[]                                                      │
│      ├── Previous Output (TXID + Index)                           │
│      ├── Sequence                                                  │
│      └── Witness (Segregated)                                     │
│          ├── ScriptWitness (redeem script for multisig)           │
│          └── SignatureWitness[]                                   │
│              ├── Dilithium5 Public Key (1,952 bytes)              │
│              └── Dilithium5 Signature (4,595 bytes)               │
│                                                                    │
│  └── Outputs[]                                                     │
│      ├── Amount                                                    │
│      └── ScriptPubKey                                              │
│                                                                    │
└────────────────────────────────────────────────────────────────────┘
```

**Key Insight**: Witness data is segregated (SegWit-style), allowing:
1. Witness pruning for SPV clients
2. Discount witness weight in fee calculation
3. Future upgradability for signature algorithms

### 1.4 Witness Size Optimization Strategies

#### Strategy 1: Signature Aggregation (Future)

```
Without Aggregation:
  Input 1: 1,952 + 4,595 = 6,547 bytes
  Input 2: 1,952 + 4,595 = 6,547 bytes
  Total: 13,094 bytes

With Aggregation (theoretical):
  Shared proof: ~2,000 bytes
  Individual signatures: 2 × 4,595 = 9,190 bytes
  Total: ~11,190 bytes (14% reduction)
```

**Note**: Dilithium doesn't natively support aggregation like Schnorr. Research required.

#### Strategy 2: Public Key Caching

```rust
/// Optimized witness with cached public keys
pub struct OptimizedWitness {
    /// Reference to cached public key (32-bit index)
    pub pubkey_ref: Option<u32>,
    /// Or full public key if not cached
    pub pubkey_full: Option<[u8; 1952]>,
    /// Signature (always required)
    pub signature: [u8; 4595],
}

// Size savings when cached:
// Without cache: 1,952 + 4,595 = 6,547 bytes
// With cache: 4 + 4,595 = 4,599 bytes (30% reduction)
```

#### Strategy 3: Compressed Integers

```rust
// Use CompactSize for all variable-length fields
// Saves 1-4 bytes per length prefix
pub fn encode_witness(witness: &Witness) -> Vec<u8> {
    let mut buf = Vec::new();

    // CompactSize for signature count
    encode_compact_size(witness.signatures.len() as u64, &mut buf);

    for sig in &witness.signatures {
        // CompactSize for signature length
        encode_compact_size(sig.len() as u64, &mut buf);
        buf.extend_from_slice(sig);
    }

    buf
}
```

### 1.5 Weight Units Calculation

BitQuan uses a discount factor for witness data (similar to Bitcoin SegWit):

```rust
/// Calculate transaction weight
pub fn calculate_weight(tx: &Transaction) -> u64 {
    let base_size = tx.base_size(); // Without witness
    let witness_size = tx.witness_size();

    // Witness data discounted 75% (weight = 1 instead of 4)
    let weight = base_size * 4 + witness_size * 1;

    weight
}

/// Calculate virtual bytes for fee estimation
pub fn calculate_vbytes(tx: &Transaction) -> u64 {
    calculate_weight(tx) / 4
}
```

**Example**:
- Base transaction: 200 bytes
- Witness (1 input): 6,547 bytes
- Weight: 200 × 4 + 6,547 × 1 = 7,347
- Virtual bytes: 7,347 / 4 = 1,837 vbytes

---

## 2. Layer 2 Architecture Options

### 2.1 Option A: State Channels (Lightning-style)

#### Overview

```
┌────────────────────────────────────────────────────────────────────┐
│                    State Channel Architecture                       │
├────────────────────────────────────────────────────────────────────┤
│                                                                    │
│   Alice                    Channel                   Bob           │
│   ─────                    ───────                   ───           │
│     │                                                   │          │
│     │  1. Fund Channel TX (on-chain)                   │          │
│     │──────────────────────────────────────────────────▶│          │
│     │     - 2-of-2 Multisig (14,500 bytes)             │          │
│     │                                                   │          │
│     │  2. Commitment TXs (off-chain)                   │          │
│     │◀─────────────────────────────────────────────────▶│          │
│     │     - Unlimited updates                          │          │
│     │     - No on-chain footprint                      │          │
│     │                                                   │          │
│     │  3. Close Channel TX (on-chain)                  │          │
│     │──────────────────────────────────────────────────▶│          │
│     │     - Final state settlement                     │          │
│     │                                                   │          │
└────────────────────────────────────────────────────────────────────┘
```

#### Pros

| Advantage | Description |
|-----------|-------------|
| **Instant Settlement** | Sub-second finality |
| **Privacy** | Off-chain transactions |
| **Scalability** | Unlimited TPS per channel |
| **Low Fees** | Only open/close on-chain |

#### Cons (BitQuan-specific)

| Challenge | Impact |
|-----------|--------|
| **Large Commitment TXs** | 14,500+ bytes for 2-of-2 |
| **HTLC Size** | Hash lock adds script overhead |
| **Channel Capacity** | Limited by on-chain UTXO |
| **Watchtowers** | Need to monitor for fraud |

#### Size Analysis for Lightning-style Channel

```
Funding Transaction:
  Input (1): 6,547 bytes (Dilithium5 witness)
  Output (2-of-2): ~200 bytes
  Total: ~6,750 bytes

Commitment Transaction (each update):
  Input (1): 6,547 bytes
  Output (2): ~400 bytes (to_local + to_remote)
  HTLC Output: ~500 bytes (if present)
  Total: ~7,500 bytes per commitment

Channel Close:
  Mutual close: ~7,000 bytes
  Unilateral close: ~7,500 + penalty TX
```

#### Verdict: **Feasible with modifications**

Requires:
1. Batching multiple HTLCs to amortize overhead
2. Simplified script (no complex HTLC scripts)
3. Higher channel minimums (due to large TX sizes)

---

### 2.2 Option B: Rollups

#### Overview

```
┌────────────────────────────────────────────────────────────────────┐
│                       Rollup Architecture                           │
├────────────────────────────────────────────────────────────────────┤
│                                                                    │
│   Users                Sequencer               L1 Chain            │
│   ─────                ─────────               ────────            │
│     │                                                    │         │
│     │  Submit TXs                                       │         │
│     │─────────────────────▶                             │         │
│     │                      │                            │         │
│     │                      │ Batch TXs                 │         │
│     │                      │ (1000s of TXs)            │         │
│     │                      │                            │         │
│     │                      │ Submit Batch + Proof       │         │
│     │                      ─────────────────────────────▶         │
│     │                                                   │         │
│     │                      ◀─────────────────────────────         │
│     │                      Batch Confirmation          │         │
│     │                                                    │         │
│     │  Get State                                        │         │
│     │◀─────────────────────                             │         │
│     │                                                    │         │
└────────────────────────────────────────────────────────────────────┘
```

#### Types of Rollups

| Type | Proof System | Finality | Security |
|------|--------------|----------|----------|
| **ZK-Rollup** | Validity proofs (SNARK/STARK) | Instant | Cryptographic |
| **Optimistic Rollup** | Fraud proofs | 7-day challenge | Economic |

#### ZK-Rollup for BitQuan

```
┌─────────────────────────────────────────────────────────────────┐
│                    ZK-Rollup Data Flow                          │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  L2 Transactions (1000 TXs × 7,500 bytes = 7.5 MB)             │
│       │                                                         │
│       ▼                                                         │
│  Compressed State Update (~100 KB)                             │
│       │                                                         │
│       ▼                                                         │
│  ZK Proof Generation (~200 bytes proof)                        │
│       │                                                         │
│       ▼                                                         │
│  L1 Batch Commitment (~10 KB on-chain)                         │
│                                                                 │
│  Compression Ratio: 750:1                                       │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

**Key Insight**: ZK proofs are constant-size regardless of transaction count. This makes rollups extremely attractive for BitQuan where individual transactions are large.

#### Optimistic Rollup for BitQuan

```
┌─────────────────────────────────────────────────────────────────┐
│                 Optimistic Rollup Data Flow                     │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  L2 Transactions (1000 TXs)                                     │
│       │                                                         │
│       ▼                                                         │
│  Sequencer Orders & Executes                                   │
│       │                                                         │
│       ▼                                                         │
│  State Root Commitment (32 bytes)                              │
│       │                                                         │
│       ▼                                                         │
│  L1 Batch Submission (~1 KB calldata)                          │
│       │                                                         │
│       ▼                                                         │
│  Challenge Period (7 days)                                     │
│       │                                                         │
│       ├── No Challenge → Finalized                             │
│       └── Challenge → Fraud Proof Resolution                   │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

#### Pros

| Advantage | ZK-Rollup | Optimistic |
|-----------|-----------|------------|
| **Finality** | Minutes | Days |
| **TPS** | 2,000+ | 500+ |
| **Data Compression** | High | Medium |
| **Complexity** | High | Medium |

#### Cons

| Challenge | ZK-Rollup | Optimistic |
|-----------|-----------|------------|
| **Proof Generation** | Compute intensive | N/A |
| **Challenge Period** | N/A | 7-day delay |
| **Sequencer Centralization** | Yes | Yes |

#### Verdict: **ZK-Rollup Recommended**

ZK-Rollups are ideal for BitQuan because:
1. Constant-size proofs amortize large signature overhead
2. Instant finality enables better UX
3. Data compression directly addresses size challenges

---

### 2.3 Option C: Sidechains

#### Overview

```
┌────────────────────────────────────────────────────────────────────┐
│                    Sidechain Architecture                           │
├────────────────────────────────────────────────────────────────────┤
│                                                                    │
│   ┌───────────────┐        Two-Way Peg        ┌───────────────┐   │
│   │               │◀──────────────────────────▶│               │   │
│   │  BitQuan L1   │                            │  Sidechain    │   │
│   │  (Main Chain) │                            │  (Child Chain)│   │
│   │               │                            │               │   │
│   │  PoW          │                            │  PoS/Federated│   │
│   │  0.5-1 TPS    │                            │  100+ TPS     │   │
│   │  Dilithium5   │                            │  Dilithium3   │   │
│   └───────────────┘                            └───────────────┘   │
│                                                                    │
│   Peg Operations:                                                  │
│   ───────────────                                                  │
│   1. Lock BQ on L1 → Mint sBQ on sidechain                        │
│   2. Use sBQ on sidechain (fast, cheap)                           │
│   3. Burn sBQ → Unlock BQ on L1                                   │
│                                                                    │
└────────────────────────────────────────────────────────────────────┘
```

#### Sidechain Types

| Type | Security Model | Trust Assumption |
|------|---------------|------------------|
| **Federated** | M-of-N signers | Trust federation |
| **PoS** | Staking economics | Trust majority stake |
| **Merged Mining** | L1 miners | Trust L1 hashpower |

#### Federated Sidechain for BitQuan

```
┌─────────────────────────────────────────────────────────────────┐
│                  Federated Peg Mechanism                        │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Lock Transaction (L1):                                        │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │ Input: User's BQ (6,547 bytes witness)                  │   │
│  │ Output: 8-of-15 Federation Multisig (~100,000 bytes)   │   │
│  └─────────────────────────────────────────────────────────┘   │
│                                                                 │
│  Federation Signs Release:                                     │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │ 8 signatures × 4,595 bytes = 36,760 bytes              │   │
│  │ 8 public keys × 1,952 bytes = 15,616 bytes             │   │
│  │ Total witness: ~52,376 bytes                            │   │
│  └─────────────────────────────────────────────────────────┘   │
│                                                                 │
│  Sidechain Mint (fast, low fee):                               │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │ Uses smaller Dilithium3 (2,400 byte signatures)        │   │
│  │ or aggregate signatures                                  │   │
│  └─────────────────────────────────────────────────────────┘   │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

#### Pros

| Advantage | Description |
|-----------|-------------|
| **Independent Consensus** | Can use faster block times |
| **Smaller Signatures** | Can use Dilithium3 on sidechain |
| **Feature Flexibility** | Can add smart contracts |
| **Isolation** | Sidechain issues don't affect L1 |

#### Cons

| Challenge | Impact |
|-----------|--------|
| **Federation Trust** | M-of-N security assumption |
| **Bridge Complexity** | Additional attack surface |
| **Liquidity Fragmentation** | Split across chains |
| **Large Peg TXs** | 50,000+ bytes for federation multisig |

#### Verdict: **Viable as secondary option**

Sidechains work but require:
1. Strong federation selection
2. Alternative signature scheme (Dilithium3) on sidechain
3. Careful bridge security

---

## 3. Cross-Chain Bridge Design

### 3.1 Lock/Mint Mechanism

```
┌────────────────────────────────────────────────────────────────────┐
│                    Lock/Mint Bridge Flow                           │
├────────────────────────────────────────────────────────────────────┤
│                                                                    │
│  Step 1: Lock on Source Chain                                     │
│  ───────────────────────────────                                  │
│  User sends TX to bridge address:                                 │
│  - Input: User's coins + Dilithium5 signature                     │
│  - Output: Time-locked to bridge (1 year default)                │
│  - Witness: 6,547 bytes                                           │
│                                                                    │
│  Step 2: Attestation                                              │
│  ─────────────────────                                            │
│  Validators observe lock TX:                                      │
│  - Wait for N confirmations (12 for BitQuan)                     │
│  - Each validator signs attestation                               │
│  - M-of-N attestations required                                   │
│                                                                    │
│  Step 3: Mint on Destination Chain                                │
│  ────────────────────────────────────                             │
│  Bridge contract verifies:                                        │
│  - M-of-N validator signatures                                    │
│  - Lock TX inclusion proof                                        │
│  - Amount matches                                                 │
│  Mints wrapped tokens to user                                     │
│                                                                    │
└────────────────────────────────────────────────────────────────────┘
```

### 3.2 Validator Set Selection

```rust
/// Bridge validator configuration
pub struct BridgeConfig {
    /// Number of validators in set
    pub validator_count: usize,

    /// Required signatures for valid attestation
    pub required_signatures: usize,

    /// Validator selection method
    pub selection_method: ValidatorSelection,

    /// Rotation period for validators
    pub rotation_period: Duration,
}

pub enum ValidatorSelection {
    /// Fixed set of trusted entities
    Fixed(Vec<ValidatorIdentity>),

    /// Elected by token holders
    StakeBased {
        min_stake: u64,
        election_period: Duration,
    },

    /// Derived from L1 miners (merged mining)
    MinerDerived {
        top_n_miners: usize,
    },
}

/// Recommended configuration for BitQuan
pub fn recommended_bridge_config() -> BridgeConfig {
    BridgeConfig {
        validator_count: 15,
        required_signatures: 11, // 73% threshold
        selection_method: ValidatorSelection::StakeBased {
            min_stake: 100_000_000, // 0.1 BQ
            election_period: Duration::from_days(30),
        },
        rotation_period: Duration::from_days(7),
    }
}
```

### 3.3 Security Assumptions

| Threat | Mitigation |
|--------|------------|
| **Validator Collusion** | M-of-N threshold, rotation |
| **Key Compromise** | HSM storage, key ceremonies |
| **Chain Reorg** | Multiple confirmations |
| **Smart Contract Bug** | Audits, formal verification |
| **Economic Attack** | Bonded validators, slashing |

### 3.4 Bridge Transaction Sizes

```
Lock Transaction (BitQuan → Other):
  Input: 6,547 bytes (Dilithium5)
  Output: ~200 bytes
  Total: ~6,750 bytes

Attestation (11-of-15):
  11 signatures × 4,595 = 50,545 bytes
  11 public keys × 1,952 = 21,472 bytes
  Total witness: ~72,000 bytes

Mint Transaction (on destination chain):
  Depends on destination chain format
  Ethereum: ~50,000 gas for verification
```

---

## 4. Recommendation

### 4.1 Primary Recommendation: ZK-Rollup

**Rationale:**

1. **Size Efficiency**: ZK proofs are constant-size (~200 bytes) regardless of how many transactions are batched. This directly addresses BitQuan's 63x larger signature problem.

2. **Instant Finality**: No 7-day challenge period like optimistic rollups.

3. **Data Compression**: 1000 transactions (7.5 MB) → 1 proof (~200 bytes) + state diff (~100 KB).

4. **Security**: Cryptographic guarantees, not economic assumptions.

### 4.2 Architecture Overview

```
┌────────────────────────────────────────────────────────────────────┐
│                  Recommended BitQuan L2 Stack                       │
├────────────────────────────────────────────────────────────────────┤
│                                                                    │
│  Layer 1 (BitQuan Main Chain)                                     │
│  ─────────────────────────────                                    │
│  - PoW consensus with SHA-256d                                    │
│  - Dilithium5 signatures (quantum-resistant)                      │
│  - 0.5-1 TPS, high security                                       │
│  - Stores rollup commitments                                      │
│                                                                    │
│  Layer 2 (BitQuan Rollup)                                         │
│  ─────────────────────────                                        │
│  - ZK-Rollup with STARK proofs                                    │
│  - Compressed Dilithium3 signatures                               │
│  - 2,000+ TPS, instant finality                                   │
│  - Post-quantum secure                                            │
│                                                                    │
│  Layer 2.5 (State Channels - Future)                              │
│  ───────────────────────────────────                              │
│  - For micropayments and high-frequency trading                   │
│  - Channels opened on rollup (smaller TXs)                        │
│  - Near-instant, sub-cent fees                                    │
│                                                                    │
└────────────────────────────────────────────────────────────────────┘
```

### 4.3 Implementation Complexity Estimate

| Component | Complexity | Time Estimate | Dependencies |
|-----------|------------|---------------|--------------|
| **ZK Circuit Design** | Very High | 6-9 months | STARK/SNARK expertise |
| **Sequencer Implementation** | High | 3-4 months | Rollup infrastructure |
| **L1 Contract** | Medium | 2-3 months | BitQuan script support |
| **Prover Network** | High | 3-4 months | Distributed computing |
| **SDK Integration** | Medium | 2-3 months | Existing bq-sdk |
| **Testing & Audits** | High | 3-4 months | Security firms |
| **Total** | - | **18-24 months** | - |

### 4.4 Migration Path from L1

```
┌────────────────────────────────────────────────────────────────────┐
│                    L1 → L2 Migration Path                          │
├────────────────────────────────────────────────────────────────────┤
│                                                                    │
│  Phase 1: Foundation (Months 1-6)                                 │
│  ───────────────────────────────────                              │
│  - Design ZK circuit for Dilithium verification                  │
│  - Implement basic sequencer                                      │
│  - Deploy testnet rollup                                          │
│                                                                    │
│  Phase 2: Integration (Months 7-12)                               │
│  ────────────────────────────────────                             │
│  - L1 commitment contract                                         │
│  - Prover network deployment                                      │
│  - Wallet SDK updates                                             │
│  - Testnet public launch                                          │
│                                                                    │
│  Phase 3: Migration (Months 13-18)                                │
│  ─────────────────────────────────────                            │
│  - Bridge L1 assets to L2                                         │
│  - Incentivize L2 usage (lower fees)                              │
│  - Mainnet rollup launch                                          │
│                                                                    │
│  Phase 4: Optimization (Months 19-24)                             │
│  ─────────────────────────────────────                            │
│  - Proof recursion for higher TPS                                 │
│  - State channels on rollup                                       │
│  - Cross-rollup communication                                     │
│                                                                    │
└────────────────────────────────────────────────────────────────────┘
```

---

## 5. Implementation Roadmap

### 5.1 Phase 1: Research & Prototyping (Q1-Q2 2026)

| Task | Deliverable |
|------|-------------|
| ZK Circuit Research | Dilithium verification circuit design |
| Proof System Selection | STARK vs SNARK trade-off analysis |
| Sequencer Prototype | Basic transaction ordering |
| Testnet Specification | L2 testnet parameters |

### 5.2 Phase 2: Core Development (Q3-Q4 2026)

| Task | Deliverable |
|------|-------------|
| Prover Implementation | STARK prover for Dilithium |
| Verifier Contract | L1 commitment verification |
| Sequencer Service | Production-grade sequencer |
| SDK Updates | Rollup support in bq-sdk |

### 5.3 Phase 3: Testing & Security (Q1 2027)

| Task | Deliverable |
|------|-------------|
| Internal Testing | 90%+ test coverage |
| External Audit | 2+ security audits |
| Bug Bounty | Immunefi/Code4rena program |
| Testnet Launch | Public testnet with real users |

### 5.4 Phase 4: Mainnet Launch (Q2 2027)

| Task | Deliverable |
|------|-------------|
| Gradual Rollout | Limited capacity initially |
| Monitoring | Prometheus/Grafana dashboards |
| Documentation | Developer guides, API docs |
| Support | Discord, GitHub issues |

---

## Appendix A: Comparison Matrix

| Feature | Lightning | ZK-Rollup | Optimistic | Sidechain |
|---------|-----------|-----------|------------|-----------|
| **TPS** | 1M+ (theoretical) | 2,000+ | 500+ | 100+ |
| **Finality** | Instant | Minutes | 7 days | Minutes |
| **L1 Footprint** | Low | Low | Medium | Low |
| **BitQuan Fit** | Moderate | **Excellent** | Good | Good |
| **Complexity** | Medium | High | Medium | Medium |
| **Security** | Economic | Cryptographic | Economic | Federation |
| **Post-Quantum** | Yes | Yes | Yes | Yes |

## Appendix B: Reference Implementation

See `crates/layer2/` for current rollup prototype implementation.

---

## References

- [BIP-173] Base32 Address Format for Native v0-16 Witness Outputs
- [BIP-350] Bech32m Format for v1+ Witness Addresses
- [Ethereum EIP-4844] Shard Blob Transactions
- [zkSync Architecture] Matter Labs ZK-Rollup Design
- [Arbitrum Nitro] Optimistic Rollup Technical Overview
- [NIST FIPS 205] CRYSTALS-Dilithium Digital Signature Standard

---

## Copyright

This document is placed in the public domain.

---

*Last Updated: 2026-03-17*
*Author: BitQuan Core Team*
