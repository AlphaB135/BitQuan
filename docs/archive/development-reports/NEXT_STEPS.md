# 🎯 Next Steps - Phase 1B Action Plan

**Current Status:** Phase 1A Complete ✅
**Next Phase:** Phase 1B - Consensus & Storage Hardening
**Timeline:** 1-2 weeks
**Goal:** Reduce unwrap() from 303 → 150 (50% reduction)

---

## 📋 Quick Start

```bash
# Create new branch for Phase 1B
git checkout -b security/p1b-consensus-hardening

# Verify current state
cargo build --all
cargo test --all
```

---

## 🎯 Target Files (Priority Order)

### 1. Consensus Module (High Priority)
**File:** `crates/consensus/src/fork.rs`
**Unwraps:** ~27
**Complexity:** High
**Time Estimate:** 2-3 hours

**Key Areas:**
- Block validation logic
- Fork choice rules
- Reorganization handling

**Refactoring Strategy:**
```rust
// Before
let block = store.get_block(&hash).unwrap();

// After
let block = store.get_block(&hash)
    .ok_or(Error::BlockNotFound)?;
```

### 2. Mempool Module (High Priority)
**File:** `crates/mempool/src/lib.rs`
**Unwraps:** ~21
**Complexity:** Medium
**Time Estimate:** 1-2 hours

**Key Areas:**
- Transaction insertion
- Fee calculation
- Eviction policy

### 3. Storage Module (Medium Priority)
**File:** `crates/storage/src/rocksdb_store.rs`
**Unwraps:** ~15
**Complexity:** Medium
**Time Estimate:** 1-2 hours

**Key Areas:**
- Database operations
- Serialization/deserialization
- UTXO management

### 4. Wallet Module (Medium Priority)
**File:** `crates/wallet/src/multisig.rs`
**Unwraps:** ~37
**Complexity:** High
**Time Estimate:** 3-4 hours

**Key Areas:**
- Signature collection
- Transaction building
- Configuration validation

---

## 🔧 Refactoring Patterns

### Pattern 1: Option::unwrap() → ok_or()
```rust
// ❌ Before
let value = map.get(&key).unwrap();

// ✅ After
let value = map.get(&key)
    .ok_or(Error::MissingField("key"))?;
```

### Pattern 2: Result::unwrap() → ? operator
```rust
// ❌ Before
let data = decode(&bytes).unwrap();

// ✅ After
let data = decode(&bytes)
    .map_err(Error::Decode)?;
```

### Pattern 3: Mutex unwrap() → expect()
```rust
// ❌ Before (in tests only)
let guard = mutex.lock().unwrap();

// ✅ After
let guard = mutex.lock()
    .map_err(|e| Error::LockPoisoned(e.to_string()))?;
```

### Pattern 4: panic!() → return Err()
```rust
// ❌ Before
if invalid_state {
    panic!("Invalid state");
}

// ✅ After
if invalid_state {
    return Err(Error::InvalidState("reason"));
}
```

---

## 📊 Daily Progress Tracking

### Day 1: Consensus Module
- [ ] Read and understand fork.rs logic
- [ ] Identify all 27 unwrap() calls
- [ ] Replace unwrap() in block validation
- [ ] Replace unwrap() in fork choice
- [ ] Add tests for error cases
- [ ] Run `cargo test -p bq-consensus`
- [ ] Commit progress

**Target:** 27 unwraps → 0

### Day 2: Mempool Module
- [ ] Audit mempool.rs unwrap() calls
- [ ] Replace unwrap() in insert/remove
- [ ] Replace unwrap() in fee calculation
- [ ] Add error tests
- [ ] Run `cargo test -p bq-mempool`
- [ ] Commit progress

**Target:** 21 unwraps → 0

### Day 3: Storage Module
- [ ] Audit rocksdb_store.rs
- [ ] Replace DB operation unwraps
- [ ] Replace serialization unwraps
- [ ] Add recovery tests
- [ ] Run `cargo test -p bq-storage`
- [ ] Commit progress

**Target:** 15 unwraps → 0

### Day 4-5: Wallet Module
- [ ] Audit multisig.rs (complex)
- [ ] Replace signature collection unwraps
- [ ] Replace transaction building unwraps
- [ ] Add multisig error tests
- [ ] Run `cargo test -p bq-wallet`
- [ ] Final commit

