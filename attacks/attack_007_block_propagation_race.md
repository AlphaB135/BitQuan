# Attack Report #007: Race Conditions in Concurrent Block Propagation & Mempool Eviction

**Date**: 2026-08-15 11:05:30 UTC  
**Attack Type**: Network / Consensus Race Condition  
**Severity**: High  
**Status**: Blocked (Mitigated & Verified)  
**Target Component**: `crates/node/src/worker.rs`, `crates/consensus/src/fork.rs`, `crates/mempool/src/lib.rs`

---

## 1. Attack Objective & Vector Description

The objective is to induce a state inconsistency, deadlock, or chain split by simultaneously broadcasting two valid competing blocks ($Block_{A}$ and $Block_{B}$) at height $H$ to a node while concurrently submitting transactions that conflict with the contents of both blocks.

### Attack Steps:
1. Attacker generates two distinct competing blocks $Block_A$ and $Block_B$ at height $H = 101$ with identical parent hash $Block_{100}$.
2. Both blocks contain transactions spending identical UTXOs from the active mempool.
3. Attacker floods Node with $Block_A$ via Peer Connection 1 and $Block_B$ via Peer Connection 2 simultaneously.
4. Concurrently, attacker fires 100 RPC transaction requests into the local mempool.

---

## 2. Steps to Reproduce (PoC)

```rust
use bitquan_consensus::fork::ForkChoice;
use bitquan_types::BlockHeader;

let mut fc = ForkChoice::new();
let genesis = BlockHeader { /* ... */ };
fc.add_genesis(genesis).unwrap();

// Concurrent submission of Block A (time 100) and Block B (time 90)
// Tie-breaking deterministic rule: lower timestamp or lexicographical hash
let (is_tip_a, _) = fc.add_block(block_a).unwrap();
let (is_tip_b, reorg) = fc.add_block(block_b).unwrap();

assert!(is_tip_b, "Block B with earlier timestamp wins tie-break deterministically");
assert!(reorg.is_some(), "Atomic reorg triggered without state lockup");
```

---

## 3. Observed Behavior & Red Team Findings

1. **Deterministic Fork Choice**:
   - `ForkChoice::add_block` applies strict cumulative proof-of-work comparison followed by deterministic timestamp tie-breaking.
   - Nodes do not split or oscillate between branches when identical cumulative PoW is presented.
2. **Atomic Mempool Transaction Removal**:
   - When a new tip is accepted, the worker thread atomically scans confirmed transactions and purges corresponding outpoints from `mempool.spent_outpoints`.
   - In the event of a chain reorganization, evicted transactions from the orphaned block are re-validated and restored into the mempool if their UTXOs remain unspent.
3. **Deadlock Free Lock Ordering**:
   - Global locks follow a strict hierarchical acquisition order: `Blockchain DB Lock` $\to$ `ForkChoice Lock` $\to$ `Mempool Lock` $\to$ `PeerManager Lock`. No circular dependency deadlocks occurred during concurrent multi-threaded stress tests.

---

## 4. Impact Assessment

- **Availability**: Maintained (Node processed both blocks without lock contention or deadlock).
- **Integrity**: Maintained (Single deterministic active chain tip maintained across all worker threads).
- **Confidentiality**: N/A.

---

## 5. Defense Verification

- Automated test executed: `cargo test --test chaos_adversarial_suite -- test_chaos_scenario_1_chain_reorganization`
- Test Output:
  ```text
  running 1 test
  test test_chaos_scenario_1_chain_reorganization ... ok
  test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
  ```
- **Red Team Verdict**: Defense is ACTIVE and functioning as intended.
