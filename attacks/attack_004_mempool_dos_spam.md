# Attack Report #004: Mempool Exhaustion & Low-Fee Spam

**Date**: 2026-08-15 10:57:30 UTC  
**Attack Type**: Mempool / Resource Exhaustion & Spam  
**Severity**: Medium  
**Status**: Blocked (Mitigated & Verified)  
**Target Component**: `crates/mempool/src/lib.rs`

---

## 1. Attack Objective & Vector Description

The objective is to flood the node's mempool with hundreds of thousands of low-fee or dust transactions (e.g. 1 satoshi / 0.00000001 BQ) to consume RAM, prevent honest transactions from being relayed, and force out-of-memory (OOM) termination on validator nodes.

### Attack Steps:
1. Attacker generates 10,000 unique validly signed transactions spending dust outputs.
2. Attacker sets transaction fee to 1 unit (sub-minimum relay fee).
3. Attacker streams transactions across all open P2P connections to saturate RAM.

---

## 2. Steps to Reproduce (PoC)

```rust
use bitquan_mempool::Mempool;
use bitquan_types::{Transaction, TxIn, TxOut, NetworkId, SigAlgorithm};

let mut mempool = Mempool::new().expect("mempool creation");

let dust_tx = Transaction {
    version: 1,
    network: NetworkId::Devnet,
    genesis_hash: [0u8; 32],
    sig_algo: SigAlgorithm::Dilithium5,
    inputs: vec![TxIn {
        prev_txid: [0x01; 32],
        prev_vout: 0,
        sequence: 0xFFFFFFFF,
        script_sig: vec![],
    }],
    outputs: vec![TxOut { value: 1, script_pubkey: vec![0x51] }],
    witnesses: vec![],
    lock_time: 0,
};

// Attempt to insert with 1 sat fee (under min relay threshold)
let result = mempool.insert(dust_tx, 1);
assert!(result.is_err());
```

---

## 3. Observed Behavior & Red Team Findings

1. **Minimum Relay Fee Enforcement**:
   - `Mempool::insert` calculates `entry.fee_per_weight` and compares against `self.policy.min_relay_fee_per_wu`.
   - Any transaction below threshold is rejected with:
     ```text
     Invalid: fee rate 0 below minimum 1
     ```
2. **Dust Output Threshold**:
   - Outputs below `policy.dust_threshold` (unless `OP_RETURN`) are rejected.
3. **Mempool Size Cap & Dynamic Eviction**:
   - When memory exceeds `max_size_bytes` (300 MB limit), the mempool evicts the lowest fee-rate transactions from the priority queue before accepting higher fee transactions.
4. **Signature Operations Limit**:
   - `sigops > max_sigops_per_tx` limits the verification CPU cost per transaction.

---

## 4. Impact Assessment

- **Availability**: Unaffected (Memory bounded at 300 MB, sub-threshold spam rejected before insertion).
- **Integrity**: Maintained (Fee priority sorting preserved).
- **Confidentiality**: N/A.

---

## 5. Defense Verification

- Automated test executed: `cargo test -p bitquan-mempool --test transaction_lifecycle_tests`
- Test Output:
  ```text
  running 7 tests
  test test_mempool_size_limit ... ok
  test test_mempool_size_tracking ... ok
  test test_multiple_transactions ... ok
  test test_transaction_add_success ... ok
  test test_low_fee_rejection ... ok
  test test_mempool_empty_state ... ok
  test test_mempool_fee_rate_policy ... ok
  test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
  ```
- **Red Team Verdict**: Defense is ACTIVE and functioning as intended.
