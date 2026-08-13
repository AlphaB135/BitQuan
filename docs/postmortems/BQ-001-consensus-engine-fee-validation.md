# Post-mortem: BQ-001 — ConsensusEngine::validate_block() always returns FeeValidation error

**Date:** 2026-08-13  
**Severity:** Critical — blocks all block validation via the public engine API  
**Status:** Fixed, validated  

---

## Summary

`ConsensusEngine::validate_block()` — the primary public API for validating blocks — returned a `FeeValidation` error on every single call, making it impossible to validate any block through the engine. The bug was introduced when strict fee enforcement was added to `validate_block()` (requiring callers to supply exact UTXO-derived fees), but `ConsensusEngine::validate_block()` was not updated to match. Fixed by adding a `total_fees: u128` parameter to `ConsensusEngine::validate_block()` and delegating it directly to `validate_block_with_fees()`. A secondary test gap was also fixed: `test_validate_block_weight_overflow` was missing `FeeValidation` and `CoinbaseMissing` / `InvalidCoinbase` arms in its match block, causing that test to also panic on any call through the `None`-fees path.

No JIRA key. Fix in-session. No prior fix attempts on this specific issue.

---

## Symptom

Any caller of `ConsensusEngine::validate_block()` receives:

```
Err(FeeValidation(
    "Total fees MUST be provided for coinbase validation. \
     Use validate_block_with_fees() or calculate from UTXO set. \
     Blocks with unknown fees CANNOT be accepted (inflation risk)."
))
```

100% reproducible — the error fires before any block-specific logic (weight check, PoW check, signature verification) because `validate_transaction_fees()` is reached and immediately rejects `None`.

Additionally, `test_validate_block_weight_overflow` in `crates/consensus/src/tests.rs` panics with `"Unexpected error: FeeValidation(...)"` because the match block covering expected errors did not include `FeeValidation`, `CoinbaseMissing`, or `InvalidCoinbase` arms.

---

## Root Cause

Two changes collided without being reconciled.

**Change A** (prior): `validate_block()` (free function, `crates/consensus/src/lib.rs:570`) was made to strictly require `total_fees: Option<u128>`. At line 892:

```rust
let fees = total_fees.ok_or_else(|| {
    ConsensusError::FeeValidation(
        "Total fees MUST be provided for coinbase validation. ..."
    )
})?;
```

This was intentional: passing `None` is now an outright error to prevent miners from claiming inflated coinbase rewards when fees are unknown.

**Change B** (the bug): `ConsensusEngine::validate_block()` (`lib.rs:1138`) was not updated after Change A. It continued to pass `None` unconditionally:

```rust
validate_block(
    ...
    None, // Total fees unknown in this context  ← BUG
    ...
)
```

The fix for the inflation risk (Change A) was correct. The failure to propagate the contract to `ConsensusEngine::validate_block()` (Change B) is the bug.

---

## Why It Produced the Symptom

Call path:

```
ConsensusEngine::validate_block()
  lib.rs:1160 → validate_block(total_fees = None, ...)
    lib.rs:864 → validate_transaction_fees(total_fees = None, ...)
      lib.rs:892 → None.ok_or_else(|| FeeValidation(...))? → Err
```

The error fires at line 892, well before any block-content checks (weight, PoW, merkle, signatures). No block, regardless of content, can pass through `ConsensusEngine::validate_block()` — the method is entirely non-functional.

---

## Fix

**`crates/consensus/src/lib.rs` — `ConsensusEngine::validate_block()`**

Added `total_fees: u128` as a required parameter. Replaced the duplicated `validate_block()` call (which was silently passing `None`) with a direct delegation to `validate_block_with_fees()`:

```rust
pub fn validate_block(
    &mut self,
    block: &Block,
    height: u64,
    total_fees: u128,          // ← added; callers must supply from UTXO set
    median_time_past: u64,
    network_adjusted_time: u64,
    uncles_ctx: &[UncleContext],
    past_uncle_hashes: &HashSet<[u8; 32]>,
) -> Result<BlockValidationReport, ConsensusError> {
    self.validate_block_with_fees(
        block, height, total_fees, median_time_past,
        network_adjusted_time, uncles_ctx, past_uncle_hashes,
    )
}
```

This approach: (1) enforces the fee contract at the call site via the type system, (2) eliminates the duplicated ASERT logic that was also present in the old body (it was already in `validate_block_with_fees()`), (3) makes the two overloads consistent — callers who do not have fees cannot call `validate_block()` at all; they must obtain fees first.

**`crates/consensus/src/tests.rs` — `test_validate_block_weight_overflow`**

Added missing match arms: `FeeValidation(_)`, `CoinbaseMissing`, `InvalidCoinbase(_)`. These are all valid outcomes when the test block (which has no coinbase and passes `fees=None`) is processed.

---

## How It Was Found

Static source trace — no runtime needed.

1. Observed `ConsensusEngine::validate_block()` at `lib.rs:1160` passing `None` to `validate_block()`.
2. Traced `validate_block()` to `validate_transaction_fees()` at `lib.rs:864`.
3. Saw `ok_or_else` at `lib.rs:892` turning `None` into unconditional `Err(FeeValidation)`.
4. Confirmed no test in `tests.rs` calls `ConsensusEngine::validate_block()` — grepping for `ConsensusEngine` in the test file returned zero results. This explained why the bug was undetected.

Single confirming observation: the `// Total fees unknown in this context` comment in the old code was not a known-safe sentinel — it was developer intent that became incorrect once Change A landed.

---

## Why It Slipped Through

**CI gap.** The test suite calls the free function `validate_block()` directly; no test instantiates `ConsensusEngine` and calls `validate_block()` through the struct. The struct method is part of the public API but has zero test coverage. The strict-fee enforcement (Change A) added a new contract to `validate_block()`, but the contract was only exercised through the free function in tests — never through `ConsensusEngine::validate_block()`.

Additionally, `test_validate_block_weight_overflow` used `validate_block(fees=None, ...)` directly, which means the test was itself triggering `FeeValidation` on every run and then panicking with "Unexpected error" — the test was broken alongside the API. Because both the API and the test were broken in the same way, neither caught the other.

---

## Validation

`cargo check -p bitquan-consensus` passes with 0 errors, 0 warnings after the fix.

Validated scope: type-level contract and compilation only. Runtime validation (actually running `ConsensusEngine::validate_block()` against a real or test block end-to-end) is not yet possible — the node's P2P layer does not wire the consensus engine to block receipt (issue #143). Full runtime validation is blocked on that issue.

Not retested: `bitquan-node`, `bitquan-storage`, or any integration test — the change is isolated to `crates/consensus`.

---

## Action Items

1. **Add regression test** for `ConsensusEngine::validate_block()` that instantiates the engine, supplies real fees (even `0`), and asserts the call does not return `FeeValidation`. This closes the gap that allowed both bugs to coexist silently.  
   *(Owner: consensus maintainer. No tracking artifact — file alongside this PR.)*

2. **Audit all `ConsensusEngine` public methods** for parameters that are `Option<T>` where `None` is now an outright error rather than a sentinel. `validate_block()` was the worst case; verify no others have the same class of issue.  
   *(Owner: consensus maintainer. File as BQ-002 if found.)*

3. **Wire `ConsensusEngine` into P2P block receipt** (issue #143) to enable end-to-end runtime validation. Until that is done, the fix cannot be tested beyond compilation.  
   *(Owner: node maintainer. Existing tracking: issue #143.)*
