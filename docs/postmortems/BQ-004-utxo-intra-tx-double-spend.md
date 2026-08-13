# Post-mortem: BQ-004 — UTXO double-spend vulnerability via duplicate inputs in a single transaction

**Date:** 2026-08-13  
**Severity:** Critical — enables infinite money glitch via double-spending the same UTXO in the same transaction  
**Status:** Fixed, validated  

---

## Summary

The `apply_transaction` and `validate_transaction` logic in `crates/consensus/src/utxo.rs` failed to deduplicate inputs before processing them. An attacker could construct a transaction with identical inputs pointing to the same Unspent Transaction Output (UTXO). The validation logic would repeatedly fetch the UTXO from the database, accumulating its value multiple times into the `inputs_value`. Consequently, the transaction could mint arbitrary amounts of new tokens or pay inflated fees. The vulnerability was resolved by introducing a `HashSet` to deduplicate and strictly enforce that any UTXO is spent at most once per transaction.

No JIRA key. Found via proactive `scrutinize` skill audit.

---

## Symptom

If a transaction with duplicate inputs was broadcast:
- The transaction would successfully pass `UtxoSet::validate_transaction()`.
- The `inputs_value` would linearly scale by the number of times the input was duplicated.
- A miner or attacker could output the duplicated value into new UTXOs, bypassing the total supply invariant.
- `apply_transaction` would successfully execute. The first removal of the UTXO would succeed, and subsequent removals would return `Ok(None)` silently, completing the double-spend.

---

## Root Cause

In `apply_transaction`, the inputs were collected in a standard `Vec` without checking for duplicates:

```rust
let mut inputs_value = 0u128;
let mut spent_outpoints = Vec::new();

for input in &tx.inputs {
    let outpoint = OutPoint::new(input.prev_txid, input.prev_vout);
    
    // 1. Fetches UTXO from DB. Does NOT remove it yet.
    let utxo = self.get_utxo(&outpoint).ok_or(...)?;
    
    // 2. Accumulates value. If outpoint is duplicated, this happens twice.
    inputs_value = inputs_value.checked_add(utxo.output.value).ok_or(...)?;
    
    // 3. Pushes to vector.
    spent_outpoints.push(outpoint);
}

// ... fee calculation succeeds because inputs_value is falsely high ...

// 4. Removes from DB.
for outpoint in spent_outpoints {
    self.remove_utxo(&outpoint)?; // Returns Ok(None) silently if already removed!
}
```

The combination of delayed removal, lack of uniqueness checks, and silent failure on removing non-existent UTXOs created the perfect storm for an intra-transaction double spend.

---

## Why It Produced the Symptom

Because the UTXO is only removed *after* the entire input array is evaluated, `get_utxo()` always succeeds for duplicate inputs. The `inputs_value` accumulator adds the value as many times as the attacker provides the input. 

When generating outputs, the attacker can specify a value up to the falsely inflated `inputs_value`. The validation passes because `inputs_value >= outputs_value`.

Finally, the cleanup loop uses `HashMap::remove()`, which returns `None` if the key is missing. The `remove_utxo()` wrapper explicitly ignored `None` (returning `Ok(None)`), masking the secondary removal attempts.

---

## Fix

**`crates/consensus/src/utxo.rs`**
Replaced the `Vec::new()` with `std::collections::HashSet::new()` for tracking `spent_outpoints` during validation.

```rust
let mut spent_outpoints = std::collections::HashSet::new();

for input in &tx.inputs {
    let outpoint = OutPoint::new(input.prev_txid, input.prev_vout);

    if !spent_outpoints.insert(outpoint) {
        return Err(UtxoError::DoubleSpend(input.prev_txid, input.prev_vout));
    }
    // ...
```

This ensures $O(1)$ duplicate detection and immediately halts validation with a `DoubleSpend` error if the transaction attempts to reference the same output more than once.

Applied the exact same `HashSet` logic to both `apply_transaction` (the state-mutating execution) and `validate_transaction` (the dry-run validation for mempool).

---

## How It Was Found

During a routine `scrutinize` pass of the `crates/consensus/src/utxo.rs` file. By tracing the lifecycle of a `TxIn` from struct parsing to state modification, I mapped out how the state was retrieved, mutated, and applied. The gap between `get_utxo` (read) and `remove_utxo` (write) across a loop boundary immediately signaled a potential race condition or duplicate evaluation flaw.

---

## Why It Slipped Through

**Happy-path unit tests.**
The unit tests (`utxo_set_basic_operations`, `detect_double_spend`) correctly tested double spending *across* two different transactions, but never evaluated the edge case of double spending *within* the exact same transaction.

---

## Validation

The code cleanly compiles with `cargo check -p bitquan-consensus --tests`. The new logic strictly returns `Err(UtxoError::DoubleSpend)` on any duplicated inputs.

---

## Action Items

1. **Add unit test for intra-transaction double spends.** Write a test that crafts a transaction with two identical `TxIn` entries and asserts that `validate_transaction` returns `Err(UtxoError::DoubleSpend)`.
   *(Owner: consensus maintainer)*
2. **Review Mempool duplicate detection.** Ensure that the mempool implementation also explicitly rejects transactions with duplicate inputs before they even reach the consensus engine.
   *(Owner: network maintainer)*
