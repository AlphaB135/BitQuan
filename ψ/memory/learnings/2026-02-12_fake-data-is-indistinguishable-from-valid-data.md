# Fake Data is Indistinguishable from Valid Data

**Context**: P2P IBD headers validation
**Date**: 2026-02-12
**Issue**: #122 C3

## Problem

`find_headers_after_cached()` returned fake `BlockHeader` objects with zeroed fields when cache-only mode was used:

```rust
// WRONG - returns fake headers
result.push(BlockHeader {
    version: 0,
    prev_block: history[i],  // Only hash is real
    merkle_root: [0u8; 32],  // Fake!
    pqc_agg_hint: [0u8; 32], // Fake!
    time: 0,                       // Fake!
    bits: 0,                      // Fake!
    nonce: 0,                     // Fake!
    algo_id: 0,                  // Fake!
});
```

This makes it **impossible to distinguish**:
- Valid peer with real headers
- Malicious peer returning fake headers
- Both produce the same "empty-ish" result

## Solution

Add `validated_headers: HashMap<hash, (header, timestamp)>` cache:

```rust
// CORRECT - validation cache
pub struct ChainState {
    validated_headers: Arc<Mutex<HashMap<[u8; 32], (BlockHeader, Instant)>>>,
}

// Cache only returns validated headers
fn find_headers_after_cached(&self, locators, limit) -> Vec<BlockHeader> {
    let validated = self.validated_headers.lock().unwrap();
    let now = Instant::now();
    
    for i in start_index..(start_index + limit) {
        let block_hash = history[i];
        
        // Only return if validated and not stale
        if let Some((header, validated_at)) = validated.get(&block_hash) {
            if now.duration_since(*validated_at) < MAX_HEADER_AGE {
                result.push(header.clone());
            }
            // Skip unvalidated headers - do NOT return fake data
        }
    }
    
    result
}
```

## Key Insights

1. **Validation is separate from caching**: Maintaining a separate validated_headers map ensures only verified data is returned.

2. **Timestamp-based expiry**: `MAX_HEADER_AGE (2 hours)` prevents stale data while allowing recent headers.

3. **Enforce size limits**: `MAX_VALIDATED_HEADERS (5000)` prevents unbounded memory growth.

4. **No fake data fallback**: Better to return empty Vec than fake headers that look valid.

## Security Impact

- **Prevents partition attacks**: Malicious peers can't poison header cache
- **Enables peer scoring**: Can distinguish reliable vs unreliable sources
- **Clean failure modes**: NoValidHeaders error allows proper error propagation

## Related

- Files: `crates/node/src/chainstate.rs`
- Functions: `find_headers_after_cached()`, `cache_validated_header()`
- Issue: #122 (BitQuan Master Fix Plan)
