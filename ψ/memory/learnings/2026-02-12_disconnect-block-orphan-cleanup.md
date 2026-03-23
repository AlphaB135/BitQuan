# Lesson Learned: disconnect_block Orphan Data Cleanup

**Date**: 2026-02-12
**Category**: Storage / Reorg Handling
**Severity**: Critical Data Corruption

## The Problem

When implementing `disconnect_block()` for chain reorganizations, the original code only handled UTXO restoration and tip updates. It failed to clean up orphan data from column families:

- `CF_BLOCKS` - Block data
- `CF_HEADERS` - Header data
- `CF_HEIGHT_INDEX` - Height to block hash mapping
- `CF_TX_INDEX` - Transaction index
- `CF_UNDO` - Undo data

## Why This Matters

During a chain reorg:
1. Blocks are disconnected via `disconnect_block()`
2. New blocks from the winning chain are connected
3. **If orphan data remains**, lookups may return stale data that conflicts with the new chain
4. This causes undefined behavior, data corruption, or crashes

## The Fix

Always clean up ALL column family data when disconnecting a block:

```rust
// After UTXO restoration and tip update...
let block_id = Self::block_id(&block.header);
let current_height = self.height()?;

// Delete orphan data
batch.delete_cf(&cf_blocks, block_id);
batch.delete_cf(&cf_headers, block_id);
batch.delete_cf(&cf_height, (current_height - 1).to_le_bytes());
for tx in &block.transactions {
    batch.delete_cf(&cf_tx, tx.txid());
}
batch.delete_cf(&cf_undo, block_id);
```

## Key Takeaways

1. **Column Family Inventory**: Maintain a list of all CFs that store block-related data
2. **Symmetric Operations**: For every `put_cf` in `connect_block()`, there must be a corresponding `delete_cf` in `disconnect_block()`
3. **Test Reorgs**: Integration tests should verify full reorg scenarios, not just single operations
4. **Audit Trail**: Document all CF usages to make cleanup code reviewable

## Related Issues

- C2: disconnect_block orphan cleanup (FIXED)
- C3: sync.rs claimed_height fallback (FIXED)
- T5: duplicate loop in handle_getblocks (FIXED)

## References

- Commit: 1730458
- File: crates/storage/src/rocksdb_store.rs:1131-1241
