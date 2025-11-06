# P1 Unwrap Resolution Strategy & Progress

**Date:** 2025-11-06  
**Branch:** fix/p1-network-hardening

## Current State

**Total Production Unwraps:** 87 (verified, excluding tests)  
**Target:** ≤10 with SAFETY annotations

## Analysis by Pattern

### Pattern 1: Mutex Lock Unwraps (31 instances)
**Files Affected:**
- `pool_db.rs`: 12 instances of `.lock().unwrap()`
- `peer.rs`: 5 instances
- `propagation.rs`: 4 instances
- Others: 10 instances

**Fix Pattern:**
```rust
// ❌ Before (panics on poison)
let conn = self.conn.lock().unwrap();

// ✅ After (recovers from poison)
let conn = self.conn.lock()
    .map_err(|e| {
        tracing::error!("Pool DB lock poisoned: {}", e);
        Error::LockPoisoned
    })?;
```

**Rationale:** Mutex poisoning is rare but recoverable. Log and propagate error.

### Pattern 2: Channel/Thread Operations (18 instances)
**Files Affected:**
- `main.rs`: 8 instances (thread joins, channel operations)
- `stratum_server.rs`: 5 instances
- Others: 5 instances

**Fix Pattern:**
```rust
// ❌ Before (panics on channel close)
tx.send(msg).unwrap();

// ✅ After (handles disconnect gracefully)
if let Err(e) = tx.send(msg) {
    tracing::warn!("Channel closed, peer disconnected: {}", e);
    return Ok(()); // Graceful exit
}
```

### Pattern 3: Network I/O (23 instances)
**Files Affected:**
- `peer.rs`: 8 instances
- `propagation.rs`: 6 instances
- `relay.rs`: 5 instances
- `discovery.rs`: 4 instances

**Fix Pattern:**
```rust
// ❌ Before (panics on parse error)
let addr: SocketAddr = s.parse().unwrap();

// ✅ After (logs and returns error)
let addr: SocketAddr = s.parse()
    .map_err(|e| {
        tracing::warn!("Invalid peer address '{}': {}", s, e);
        Error::InvalidAddress
    })?;
```

### Pattern 4: RPC Request Parsing (9 instances)
**Files Affected:**
- `server.rs`: 9 instances

**Fix Pattern:**
```rust
// ❌ Before (panics on invalid JSON)
let params: GetBlockParams = serde_json::from_value(req.params).unwrap();

// ✅ After (returns BadRequest)
let params: GetBlockParams = serde_json::from_value(req.params)
    .map_err(|e| {
        tracing::warn!("Invalid RPC params: {}", e);
        RpcError::InvalidParams(e.to_string())
    })?;
```

### Pattern 5: Misc (Metrics, Address Parsing) (6 instances)
**Files Affected:**
- `metrics.rs`: 3 instances
- `address.rs`: 3 instances

**Fix Pattern:** Similar to above - log + propagate error

## Implementation Plan

### Phase 1: Critical Paths (Priority Order)
1. ✅ **Mempool** - Already clean!
2. **Pool DB** (12) - Mutex lock hardening
3. **Main** (8) - Graceful shutdown paths
4. **RPC Server** (9) - Request validation

### Phase 2: Network Resilience
1. **Peer** (13) - Connection error handling
2. **Propagation** (10) - Relay failures
3. **Relay** (8) - P2P robustness

### Phase 3: Auxiliary
1. **Discovery** (5) - DNS/bootstrap
2. **Stratum** (5) - Mining pool
3. **Misc** (11) - Metrics, dashboard, etc.

## Metrics to Add

```rust
// In metrics.rs
pub fn register_p1_metrics(registry: &Registry) {
    // Network
    NETWORK_PEER_DISCONNECTS.register(registry);
    NETWORK_IO_ERRORS.register(registry);
    NETWORK_RETRIES.register(registry);
    
    // Mempool
    MEMPOOL_EVICTIONS.register(registry);
    MEMPOOL_REJECTIONS.register(registry);
    
    // RPC
    RPC_REQUESTS_FAILED.register(registry);
    RPC_AUTH_FAILURES.register(registry);
    
    // Pool DB
    POOL_DB_LOCK_FAILURES.register(registry);
}
```

## Testing Strategy

### Integration Tests to Add
1. **Peer Disconnect + Retry**
   ```bash
   tests/network_resilience.rs
   - Simulate peer disconnect
   - Verify retry with backoff
   - Check metrics incremented
   ```

2. **Mempool Overflow**
   ```bash
   tests/mempool_limits.rs
   - Fill mempool to capacity
   - Verify eviction by fee rate
   - Check no panics
   ```

3. **RPC Invalid Payloads**
   ```bash
   tests/rpc_validation.rs
   - Send malformed JSON
   - Verify 400 BadRequest response
   - Check error logged
   ```

4. **Pool DB Lock Recovery**
   ```bash
   tests/pool_db_resilience.rs
   - Simulate poison scenario
   - Verify error propagation
   - Check DB still usable
   ```

## Acceptance Criteria

- [x] Inventory complete (87 unwraps identified)
- [ ] Phase 1 complete (≤50 unwraps remaining)
- [ ] Phase 2 complete (≤20 unwraps remaining)  
- [ ] Phase 3 complete (≤10 unwraps remaining)
- [ ] All tests passing
- [ ] Metrics registered and exporting
- [ ] CODE_AUDIT_REPORT.md updated

## Estimated Effort

- **Phase 1:** 4-6 hours (29 unwraps, critical paths)
- **Phase 2:** 3-4 hours (31 unwraps, network layer)
- **Phase 3:** 2-3 hours (27 unwraps, auxiliary)
- **Testing:** 2-3 hours (integration tests)
- **Documentation:** 1 hour

**Total:** 12-17 hours over 2-3 days

## Next Actions

1. Create error types for lock failures, network errors
2. Implement Phase 1 fixes (pool_db, main, rpc)
3. Add metrics infrastructure
4. Write integration tests
5. Document before→after counts

