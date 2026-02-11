# P2P Efficiency Pattern: Common Ancestor Discovery

**Context**: IBD bandwidth optimization
**Date**: 2026-02-12
**Issue**: #122 C5

## Problem

Original `handle_getblocks()` announced blocks from height 0, wasting bandwidth:

```rust
// WRONG - announces from genesis
let mut height = 0u64;
while inv.len() < limit {
    match ctx.storage.get_block_by_height(height).await {
        Ok(Some(block)) => {
            inv.push(block_hash);
        }
        // ...
    }
    height += 1;
}
```

Peers requesting blocks after locator at height 1000 would receive 1000+ blocks they already have.

## Solution

Find common ancestor height first, then start announcing AFTER that point:

```rust
// CORRECT - start from common ancestor
let mut start_height = 0u64;

// Find common ancestor height
for locator_hash in &locator_hashes {
    if let Some(ancestor_hash) = locator_hashes.iter().find(|hash| {
        ctx.storage.get_block(hash).await.ok().flatten().is_some()
    }) {
        // Search for ancestor's height
        for h in 0..=chain_height {
            if let Ok(Some(block)) = ctx.storage.get_block_by_height(h).await {
                let block_hash = header_hash(&block.header);
                if block_hash == *locator_hash {
                    start_height = h + 1;  // Start AFTER ancestor
                    break;
                }
            }
        }
        break;
    }
}

let mut height = start_height;  // Use computed start
```

## Key Insights

1. **Locator search is ordered**: Check locators from newest to oldest, first match wins.

2. **Height calculation**: Start from `ancestor_height + 1`, not from 0.

3. **Only announce what peer needs**: If peer has block 1000 and we're at 1050, only send 50 blocks (1050-1100).

## Performance Impact

- **Bandwidth savings**: For sync at height 1000k with 1000-block deep peer, saves ~999 blocks worth of data
- **Reduced latency**: Smaller response messages process faster
- **Better UX**: Peer sees progress from their actual tip, not from genesis

## Related

- Files: `crates/node/src/worker.rs`
- Function: `handle_getblocks()`
- Issue: #122 (BitQuan Master Fix Plan)
