# Async Network Integration Guide

## Changes Required for Phase 2

### 1. Update node/Cargo.toml

Add tokio dependency to node crate:

```toml
[dependencies]
tokio = { version = "1", features = ["full"] }
```

### 2. Update mine_continuous call in main.rs

**BEFORE (Line ~767):**
```rust
mine_continuous(MiningOptions {
    datadir: &datadir,
    // ... options
})
```

**AFTER:**
```rust
// Spawn mining in blocking thread pool (CPU-intensive)
let mining_handle = tokio::task::spawn_blocking(move || {
    mine_continuous(MiningOptions {
        datadir: &datadir,
        // ... options (all must be owned/cloned)
    })
});

// Wait for mining to complete
mining_handle.await??
```

### 3. Update P2P server initialization

**Find:** `P2PListener::bind` or sync TcpListener for P2P

**Replace with:**
```rust
use bitquan_network::server_async::spawn_p2p_server_with_limit;
use bitquan_network::peer_async::AsyncPeerManager;

// Create async peer manager
let peer_manager = Arc::new(AsyncPeerManager::new(
    100, // max peers
    network_id
));

// Spawn P2P server (non-blocking)
spawn_p2p_server_with_limit(
    &p2p_bind,
    peer_manager.clone(),
    100 // max connections
).await?;

log::info!("P2P server running in background");
```

### 4. Key Points

1. **Mining is CPU-bound** → Use `spawn_blocking`
2. **P2P is I/O-bound** → Use async (tokio::spawn)
3. **Main is already async** → Good! Just need to call `.await`

### 5. Architecture

```
main() [tokio::main async]
├─ RPC Server [tokio::spawn async task]
├─ P2P Server [tokio::spawn async task]
│  └─ Per-peer handlers [tokio::spawn each]
└─ Mining [tokio::task::spawn_blocking]
   └─ CPU-intensive loop (doesn't block async runtime)
```

### 6. Testing

```bash
# Compile check
cargo check -p bitquan-node

# Run node
cargo run --release --bin bitquan-node -- run

# Should see:
# - P2P server listening on ...
# - Mining started in background
# - No "blocking runtime" warnings
```

## Implementation Status

- [x] Phase 1: peer_async.rs created
- [x] Phase 1: server_async.rs created
- [ ] Phase 2: Update mine_continuous to spawn_blocking
- [ ] Phase 2: Update P2P to use AsyncP2PListener
- [ ] Phase 2: Test integration

## Notes

The main.rs file is complex with many commands. We need to:
1. Find all places that call `mine_continuous`
2. Wrap with `spawn_blocking`
3. Find all places that use sync P2P
4. Replace with async P2P

This requires careful analysis of the full file.
