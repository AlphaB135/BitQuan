# IBD Implementation Pattern

**Date**: 2026-01-20
**Context**: Production Readiness Audit - Phase 2a
**Tags**: p2p, ibd, consensus, blockchain

## Pattern: Header-First Initial Block Download

### Problem
When a node starts or falls behind, it needs to sync with the network. Downloading full blocks immediately is inefficient because:
- Blocks are large (1MB+ each)
- Cannot validate chain continuity without downloading
- Wastes bandwidth on invalid chains

### Solution
**Header-first IBD** - Bitcoin's proven approach:

1. **Send GetHeaders** with block locator hashes (exponential backoff from tip)
2. **Receive Headers** message with up to 2000 headers
3. **Validate each header**:
   - Chain links (prev_block matches previous hash)
   - Proof of work (hash meets target difficulty)
4. **Queue block downloads** using GetData for validated headers
5. **Process blocks** as they arrive via Block message

### Code Reference

`crates/node/src/worker.rs:1036-1170` - `handle_headers()`

```rust
// Validate chain links
let prev_hash = header.prev_block;
let expected_prev = if idx == 0 {
    tip_hash
} else {
    header_hash(&headers[idx - 1])
};

if prev_hash != expected_prev {
    break; // Stop processing on invalid link
}

// Validate proof of work
let target = target_from_bits(header.bits)?;
if !meets_target(&hash, &target) {
    return Err(WorkerError::InvalidData("Invalid proof of work"));
}
```

### Key Insights

1. **Headers are lightweight** (~80 bytes vs 1MB+ blocks)
2. **Can validate entire chain** before downloading any block
3. **Prevents DoS** - reject invalid chains early
4. **Block locator** helps find common ancestor efficiently

### Why This Matters

- **Security**: Prevents accepting invalid blocks
- **Performance**: Reduces bandwidth by 99% during sync
- **Reliability**: Detects invalid chains before wasting resources

## Related Files

- `crates/node/src/worker.rs` - IBD implementation
- `crates/network/src/protocol.rs` - GetHeaders/Headers messages
- `crates/mempool/src/lib.rs` - Mempool P2P announcement
