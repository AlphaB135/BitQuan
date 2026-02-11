# Infinite Loop Prevention: TODO Stub Anti-Pattern

**Context**: P2P sync reliability
**Date**: 2026-02-12
**Issue**: #122 C6

## Problem

`request_blocks_from_peer()` was a TODO stub that always returned `Ok(vec![])`:

```rust
// WRONG - always succeeds, never fails
pub fn request_blocks_from_peer(...) -> Result<Vec<BlockHeader>> {
    // TODO: Implement actual network communication
    
    // Return empty vector for now - in production this would contain actual headers
    Ok(vec![])  // Always succeeds!
}
```

The calling code checked for empty and continued:
```rust
match request_blocks_from_peer(...)? {
    Ok(headers) => {
        if headers.is_empty() {
            // SECURITY FIX: Don't break - try next peer instead
            continue;  // Infinite loop!
        }
    }
    Err(_) => break;  // Never hit
}
```

Result: **Infinite retry loop** when peer can't provide blocks.

## Solution

Return explicit error instead of empty Vec:

```rust
// CORRECT - fail fast
pub fn request_blocks_from_peer(...) -> Result<Vec<BlockHeader>> {
    // Not implemented - peer unavailable
    Err(bitquan_types::Error::Net(
        "request_blocks_from_peer not implemented - peer unavailable".to_string()
    ))
}
```

Now the calling code properly handles the error:
```rust
match request_blocks_from_peer(...)? {
    Ok(headers) => { /* normal processing */ }
    Err(e) => {
        log::error!("Peer {} unavailable: {}", peer_id, e);
        break;  // Try next peer
    }
}
```

## Key Insights

1. **TODO stubs are dangerous**: They silently succeed, hiding failures.

2. **Fail fast**: Return explicit error so caller can handle appropriately.

3. **Use error variants**: Don't return Ok(vec![]) when operation failed.

4. **Log the failure**: Error message should explain WHAT failed, not just "not implemented".

## Reliability Impact

- **Prevents infinite loops**: Error breaks retry loop immediately
- **Enables fallback**: Caller can try different peer
- **Better debugging**: Error messages show actual failure points

## Related

- Files: `crates/network/src/sync.rs`
- Function: `request_blocks_from_peer()`
- Issue: #122 (BitQuan Master Fix Plan)
