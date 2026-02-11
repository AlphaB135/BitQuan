# Hash Verification Pattern

**Context**: BitQuan storage layer data integrity
**Date**: 2026-02-12
**Issue**: #122 C1

## Problem

The original C1 fix computed `block_hash(&block.header)` twice and compared identical values:

```rust
// WRONG - compares same hash twice
let expected_hash = block_hash(&block.header);
let actual_hash = block_hash(&block.header);  // Same computation!
if expected_hash != actual_hash {  // Never triggers
    return Err(...);
}
```

This makes hash verification completely broken - silent data corruption passes through.

## Solution

Compare the **stored** block_id (from CF_HEIGHT_INDEX column family) against the **recomputed** hash from the block header:

```rust
// CORRECT - stored vs recomputed
let block_id_bytes = self.db.get_cf(&cf_height, height.to_le_bytes())?;
let mut stored_hash = [0u8; 32];
stored_hash.copy_from_slice(&block_id_bytes);

let recomputed_hash = pow::header_hash(&block.header);

if stored_hash != recomputed_hash {
    return Err(StorageError::DatabaseError(format!(
        "Hash mismatch at height {}: stored={}, recomputed={}",
        check_height, 
        hex::encode(stored_hash),
        hex::encode(recomputed_hash)
    )));
}
```

## Key Insights

1. **Stored value is source of truth**: The hash stored when block was inserted is the authoritative value. Recomputing from the same data will always match.

2. **Column family access**: CF_HEIGHT_INDEX maps `height → block_id (hash)`. This is the stored value we need to verify against.

3. **Fail fast on mismatch**: Don't increment corrupted_blocks counter and continue - return Err immediately. Data corruption is a critical failure, not something to count.

4. **Hex encoding for debugging**: Include both hashes in error message for forensic analysis.

## Detection Targets

- **Database corruption**: Storage media failure, bit rot
- **Memory corruption**: In-flight modification before write
- **Software bugs**: Incorrect serialization logic

## Related

- File: `crates/storage/src/rocksdb_store.rs`
- Function: `verify_block_integrity()`
- Issue: #122 (BitQuan Master Fix Plan)
