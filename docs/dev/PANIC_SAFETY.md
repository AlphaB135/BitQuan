# Panic Safety Policy

## Overview

BitQuan strives for panic-free production code. This document defines the policy for handling errors safely.

## ❌ Avoid Panics

### Never Use in Production Code

```rust
// ❌ BAD - Can panic
let value = some_option.unwrap();
let result = some_result.expect("failed");
let item = vec[index];  // Unchecked indexing
```

### ✅ Use Result-Based Error Handling

```rust
// ✅ GOOD - Returns Result
let value = some_option.ok_or(Error::NotFound)?;
let result = some_result?;
let item = vec.get(index).ok_or(Error::OutOfBounds)?;
```

## Safe Patterns

### 1. Option Handling

```rust
// ❌ BAD
let block = chain.get_block(hash).unwrap();

// ✅ GOOD
let block = chain.get_block(hash)
    .ok_or(Error::BlockNotFound(hash))?;
```

### 2. Result Propagation

```rust
// ❌ BAD
fn process() {
    let data = read_file().expect("failed to read");
    // ...
}

// ✅ GOOD
fn process() -> Result<()> {
    let data = read_file()
        .context("Failed to read configuration")?;
    // ...
    Ok(())
}
```

### 3. Collection Access

```rust
// ❌ BAD
let first = blocks[0];

// ✅ GOOD
let first = blocks.first()
    .ok_or(Error::EmptyChain)?;
```

### 4. Arithmetic Operations

```rust
// ❌ BAD - Can overflow
let total = a + b + c;

// ✅ GOOD - Checked arithmetic
let total = a.checked_add(b)
    .and_then(|sum| sum.checked_add(c))
    .ok_or(Error::Overflow)?;
```

## When Panics Are Acceptable

### 1. Tests

```rust
#[test]
fn test_something() {
    let result = function_under_test().unwrap();
    assert_eq!(result, expected);  // OK in tests
}
```

### 2. Invariant Violations

```rust
// When fundamental assumptions are violated
if height > 0 && prev_hash.is_none() {
    panic!("Invariant violated: block {} has no parent", height);
}
```

### 3. Static Initialization

```rust
// Compile-time guaranteed to succeed
static REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^\d+$").unwrap()  // OK - pattern is valid
});
```

## Panic Recovery

### Node-Level Panic Hook

```rust
fn main() -> Result<()> {
    // Install panic hook for crash reporting
    std::panic::set_hook(Box::new(|panic_info| {
        eprintln!("💥 PANIC: {}", panic_info);
        // Log to file, send metrics, etc.
    }));

    run_node()
}
```

### Catch Unwind (Use Sparingly)

```rust
use std::panic;

let result = panic::catch_unwind(|| {
    potentially_panicking_operation()
});

match result {
    Ok(value) => handle_success(value),
    Err(_) => {
        log::error!("Operation panicked, recovering...");
        handle_panic_recovery()
    }
}
```

## Audit Process

### 1. Automated Audit

```bash
# Find all unwrap/expect in production code
./scripts/audit-panics.sh
```

### 2. Manual Review

Review each unwrap/expect:
- [ ] Is it in test code? (OK)
- [ ] Is it checking a static invariant? (Document why)
- [ ] Can it be replaced with `?` operator? (Do it)
- [ ] Can it be replaced with `unwrap_or_default()`? (Consider)

### 3. Continuous Monitoring

```bash
# Pre-commit hook
if git diff --cached --name-only | grep -q '\.rs$'; then
    if git diff --cached | grep -q "\.unwrap()\|\.expect("; then
        echo "⚠️  New unwrap/expect detected. Review for safety."
    fi
fi
```

## Error Types

### Define Comprehensive Errors

```rust
#[derive(Debug, thiserror::Error)]
pub enum ConsensusError {
    #[error("Block not found: {0}")]
    BlockNotFound(BlockHash),

    #[error("Invalid proof of work")]
    InvalidPoW,

    #[error("Arithmetic overflow in {operation}")]
    Overflow { operation: String },
}
```

### Context-Rich Errors

```rust
use anyhow::Context;

fn load_config(path: &Path) -> Result<Config> {
    let contents = fs::read_to_string(path)
        .with_context(|| format!("Failed to read config: {:?}", path))?;

    serde_json::from_str(&contents)
        .context("Failed to parse config JSON")?
}
```

## Testing for Panics

### Should-Panic Tests

```rust
#[test]
#[should_panic(expected = "index out of bounds")]
fn test_invalid_access() {
    let vec = vec![1, 2, 3];
    let _ = vec[10];  // Should panic
}
```

### Catching Expected Panics

```rust
#[test]
fn test_invariant_violation() {
    let result = std::panic::catch_unwind(|| {
        // Code that should panic
        validate_invariant(invalid_data);
    });

    assert!(result.is_err());
}
```

## Fuzzing

Fuzzing helps discover panic conditions:

```bash
# Run fuzz targets
cd fuzz
cargo fuzz run block_parser -- -max_total_time=300
```

Example fuzz target:

```rust
#[macro_use]
extern crate libfuzzer_sys;

fuzz_target!(|data: &[u8]| {
    // Should never panic, even with malformed input
    let _ = Block::deserialize(data);
});
```

## Metrics

Track panic rate in production:

```rust
use metrics::counter;

fn install_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        counter!("node.panics").increment(1);
        // Log panic info
    }));
}
```

## Migration Guide

### Step 1: Identify

```bash
./scripts/audit-panics.sh
```

### Step 2: Categorize

- Tests: OK, keep as-is
- Invariants: Document why
- Production: Needs fixing

### Step 3: Fix

```rust
// Before
fn get_block(hash: &Hash) -> Block {
    self.blocks.get(hash).unwrap()
}

// After
fn get_block(&self, hash: &Hash) -> Result<Block> {
    self.blocks.get(hash)
        .cloned()
        .ok_or(Error::BlockNotFound(*hash))
}
```

### Step 4: Test

```rust
#[test]
fn test_missing_block() {
    let chain = Chain::new();
    let result = chain.get_block(&random_hash());
    assert!(matches!(result, Err(Error::BlockNotFound(_))));
}
```

## Status

**Current State:**
- 104 unwrap/expect found in critical modules
- ~95% are in test code (acceptable)
- ~5 in production code (need review)

**Action Items:**
- [ ] Review production unwrap/expect
- [ ] Replace with Result where possible
- [ ] Document remaining invariants
- [ ] Add fuzz targets
- [ ] Set up panic monitoring

## References

- [Rust Error Handling](https://doc.rust-lang.org/book/ch09-00-error-handling.html)
- [anyhow crate](https://docs.rs/anyhow/)
- [thiserror crate](https://docs.rs/thiserror/)

---

**Policy Version**: 1.0
**Last Updated**: 2025-11-02
**Next Review**: Before v1.0 release
