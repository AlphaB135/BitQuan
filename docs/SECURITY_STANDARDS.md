# Security Standards & Best Practices

**Status:** v1.0-audit-20251122 (Pre-Mainnet)
**Last Updated:** 2025-01-22

---

## 🎯 Overview

This document defines BitQuan's security coding standards. All production code MUST follow these guidelines.

**Current Compliance:** ✅ 100% (All security fixes applied, see PR #80, #83)

---

## 🔴 A. Error Handling - Zero Unwrap Policy

### ❌ FORBIDDEN in Production Code

```rust
// ❌ BAD - Will panic on None/Err
let value = map.get(&key).unwrap();
let num = s.parse::<u64>().unwrap();
let data = mutex.lock().unwrap();
let bytes: [u8; 32] = vec.try_into().unwrap();
```

### ✅ REQUIRED: Use ? Operator

```rust
// ✅ GOOD - Propagates error
let value = map.get(&key).ok_or(Error::KeyNotFound)?;
let num = s.parse::<u64>().map_err(Error::ParseError)?;
let data = mutex.lock().map_err(|e| Error::LockPoisoned(e.to_string()))?;
```

### ✅ ACCEPTABLE: With SAFETY Comment

```rust
// ✅ GOOD - Justified unwrap with explanation
// SAFETY: This vector is always 32 bytes because we allocated it at line 42
// and verified the length at line 45
let bytes: [u8; 32] = vec.try_into().unwrap();

// SAFETY: HRP is compile-time constant validated by tests
let hrp = Hrp::parse("bq").expect("network HRP is valid");
```

**SAFETY Comment Requirements:**
1. Must explain WHY the unwrap cannot fail
2. Must reference specific invariants or checks
3. Must be on the line immediately before the unwrap

### ✅ ACCEPTABLE: In Test Code

```rust
#[cfg(test)]
fn test_transaction_validation() {
    let tx = create_test_tx().unwrap(); // OK in tests
    assert!(validate(tx).is_ok());
}
```

### 📊 Current Status

**As of 2025-11-08:**
- Total unwrap/expect in production: **430** ❌
- SAFETY comments: **~5**
- Compliance rate: **1%**
- **Target:** <10 unwraps with SAFETY comments

**Action Plan:**
1. Phase 1 (v0.0.2): Document standard, fix critical paths
2. Phase 2 (v0.0.3): Systematic audit, fix 50% of unwraps
3. Phase 3 (v0.1.0): Full compliance, <10 unwraps total

---

## 🔴 B. Arithmetic Operations - Checked Math

### ❌ FORBIDDEN: Unchecked Arithmetic

```rust
// ❌ BAD - Can overflow
let total = a + b;
let fee = input_value - output_value;
let weight = count * WEIGHT_PER_SIG;
let sum: u64 = vec.iter().map(|x| x.value).sum();
```

### ✅ REQUIRED: Checked Operations

```rust
// ✅ GOOD - Addition with overflow check
let total = a.checked_add(b)
    .ok_or(Error::Overflow("total calculation"))?;

// ✅ GOOD - Subtraction with underflow check
let fee = input_value.checked_sub(output_value)
    .ok_or(Error::Underflow("fee calculation"))?;

// ✅ GOOD - Multiplication with overflow check
let weight = count.checked_mul(WEIGHT_PER_SIG)
    .ok_or(Error::Overflow("weight calculation"))?;

// ✅ GOOD - Sum with try_fold
let sum = vec.iter().try_fold(0u64, |acc, x| {
    acc.checked_add(x.value)
        .ok_or(Error::Overflow("total output value"))
})?;
```

### ✅ ACCEPTABLE: Saturating for Counters/Metrics

```rust
// ✅ GOOD - Counters that shouldn't fail
self.block_count = self.block_count.saturating_add(1);
self.peer_count = self.peer_count.saturating_sub(1);
```

**When to use saturating_*:**
- Counters (block height, peer count, etc.)
- Metrics and statistics
- Non-critical values where clamping is acceptable

**When to use checked_*:**
- Money/value calculations
- Fee calculations
- Weight/size calculations
- Consensus-critical math

### 📊 Current Status

**As of 2025-11-08:**
- checked_* usage: **91 instances** ✅
- try_fold usage: **14 instances** ✅
- Compliance: **~80%** ⚠️

**Missing:**
- Counter increments (should use saturating_*)
- Some string/index operations
- Comprehensive overflow tests

---

## 🔴 C. Cryptographic Operations

### C.1 Random Number Generation

#### ❌ FORBIDDEN: Thread RNG

```rust
// ❌ BAD - Not cryptographically secure
use rand::thread_rng;
let mut rng = thread_rng();
rng.fill_bytes(&mut buffer);
```

#### ✅ REQUIRED: OS RNG

```rust
// ✅ GOOD - Cryptographically secure
use rand::rngs::OsRng;
OsRng.fill_bytes(&mut buffer);

// ✅ GOOD - Direct getrandom
use getrandom::getrandom;
getrandom(&mut buffer).map_err(Error::RngFailure)?;
```

**Status:** ✅ **COMPLIANT** (No thread_rng found, 10 files use OsRng)

---

### C.2 Constant-Time Comparison

#### ❌ FORBIDDEN: Direct Comparison

```rust
// ❌ BAD - Timing attack vulnerable
if signature == expected_signature {
    return Ok(());
}

// ❌ BAD - Short-circuits on first difference
if password.as_bytes() == stored_hash {
    return Ok(());
}
```

#### ✅ REQUIRED: Constant-Time Comparison

```rust
// ✅ GOOD - Constant-time comparison
use subtle::ConstantTimeEq;

if signature.ct_eq(expected_signature).into() {
    return Ok(());
} else {
    return Err(Error::InvalidSignature);
}

// ✅ GOOD - MAC verification
use subtle::ConstantTimeEq;
let valid = mac.ct_eq(&expected_mac).into();
if !valid {
    return Err(Error::InvalidMac);
}
```

**When to use:**
- Signature verification
- MAC verification
- Password/hash comparison
- Any secret comparison

**Status:** ⚠️ **PARTIAL** (Only 1 file uses subtle::ConstantTimeEq)

**TODO:**
- [ ] Add to signature verification in consensus
- [ ] Add to MAC verification in wallet
- [ ] Add to password comparison in RPC

---

### C.3 Zeroize Sensitive Data

#### ❌ FORBIDDEN: Leaving Secrets in Memory

```rust
// ❌ BAD - Password remains in memory
let password = String::from("secret");
// Use password...
// Compiler may optimize out memset(0)
```

#### ✅ REQUIRED: Explicit Zeroize

```rust
// ✅ GOOD - Explicitly clear memory
use zeroize::Zeroize;

let mut password = String::from("secret");
// Use password...
password.zeroize(); // Guaranteed to clear memory

// ✅ GOOD - Automatic zeroize on drop
use zeroize::Zeroizing;

let password = Zeroizing::new(String::from("secret"));
// Automatically zeroized when dropped
```

**What to zeroize:**
- Passwords
- Private keys
- Mnemonics
- Encryption keys
- Session tokens
- Any sensitive material

**Status:** ✅ **GOOD** (6 files use Zeroize)

---

## 🔴 D. Input Validation

### ✅ REQUIRED: Validate All External Input

```rust
pub fn validate_transaction(tx: &Transaction) -> Result<(), Error> {
    // 1. Size limits (fail fast)
    if tx.inputs.len() > MAX_INPUTS {
        return Err(Error::TooManyInputs);
    }
    if tx.outputs.len() > MAX_OUTPUTS {
        return Err(Error::TooManyOutputs);
    }

    // 2. Value validation (checked arithmetic)
    let total_out = tx.outputs.iter().try_fold(0u64, |acc, out| {
        if out.value == 0 {
            return Err(Error::ZeroValueOutput);
        }
        acc.checked_add(out.value)
            .ok_or(Error::Overflow("total output value"))
    })?;

    // 3. Script size validation
    for input in &tx.inputs {
        if input.script.len() > MAX_SCRIPT_SIZE {
            return Err(Error::ScriptTooLarge);
        }
    }

    // 4. Network context (replay protection)
    if tx.network_id != expected_network {
        return Err(Error::WrongNetwork {
            expected: expected_network,
            got: tx.network_id,
        });
    }

    // 5. Signature count limits
    let sig_count = tx.count_signatures()
        .ok_or(Error::TooManySignatures)?;
    if sig_count > MAX_SIGNATURES_PER_TX {
        return Err(Error::TooManySignatures);
    }

    Ok(())
}
```

### Validation Checklist

**For every external input (network, RPC, file):**

- [ ] **Size limits** - Check length before processing
- [ ] **Value ranges** - Check min/max bounds
- [ ] **Format validation** - Verify structure/encoding
- [ ] **Replay protection** - Check network ID/context
- [ ] **Resource limits** - Prevent DoS (script size, signature count)
- [ ] **Arithmetic safety** - Use checked operations
- [ ] **Error messages** - Return specific, actionable errors

**Status:** ⚠️ **PARTIAL** (Good validators exist, need comprehensive audit)

---

## 📋 Testing Requirements

### A. Error Handling Tests

```rust
#[test]
fn test_parse_error_handling() {
    // Test that errors are propagated, not panicked
    let result = parse_invalid_data();
    assert!(result.is_err());
}

#[test]
#[should_panic(expected = "SAFETY violation")]
fn test_safety_comment_justification() {
    // If you have a SAFETY comment, add a test proving it
    // This test should panic if the invariant is violated
}
```

### B. Overflow Tests

```rust
#[test]
fn test_value_overflow() {
    let large = u64::MAX;
    let result = calculate_total(large, 1);
    assert!(matches!(result, Err(Error::Overflow(_))));
}

#[test]
fn test_underflow() {
    let small = 0u64;
    let result = calculate_fee(small, 1);
    assert!(matches!(result, Err(Error::Underflow(_))));
}
```

### C. Timing Attack Tests

```rust
#[test]
fn test_constant_time_comparison() {
    use std::time::Instant;

    let sig1 = [0u8; 64];
    let sig2 = [1u8; 64];
    let sig3 = sig1.clone();

    // Measure time for mismatch
    let start = Instant::now();
    let _ = verify_signature_ct(&sig1, &sig2);
    let mismatch_time = start.elapsed();

    // Measure time for match
    let start = Instant::now();
    let _ = verify_signature_ct(&sig1, &sig3);
    let match_time = start.elapsed();

    // Times should be similar (within 10%)
    let diff = mismatch_time.as_nanos().abs_diff(match_time.as_nanos());
    let avg = (mismatch_time.as_nanos() + match_time.as_nanos()) / 2;
    assert!(diff < avg / 10, "Timing difference too large");
}
```

---

## 🚨 CI/CD Enforcement

### Pre-Commit Hooks

```bash
#!/bin/bash
# .git/hooks/pre-commit

# 1. Check for unwrap without SAFETY
if git diff --cached | grep -E '\.unwrap\(\)|\.expect\(' | grep -v 'SAFETY:'; then
    echo "❌ Found unwrap/expect without SAFETY comment"
    exit 1
fi

# 2. Check for thread_rng
if git diff --cached | grep 'thread_rng'; then
    echo "❌ Found thread_rng usage (use OsRng instead)"
    exit 1
fi

# 3. Check for unchecked arithmetic in critical paths
if git diff --cached -- crates/consensus crates/mempool | grep -E '\+.*\w+\s*[\+\-\*]\s*\w+' | grep -v 'checked_'; then
    echo "⚠️  Warning: Possible unchecked arithmetic in critical code"
fi
```

### CI Checks

```yaml
# .github/workflows/security.yml
name: Security Checks

on: [push, pull_request]

jobs:
  security:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2

      - name: Check for unwrap without SAFETY
        run: |
          ./scripts/check_unwraps.sh

      - name: Check for thread_rng
        run: |
          if rg "thread_rng" crates/; then
            echo "Found thread_rng usage"
            exit 1
          fi

      - name: Run security tests
        run: cargo test --all -- --test-threads=1 security::
```

---

## 📚 References

- [Rust Error Handling](https://doc.rust-lang.org/book/ch09-00-error-handling.html)
- [Checked Arithmetic RFC](https://rust-lang.github.io/rfcs/0560-integer-overflow.html)
- [Zeroize Documentation](https://docs.rs/zeroize/)
- [Subtle Crate](https://docs.rs/subtle/)
- [OWASP Cryptographic Storage](https://cheatsheetseries.owasp.org/cheatsheets/Cryptographic_Storage_Cheat_Sheet.html)

---

## 🎯 Compliance Roadmap

### v0.0.2-alpha (Current)
- ✅ Document security standards
- ⚠️ Current compliance: 65/100
- ❌ Known issues: 430 unwraps, missing constant-time

### v0.0.3-alpha (2 weeks)
- [ ] Fix 50% of unwraps (215 → <100)
- [ ] Add constant-time signature verification
- [ ] Add overflow tests for all critical paths
- [ ] Target: 80/100 compliance

### v0.1.0 (1 month)
- [ ] Full unwrap audit (<10 with SAFETY)
- [ ] Complete constant-time implementation
- [ ] Comprehensive test coverage
- [ ] CI enforcement enabled
- [ ] Target: 95/100 compliance

### v1.0.0 (Mainnet)
- [ ] External security audit passed
- [ ] 100% compliance verification
- [ ] Formal verification of critical paths
- [ ] Target: 100/100 compliance

---

**Last Updated:** 2025-11-08
**Next Review:** Before v0.0.3-alpha release
