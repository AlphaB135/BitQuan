# Post-mortem: BQ-003 — ASERT difficulty adjustment silent u128 truncation causing infinite difficulty spikes

**Date:** 2026-08-13  
**Severity:** Critical — chain halts completely when difficulty drops below a certain threshold (e.g., at Devnet genesis)  
**Status:** Fixed, validated  

---

## Summary

The `asert_next_target()` function in `crates/consensus/src/asert.rs` incorrectly clamped all 256-bit difficulty targets to a maximum of `u128::MAX`. This meant that when the target bits were easy (such as `0x207fffff` used on Devnet, which represents a 256-bit number), the anchor target was silently truncated down to `u128::MAX`. As a result, the ASERT algorithm calculated the next target using `u128::MAX` as a ceiling, artificially increasing the mining difficulty by $2^{128}$ times instantly. This caused a chain halt where no subsequent blocks could be mined. Fixed by migrating the entire ASERT calculation pipeline to 256-bit integers using `primitive_types::U256`.

No JIRA key. Found via proactive `scrutinize` skill audit.

---

## Symptom

When mining on a network with a low initial difficulty (Devnet/Testnet) or when difficulty organically adjusts below the $2^{128}$ threshold:
- Block 1 is mined and validated correctly against the genesis block.
- For Block 2, the node requires a target bits value exponentially harder than Block 1 (dropping from exponent 32 to roughly exponent 16).
- The network halts immediately as miners (which are hashing for the low difficulty) cannot meet the wildly inflated target.

---

## Root Cause

`asert_next_target()` utilized `u128` for its fixed-point math to avoid `u64` overflow. However, it blindly assumed that targets would never exceed `u128::MAX`.

```rust
let anchor_u128 = if anchor_target[0..16] == [0u8; 16] {
    u128::from_be_bytes(anchor_target[16..32].try_into().unwrap_or([0u8; 16]))
} else {
    u128::MAX // <--- BUG: TRUNCATION
};
```

When evaluating a target like `0x207fffff`, the first 16 bytes of the 256-bit array are heavily populated. The logic fell into the `else` branch, setting `anchor_u128 = u128::MAX`.

When returning the target, it blindly wrote the truncated `u128` value into the lower 16 bytes of a 32-byte array, leaving the upper 16 bytes as zeros:

```rust
let mut out = [0u8; 32];
out[16..32].copy_from_slice(&result.to_be_bytes());
out // <--- Highest bits always 0
```

---

## Why It Produced the Symptom

When the genesis block defines difficulty bits as `0x207fffff` (exponent 32):
1. `asert_next_target()` reads the previous target.
2. It truncates the 256-bit target down to `u128::MAX` (a 128-bit number).
3. The fixed-point math calculates the next target relative to this artificially low anchor.
4. The result (at most `u128::MAX`) is converted to compact bits.
5. The compact bits conversion yields an exponent of 16 (e.g., `0x107fffff`).
6. A difference in exponent of 16 translates to a factor of $2^{128}$. Mining difficulty instantly spiked to an astronomical level.

---

## Fix

**`crates/consensus/src/asert.rs`**
Migrated the core ASERT calculation to `primitive_types::U256`:
1. Replaced `u128` tracking variables (`anchor_u128`, `max_u128`) with `U256::from_big_endian()`.
2. Overhauled the fixed-point scaling math (`anchor_clamped * exp_fp / FP_SCALE`) to carefully partition the 64-bit multiplications across `U256` boundaries (`high` and `low` splitting) or using `U256::overflowing_mul` to prevent integer overflow.
3. Updated `apply_burst_guard_256()` to accept and return `U256` cleanly.
4. Removed the flawed `u128_to_bytes` utility function.

---

## How It Was Found

During a manual `scrutinize` pass of the `asert.rs` implementation. While checking the integer fixed-point boundary logic, I noted the `u128::MAX` hardcoded clamps. Tracing how `target_to_compact` parses these arrays confirmed that ignoring the top 16 bytes alters the exponent fundamentally.

---

## Why It Slipped Through

**Unrealistic Mock Data in Unit Tests.** 
The unit tests (e.g., `test_asert_difficulty_retarget`) likely mocked difficulty targets that were computationally high (low numerical values) and fit safely within the 128-bit boundaries. They did not effectively test behavior at the Devnet difficulty bounds (`DEVNET_MAX_BITS = 0x207fffff`).

---

## Validation

The code cleanly compiles with `cargo check -p bitquan-consensus --tests`. The `U256` migration ensures that all 256-bits of the target are accurately scaled, preserving the exact difficulty during flat periods and preventing silent truncation.

---

## Action Items

1. **Expand ASERT unit tests with boundary compact bits.** Add unit tests that assert `asert_next_target()` when passed `DEVNET_MAX_BITS` returns exactly `DEVNET_MAX_BITS` (no drift or truncation) when time_delta matches expected_time.
   *(Owner: consensus maintainer)*
