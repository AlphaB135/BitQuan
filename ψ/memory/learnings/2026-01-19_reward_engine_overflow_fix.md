# Reward Engine u64 Overflow Bug - Root Cause & Fix

**Date**: 2026-01-19
**Context**: Critical u64 overflow in `reward_engine.rs` when tracking block rewards

## What We Discovered

### The Bug
```rust
// BEFORE (BROKEN):
const INITIAL_REWARD: u128 = 50_000_000_000_000_000_000;  // 50 BQ in qbits
total_distributed: Arc<AtomicU64>,  // max = 18.4e18

self.total_distributed.fetch_add(amount as u64, ...);  // OVERFLOW!
// 50e18 > 18.4e18 → overflow on first block!
```

**Impact**: First block reward (50 BQ) caused immediate overflow → total rewards tracking broken

## Why It Happened

1. **Unit Mismatch**: Block rewards calculated in `qbits` (1e18 precision)
2. **Wrong Storage Type**: `AtomicU64` can only hold ~18.4e18 qbits = ~18.4 BQ
3. **Truncation Cast**: `amount as u64` silently truncated large values

## The Fix: Smart Scaling

### Solution: Store in BQ, Return in qbits

```rust
// AFTER (FIXED):
const QBITS_PER_BQ: u128 = 1_000_000_000_000_000_000;

/// Total rewards distributed counter (in BQ, not qbits, to fit in u64).
/// Stored as BQ to avoid u64 overflow (u64::MAX = ~18.4 billion BQ).
total_distributed: Arc<AtomicU64>,

// Store: scale down to BQ
let amount_bq = amount / QBITS_PER_BQ;
self.total_distributed.fetch_add(amount_bq as u64, ...);

// Return: scale back to qbits (from DB for precision)
pub fn total_distributed(&self) -> u128 {
    self.db.total_rewards().unwrap_or(0)  // Exact value from DB
}
```

### Why This Works

| Metric | Value |
|--------|-------|
| Initial reward | 50 BQ |
| Storage per block | 50 (as u64) |
| u64::MAX | ~18.4 billion BQ |
| Blocks until overflow | ~368 million blocks (~7,000 years at 10 min/block) |

## Key Insights

### 1. Scale at Storage Boundaries
- **Input**: qbits (high precision)
- **Storage**: BQ (compact, fits in u64)
- **Output**: qbits (from DB, exact)

### 2. Precision Loss is OK for Counter
- Counter is for statistics, not accounting
- DB stores exact qbits values
- `total_distributed()` queries DB for precision

### 3. Alternative Approaches Considered

| Approach | Pros | Cons |
|----------|------|------|
| AtomicU128 | Exact values | **Unstable** feature (nightly only) |
| Mutex<u128> | Exact values | Performance hit, nightly needed |
| **BQ Scaling** | Stable, fast, sufficient | Small precision loss (acceptable) |

### 4. Test Expectations Were Also Wrong

Found tests expecting wrong BQ values:
```rust
// WRONG (0.0005 BQ):
assert_eq!(balance.spendable, 500_0000_0000);

// CORRECT (500 BQ = 10 blocks × 50 BQ):
assert_eq!(balance.spendable, 500_000_000_000_000_000_000);
```

**Lesson**: Verify test expectations match reality!

## How To Apply

### For Similar Issues

1. **Check Type Bounds First**
   ```bash
   # Find potential overflow points
   rg "as u64|as u32|as usize" --type rust
   ```

2. **Use Appropriate Scaling**
   - Identify natural units (BQ, satoshis, etc.)
   - Scale at boundaries, not internally
   - Document scaling clearly

3. **Query Source of Truth**
   ```rust
   // Counter for stats: scaled, compact
   total_distributed: Arc<AtomicU64>  // BQ-scale

   // Exact values: query DB
   pub fn total_distributed(&self) -> u128 {
       self.db.total_rewards().unwrap_or(0)  // qbits, exact
   }
   ```

### Detection Checklist

- [ ] Review all `as u64/u32/usize` casts for truncation
- [ ] Check constants vs type limits (u64::MAX = 18.4e18)
- [ ] Verify test expectations match implementation
- [ ] Document scaling decisions in code comments
- [ ] Run tests with realistic values (not 1000, use 1e18)

## Files Modified

| File | Change |
|------|--------|
| `src/reward_engine.rs` | BQ scaling implementation |
| `tests/reward_engine.rs` | Fixed test values (3e18 not 3000) |
| `tests/reward_maturity_test.rs` | Fixed BQ expectations (500e18 not 5e11) |

## Results

- **Overflow**: Fixed ✅
- **Tests**: 76 passed, 1 ignored (stub) ✅
- **Precision**: Maintained via DB queries ✅
- **Capacity**: 368M blocks before overflow ✅

## Tags
`overflow` `u64` `scaling` `reward-engine` `rust` `bug-fix`
