# P1 Unwrap/Expect/Panic Inventory - Node/Network/Mempool

**Date:** 2025-11-06  
**Branch:** fix/p1-network-hardening  
**Baseline:** P0 complete (consensus/crypto clean)

## Scope

Non-consensus-critical runtime modules: node operations, networking, mempool, and RPC.

## Production Unwrap Count (excluding tests)

### Node Layer (42 total)
- `crates/node/src/pool_db.rs`: 12 unwraps
- `crates/node/src/main.rs`: 8 unwraps
- `crates/node/src/stratum_server.rs`: 5 unwraps
- `crates/node/src/address.rs`: 4 unwraps
- `crates/node/src/chainstate.rs`: 3 unwraps
- `crates/node/src/metrics.rs`: 3 unwraps
- `crates/node/src/ws_dashboard.rs`: 3 unwraps
- `crates/node/src/wallet.rs`: 2 unwraps
- `crates/node/src/miner.rs`: 1 unwrap
- `crates/node/src/reward_engine.rs`: 1 unwrap

### Network Layer (36 total)
- `crates/network/src/peer.rs`: 13 unwraps
- `crates/network/src/propagation.rs`: 10 unwraps
- `crates/network/src/relay.rs`: 8 unwraps
- `crates/network/src/discovery.rs`: 5 unwraps

### Mempool Layer (29 total)
- `crates/mempool/src/lib.rs`: 29 unwraps

### RPC Layer (9 total)
- `crates/rpc/src/server.rs`: 9 unwraps

**Total P1 Production Unwraps: 116**

## Remediation Strategy

### High Priority (Runtime Stability)
1. **Mempool (29)** - Transaction validation and eviction
2. **Node main (8)** - Startup and shutdown paths
3. **Pool DB (12)** - Database operations

### Medium Priority (Network Resilience)
1. **Peer management (13)** - Connection handling
2. **Propagation (10)** - Block/tx relay
3. **RPC server (9)** - Request handling
4. **Relay (8)** - P2P relay logic

### Lower Priority (Auxiliary)
1. **Discovery (5)** - Peer discovery
2. **Stratum (5)** - Mining pool
3. **Metrics/Dashboard (6)** - Monitoring
4. **Misc (11)** - Address, wallet, chainstate

## Target

**Goal:** ≤10 production unwraps remaining (annotated with SAFETY)  
**Actual Reduction Expected:** 106 unwraps (116 → ~10)

## Approach

- Replace unwrap with pattern matching + logging
- Add retry logic with exponential backoff for I/O
- Use `?` operator for error propagation in async handlers
- Add metrics for failures and retries
- Comprehensive integration tests

