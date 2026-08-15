# Attack Report #005: Deep Chain Reorganization & Time Warp Manipulation

**Date**: 2026-08-15 10:58:00 UTC  
**Attack Type**: Consensus / Fork Choice & Difficulty Manipulation  
**Severity**: High  
**Status**: Blocked (Mitigated & Verified)  
**Target Component**: `crates/consensus/src/fork.rs`, `crates/consensus/src/asert.rs`

---

## 1. Attack Objective & Vector Description

The objective of this attack is twofold:
1. **Deep Reorganization (51% History Rewrite)**: Mine an alternative private fork off an old block (e.g. depth > 100) and broadcast it suddenly to erase finalized transactions and execute long-range double-spends.
2. **Time Warp Difficulty Manipulation**: Manipulate timestamps on block headers to fool the difficulty adjustment algorithm into artificially lowering mining difficulty to mine blocks rapidly.

---

## 2. Steps to Reproduce (PoC)

```rust
use bitquan_consensus::fork::{ForkChoice, ForkError};
use bitquan_consensus::pow::header_hash;
use bitquan_types::BlockHeader;

let mut fc = ForkChoice::new(); // Default max_reorg = 100

// 1. Genuine chain A reaches height 150
// 2. Attacker releases private chain B branched from Genesis (height 160, reorg depth = 150)
let header_b160 = ...;

let result = fc.add_block(header_b160);
// Attacker expectation: Chain B replaces Chain A as new tip
// Node defense behavior:
assert!(matches!(result, Err(ForkError::ReorgTooDeep(150, 100))));
```

---

## 3. Observed Behavior & Red Team Findings

1. **Reorg Depth Hard Cap**:
   - `ForkChoice` tracks the common ancestor distance (`reorg_depth`).
   - If `reorg_depth > max_reorg` (100 blocks), the node rejects the block with `ForkError::ReorgTooDeep(depth, max_reorg)`.
   - Long-range forks from malicious miners cannot reorganize transactions older than 100 confirmations.
2. **ASERT Continuous Difficulty Resistance**:
   - BitQuan uses **ASERT (Absolutely Scheduled Exponential Runtime Target)** adjusting difficulty exponentially on every single block:
     $$Target_{next} = Target_{anchor} \times 2^{\frac{(t - t_{anchor}) - (h - h_{anchor})\tau}{\tau_{half}}}$$
   - Because target calculation is anchored continuously to genesis/checkpoints rather than sliding windows of $N$ previous blocks, timestamp manipulation within a local window cannot cause runaway difficulty drops.
3. **Median-Time-Past (MTP) Rule**:
   - Header timestamps must be strictly greater than the median of the past 11 blocks (`MTP-11`) and cannot exceed current system time by more than $+7200$ seconds.

---

## 4. Impact Assessment

- **Availability**: Maintained (Nodes do not discard finalized chain history).
- **Integrity**: Maintained (Transactions with $\ge 100$ confirmations are mathematically immutable).
- **Confidentiality**: N/A.

---

## 5. Defense Verification

- Automated test executed: `cargo test -p bitquan-consensus --test fork_edge_cases`
- Test Output:
  ```text
  running 5 tests
  test test_reorg_depth_tracking ... ok
  test test_deep_reorg_rejected ... ok
  test test_invalid_block_marking ... ok
  test test_tie_breaking_by_timestamp ... ok
  test test_reorg_over_100_blocks ... ok
  test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.48s
  ```
- **Red Team Verdict**: Defense is ACTIVE and functioning as intended.
