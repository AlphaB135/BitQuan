# API Stability and Visibility Policy

**Status:** Draft (v0.0.2-alpha)
**Last Updated:** 2025-11-08

---

## 📋 Overview

This document defines BitQuan's API stability guarantees and visibility guidelines for crate interfaces.

---

## 🎯 Visibility Levels

### `pub` - Public Stable API

**Definition:** Functions, types, and modules marked `pub` are part of the **stable public API**.

**Guarantees:**
- ✅ **Semantic Versioning**: Breaking changes require major version bump
- ✅ **Deprecation Notice**: Minimum 1 minor version before removal
- ✅ **Migration Guide**: Provided for all breaking changes

**Examples:**
```rust
// Stable - guaranteed backward compatibility
pub fn validate_transaction(...) -> Result<()>
pub struct Block { ... }
pub use transaction::{Transaction, TxIn, TxOut};
```

### `pub(crate)` - Internal API

**Definition:** Functions and types marked `pub(crate)` are **internal implementation details**.

**Guarantees:**
- ❌ **No Stability Guarantee**: Can change at any time
- ❌ **No Deprecation Notice**: May be removed without warning
- ✅ **Crate-Only**: Not visible to external users

**Examples:**
```rust
// Internal - subject to change
pub(crate) fn calculate_block_weight_with_beta(...) -> u64
pub(crate) use pow::{DEVNET_MAX_BITS, DEVNET_MIN_BITS};
```

### Private (`mod` without `pub`)

**Definition:** Modules and items with no visibility modifier.

**Scope:** Only visible within the same file/module.

---

## 📊 Current API Surface

### Stable Public APIs (`pub`)

#### `bitquan-types`
- ✅ `Transaction`, `Block`, `BlockHeader`
- ✅ `TxIn`, `TxOut`, `Witness`
- ✅ `NetworkId`, `TxContext`
- ✅ `Error`, `Result`
- ✅ `WireEncode`, `WireDecode`
- ✅ `validate_transaction()`, `validate_block_structure()`

#### `bitquan-crypto`
- ✅ Wallet encryption/decryption
- ✅ Keystore operations
- ✅ BIP39 mnemonic support

#### `bitquan-consensus`
- ✅ `validate_block()`, `validate_transaction_signatures()`
- ✅ `calculate_tx_weight()`, `calculate_block_weight()`
- ✅ `check_header_pow()`, `header_hash()`
- ✅ `UtxoSet`, `UtxoEntry`, `OutPoint`
- ✅ `ConsensusParams`, `NetworkParams`
- ✅ `DifficultyParams`, `RewardSchedule`

#### `bitquan-mempool`
- ✅ `Mempool::add_transaction()`
- ✅ `Mempool::get_transactions_for_block()`

#### `bitquan-storage`
- ✅ `RocksDBStore::new()`
- ✅ `BlockStore` trait

#### `bitquan-network`
- ✅ Peer management APIs
- ✅ P2P protocol structures

#### `bitquan-rpc`
- ✅ JSON-RPC 2.0 methods
- ✅ JWT authentication

### Internal APIs (`pub(crate)` or deprecated)

#### `bitquan-consensus`
- ⚠️ `calculate_block_weight_with_beta()` - **Deprecated + pub(crate)**
  - **Reason:** Testing/evolution function, not production API
  - **Alternative:** Use `calculate_block_weight()` with `ConsensusParams`

---

## 🔧 Deprecation Process

### Step 1: Mark as Deprecated
```rust
#[deprecated(since = "0.1.0", note = "Use new_function() instead")]
pub fn old_function() { ... }
```

### Step 2: Update Documentation
- Add deprecation notice to CHANGELOG.md
- Update migration guide
- Add "Deprecated" badge to docs

### Step 3: Wait One Minor Version
- Example: Deprecated in v0.1.0, remove in v0.2.0 (minimum)
- For major APIs, wait two minor versions

### Step 4: Remove
- Remove in next minor/major version
- Document removal in CHANGELOG

---

## 📝 Guidelines for Contributors

### Adding New Public APIs

**Before making something `pub`, ask:**

1. ✅ **Is this essential for external users?**
   - If no → Use `pub(crate)`

2. ✅ **Can we commit to long-term stability?**
   - If no → Use `pub(crate)` or mark `#[doc(hidden)]`

3. ✅ **Is the API well-designed and unlikely to change?**
   - If no → Keep private until design stabilizes

### Marking Items `pub(crate)`

**Use `pub(crate)` when:**
- ✅ Function is only used by other crates in the workspace
- ✅ Implementation detail that may change frequently
- ✅ Testing/debugging utility not meant for external use

**Example:**
```rust
// Good - internal helper
pub(crate) fn internal_validation_helper(...) -> Result<()> {
    // Implementation
}

// Good - only used by consensus tests
#[cfg(test)]
pub(crate) fn calculate_weight_custom_params(...) -> u64 {
    // Test helper
}
```

### Cross-Crate Dependencies

**Problem:** `pub(crate)` only works within the same crate.

**Solution:** For workspace-internal APIs:
```rust
// In types/src/lib.rs
/// Returns signature count (workspace-internal use)
///
/// **Note:** This is an internal helper. External users should
/// use consensus APIs directly.
#[doc(hidden)]
pub fn count_signatures(block: &Block) -> u64 {
    // Used by bitquan-consensus internally
}
```

**Alternative:** Use `#[doc(hidden)]` to hide from docs but keep `pub`.

---

## 🎯 Version 1.0 Goals

Before declaring v1.0.0 stable:

1. ✅ Audit all `pub` items across crates
2. ✅ Move unnecessary `pub` items to `pub(crate)`
3. ✅ Add `#[doc(hidden)]` to workspace-internal APIs
4. ✅ Document all stable APIs with examples
5. ✅ External security audit of public APIs
6. ✅ Establish SemVer compliance policy

---

## 📚 References

- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [Semantic Versioning 2.0](https://semver.org/)
- [Cargo SemVer Compatibility](https://doc.rust-lang.org/cargo/reference/semver.html)

---

**Status:** This policy is a living document and will evolve as BitQuan matures.
