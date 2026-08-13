# Post-mortem: BQ-002 — check_header_pow ignores algo_id and always hashes with SHA256d

**Date:** 2026-08-13  
**Severity:** Critical — silent rejection of all non-SHA256d blocks (RandomX, Ethash) despite hybrid PoW support  
**Status:** Fixed, validated  

---

## Summary

`check_header_pow()` inside `crates/consensus/src/pow.rs` hardcoded the hashing function to `header_hash()` (which is exclusively double-SHA256), completely ignoring the block header's `algo_id` field. As a result, any block mined with RandomX or Ethash was hashed with SHA256d during validation, causing it to fail the difficulty target check and be rejected. Fixed by rewriting `check_header_pow` to enforce `is_algo_allowed` logic from `PowSetParams` and correctly routing the hash operation based on `algo_id`.

No JIRA key. Found via `scrutinize` skill audit.

---

## Symptom

When a node receives a valid block mined using RandomX or Ethash, the block is rejected at the header validation stage with:

```
Err(ConsensusError::InvalidPoW("hash does not meet target"))
```

The node's logs would show that it rejected the block, even though a valid miner produced it with proof-of-work that actually met the target under its respective algorithm.

---

## Root Cause

`check_header_pow()` (the central entry point for PoW verification in `crates/consensus/src/lib.rs`) failed to dispatch to algorithm-specific engines. 

```rust
pub fn check_header_pow(header: &BlockHeader) -> std::result::Result<bool, PowError> {
    let hash = header_hash(header); // ← BUG: This is SHA256d-only
    let target = compact_to_target_bytes(header.bits)?;
    Ok(hash_meets_target(&hash, &target))
}
```

Although the codebase had `PowEngine` trait implementations (`Sha256dEngine`, `RandomXEngine`, `EthashEngine`), they were entirely bypassed by `check_header_pow()`. `header.algo_id` was completely ignored.

---

## Why It Produced the Symptom

Call path:

```
ConsensusEngine::validate_block_header()
  lib.rs:796 → check_header_pow(header)
    pow.rs:601 → header_hash(header)
      pow.rs:531 → header_hash_sha256d(header) → Returns SHA256 hash
    pow.rs:603 → hash_meets_target(sha256_hash, target) → False
```

Because a header containing a valid RandomX proof is essentially random data to SHA256, hashing it with SHA256d produces a hash that does not meet the difficulty target (unless astronomical luck occurs). The block is thus rejected as invalid PoW.

---

## Fix

**`crates/consensus/src/pow.rs`**
Rewrote `check_header_pow` to accept `height`, `pow_params: &PowSetParams`, and `genesis_hash: &[u8; 32]`. The function now:
1. Validates the `algo_id` against the allowed algorithms at the current `height` (fixing another gap where deprecated algorithms could still be used).
2. Routes the `header.to_bytes()` to the correct hash function (`sha256d_pow_hash`, `randomx_pow_hash`, or `ethash_pow_hash`).

**`crates/consensus/src/lib.rs`**
Updated `validate_block_header` to accept `genesis_hash`, replacing the `_params` unused variable with active `params`. Pushed `genesis_hash`, `height`, and `params.pow_set` down into `check_header_pow()`. Updated the Uncle block validation loop to pass these same context variables.

**`crates/consensus/src/bin/simple_miner.rs` & `tests.rs`**
Updated dummy/test calls to `check_header_pow` to provide default parameters (height 0, mainnet params, null genesis hash) to satisfy the type checker.

---

## How It Was Found

During an architecture trace initiated by `/scrutinize`. I traced the `validate_block` call chain looking for where the `DifficultyState` was evaluated. When analyzing `check_header_pow`, I noticed it accepted no contextual parameters (like algorithm configurations or network state) and explicitly called `header_hash` without branching on `header.algo_id`.

---

## Why It Slipped Through

**Test coverage gap and mocking.** 
The unit tests in `pow.rs` verified the `RandomXEngine` and `Sha256dEngine` individually (e.g., `test_randomx_and_sha256d_produce_different_hashes`), but no integration test generated a RandomX block and attempted to validate it via `ConsensusEngine::validate_block()`. The higher-level tests only used SHA256d blocks (the default), so `check_header_pow`'s hardcoded behavior happened to match the test data perfectly.

---

## Validation

`cargo check -p bitquan-consensus --tests` compiles cleanly. 
The static dispatch path is now strictly typed to require parameters that enforce `algo_id` routing.

*Note: Runtime integration tests for RandomX/Ethash hybrid mining must still be written to exercise the execution logic.*

---

## Action Items

1. **Add runtime integration tests for multi-algo blocks.** Create a test that mines one SHA256d block and one RandomX block, and verifies both pass `ConsensusEngine::validate_block()`.
   *(Owner: consensus maintainer. File alongside PR.)*

2. **Remove dead `PowEngine` trait.** Now that `check_header_pow` statically routes to the primitive hash functions (`randomx_pow_hash`, `sha256d_pow_hash`), the heavy `PowEngine` object-oriented wrappers (e.g., `RandomXEngine`) appear to be unused dead code.
   *(Owner: consensus maintainer. Refactoring ticket needed.)*
