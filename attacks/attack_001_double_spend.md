# Attack Report #001: Double-Spend & 0-Conf Race Attack

**Date**: 2026-08-15 10:55:00 UTC  
**Attack Type**: Mempool & Consensus / Double-Spend  
**Severity**: Critical  
**Status**: Blocked (Mitigated & Verified)  
**Target Component**: `crates/mempool/src/lib.rs`, `crates/consensus/src/validation.rs`

---

## 1. Attack Objective & Vector Description

The objective of this attack is to exploit 0-confirmation transactions or race conditions within the P2P mempool to spend the same Unspent Transaction Output (UTXO) twice.

### Attack Scenarios:
1. **0-Conf Merchant Fraud (Sequential Double-Spend)**:
   - Attacker broadcasts Transaction 1 ($Tx_1$: Alice $\to$ Merchant) paying for goods.
   - Immediately after merchant accepts 0-conf, attacker broadcasts Transaction 2 ($Tx_2$: Alice $\to$ Attacker Wallet) referencing the exact same `(prev_txid, prev_vout)` input with a higher or equal fee rate.
2. **Concurrent Multi-Node Race**:
   - Attacker connects to Node A and Node B simultaneously.
   - Attacker injects $Tx_1$ to Node A and $Tx_2$ to Node B concurrently at $t_0$.
3. **Multi-Input Partial Double-Spend**:
   - Attacker crafts a transaction with inputs $[UTXO_A, UTXO_B]$ where $UTXO_A$ is fresh and $UTXO_B$ is already spent in the mempool.

---

## 2. Steps to Reproduce (PoC)

### A. Unit-Level Test Vector
```rust
use bitquan_types::{Transaction, TxIn, TxOut, OutPoint, SigAlgorithm, NetworkId};
use bitquan_mempool::{Mempool, MempoolPolicy};

let mut mempool = Mempool::new(MempoolPolicy::default());

let shared_input = TxIn {
    prev_txid: [0xaa; 32],
    prev_vout: 0,
    script_sig: vec![],
    sequence: 0xffffffff,
};

// Tx 1: Alice -> Merchant
let tx1 = Transaction {
    version: 1,
    network: NetworkId::Devnet,
    genesis_hash: [0u8; 32],
    lock_time: 0,
    inputs: vec![shared_input.clone()],
    outputs: vec![TxOut { value: 50_000, script_pubkey: vec![0x51] }],
    sig_algo: SigAlgorithm::Dilithium5,
    witnesses: vec![],
};

// Tx 2: Alice -> Attacker (Competing Double-Spend)
let tx2 = Transaction {
    version: 1,
    network: NetworkId::Devnet,
    genesis_hash: [0u8; 32],
    lock_time: 0,
    inputs: vec![shared_input],
    outputs: vec![TxOut { value: 50_000, script_pubkey: vec![0x52] }],
    sig_algo: SigAlgorithm::Dilithium5,
    witnesses: vec![],
};

// 1. Ingest Tx1
assert!(mempool.insert(tx1, 5_000).is_ok());

// 2. Ingest Tx2 (Must be rejected)
let result = mempool.insert(tx2, 5_000);
assert!(result.is_err());
```

---

## 3. Observed Behavior & Red Team Findings

1. **Mempool Ingestion Defense**:
   - When $Tx_1$ is inserted, its inputs are recorded in `self.spent_outpoints` (a HashSet of `OutPoint`).
   - When $Tx_2$ arrives, the mempool evaluates `self.spent_outpoints.contains(&outpoint)`.
   - The mempool immediately returns `Err(Error::Invalid("Double spend detected..."))` and aborts before queueing or relaying the transaction.
2. **Atomic Ingestion Audit**:
   - In multi-input transactions, `new_outpoints` are gathered and checked atomically before insertion into `self.spent_outpoints`. No partial locking occurs on validation failure.
3. **Consensus Validation**:
   - In `bitquan-consensus`, `validate_transaction` checks that all inputs exist in the UTXO set and are not duplicated in the same block.

---

## 4. Impact Assessment

- **Availability**: Low (No node crash or deadlock during double-spend attempts).
- **Integrity**: Maintained (Double-spend was successfully rejected; UTXO set remained mathematically consistent).
- **Confidentiality**: N/A.

---

## 5. Defense Verification

- Automated test executed: `cargo test --test chaos_adversarial_suite -- test_chaos_scenario_4_race_condition_double_spend`
- Test Output:
  ```text
  ✅ Tx 1 (Alice -> Bob) accepted into mempool
  🛡️  Tx 2 (Alice -> Charlie) BLOCKED: Invalid: Double spend detected: input prev_txid=0 prev_vout=... already spent in mempool
  🎯 [CHAOS 4 PASSED] Double-spend attack instantly detected and rejected!
  ```
- **Red Team Verdict**: Defense is ACTIVE and functioning as intended.