**Target:** 37 unwraps → 0

---

## ✅ Quality Checklist (Per Module)

Before committing each module:

### Code Quality
- [ ] All unwrap() replaced with proper error handling
- [ ] Error messages are descriptive
- [ ] No new unwrap() added
- [ ] Code follows existing patterns

### Testing
- [ ] All existing tests pass
- [ ] New tests for error cases added
- [ ] Edge cases covered
- [ ] No panics in production paths

### Documentation
- [ ] Code comments updated
- [ ] Error variants documented
- [ ] SAFETY comments where needed

### Build & CI
- [ ] `cargo build --all` passes
- [ ] `cargo test --all` passes
- [ ] `cargo clippy --all-targets` clean
- [ ] `cargo fmt --all` applied

---

## 📈 Success Metrics

### Phase 1B Goals
```
Start:  303 unwrap() calls
Target: 150 unwrap() calls
Goal:   153 calls eliminated (50% reduction)

Breakdown:
- Consensus: -27 (9%)
- Mempool:   -21 (7%)
- Storage:   -15 (5%)
- Wallet:    -37 (12%)
- Others:    -53 (17%)
TOTAL:       -153 (50%)
```

### Quality Targets
- ✅ Build: Clean compilation
- ✅ Tests: 100% passing
- ✅ Coverage: >85%
- ✅ CI: All checks pass

---

## 🚨 Common Pitfalls

### 1. Don't Just Replace Blindly
```rust
// ❌ Bad
let x = foo().ok_or(Error::Unknown)?;

// ✅ Good
let x = foo().ok_or(Error::FooFailed("specific reason"))?;
```

### 2. Handle All Error Paths
```rust
// ✅ Good
match operation() {
    Ok(value) => handle_success(value),
    Err(e) => {
        log::error!("Operation failed: {:?}", e);
        return Err(e);
    }
}
```

### 3. Add Tests for Errors
```rust
#[test]
fn test_missing_block_error() {
    let result = fork.get_block(&fake_hash);
    assert!(matches!(result, Err(Error::BlockNotFound(_))));
}
```

---

## 📞 Getting Help

### Resources
- **Error Handling Guide:** `docs/dev/PANIC_SAFETY.md`
- **Security Standards:** `docs/SECURITY_STANDARDS.md`
- **Refactoring Rules:** See prompt at top of this file

### Commands
```bash
# Find unwraps in a file
rg "\.unwrap\(\)" crates/consensus/src/fork.rs

# Count unwraps
rg "\.unwrap\(\)" crates/ --type rust | wc -l

# Test specific module
cargo test -p bq-consensus

# Check for new unwraps
git diff main | grep "unwrap()"
```

---

## 🎉 Phase 1B Completion

When all 4 modules are done:

### Final Steps
```bash
# Verify all changes
cargo test --all --release

# Check unwrap count
rg "\.unwrap\(\)" crates/ --type rust -c | awk -F: '{s+=$2} END {print s}'

# Should be ~150 or less

# Commit final changes
git add -A
git commit -m "fix(security): Phase 1B complete - consensus, mempool, storage, wallet hardened"

# Push to GitHub
git push origin security/p1b-consensus-hardening

# Create PR
gh pr create --title "Phase 1B: Eliminate 153 unwrap() calls" \
  --body "Reduces unwrap() from 303 → 150 (50% reduction)"
```

### Celebration Metrics
- ✅ 50% panic reduction achieved (303 → 150)
- ✅ 4 critical modules hardened
- ✅ Mainnet readiness: 75% → 85%
- ✅ Ready for Phase 2 (CI protection)

---

## 📅 Timeline

**Week 1:**
- Mon-Tue: Consensus + Mempool (48 unwraps)
- Wed-Thu: Storage + start Wallet (52 unwraps)
- Fri: Complete Wallet + testing (53 unwraps)

**Week 2:**
- Mon: Code review + fixes
- Tue: Final testing + documentation
- Wed: PR creation + merge
- Thu-Fri: Phase 2 planning

---

**START NOW:** `git checkout -b security/p1b-consensus-hardening` 🚀
