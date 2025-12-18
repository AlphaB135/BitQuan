# 🔥 CRITICAL: main.rs Async Migration Prompt

**TO:** Another AI Assistant
**FROM:** Senior Rust Async Architect
**CONTEXT:** BitQuan blockchain - Phase 2 Part 2 of async network migration
**BRANCH:** `feature/async-network-migration`
**STATUS:** Infrastructure ready, needs main.rs integration

---

## 📋 TASK: Update main.rs for Async Network

### Background

We have successfully created:
1. ✅ `crates/network/src/peer_async.rs` - Async peer with Slowloris protection
2. ✅ `crates/network/src/server_async.rs` - Async P2P server with tokio

Now we need to integrate these into `crates/node/src/main.rs`.

### Critical Information

**File:** `crates/node/src/main.rs` (~2800 lines)
**Main function:** Already has `#[tokio::main] async fn main()` ✅
**Key functions to update:**
- `run_node()` (line ~1018)
- `start_p2p_server()` (line ~1040)
- `mine_continuous()` calls (line ~767)

---

## 🎯 YOUR MISSION

### 1. Update `run_node()` function

**Location:** Line ~1018

**CURRENT CODE:**
```rust
fn run_node(
    config_path: &str,
    rpc_bind: Option<&str>,
    p2p_bind: Option<&str>,
    network: NetworkId,
) -> Result<()> {
    // ...
    start_p2p_server(p2p_addr, network)
}
```

**CHANGE TO:**
```rust
async fn run_node(
    config_path: &str,
    rpc_bind: Option<&str>,
    p2p_bind: Option<&str>,
    network: NetworkId,
) -> Result<()> {
    // ...
    start_p2p_server_async(p2p_addr, network).await
}
```

**Key changes:**
- Add `async` keyword
- Change `start_p2p_server()` → `start_p2p_server_async().await`

---

### 2. Replace `start_p2p_server()` with async version

**Location:** Line ~1040

**CURRENT CODE (SYNC - DELETE THIS):**
```rust
fn start_p2p_server(addr: &str, network: NetworkId) -> Result<()> {
    let listener = TcpListener::bind(addr)?;
    listener.set_nonblocking(false)?;
    println!("P2P server listening at {addr}");
    loop {
        let (stream, peer) = listener.accept()?;
        println!("Incoming connection from {peer}");
        thread::spawn(move || {
            if let Err(e) = handle_peer(stream, network) {
                eprintln!("peer error: {e}");
            }
        });
    }
}
```

**REPLACE WITH (ASYNC - NEW CODE):**
```rust
async fn start_p2p_server_async(addr: &str, network: NetworkId) -> Result<()> {
    use bitquan_network::server_async::spawn_p2p_server_with_limit;
    use bitquan_network::peer_async::AsyncPeerManager;
    use std::sync::Arc;

    // Create async peer manager
    let peer_manager = Arc::new(AsyncPeerManager::new(
        100, // max peers
        network
    ));

    // Spawn P2P server in background
    spawn_p2p_server_with_limit(
        addr,
        peer_manager.clone(),
        100 // max connections
    ).await?;

    log::info!("Async P2P server running on {}", addr);

    // Keep running (server is in background task)
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;

        // Cleanup dead peers every minute
        peer_manager.cleanup_peers().await;

        let peer_count = peer_manager.ready_peer_count().await;
        log::info!("Active peers: {}", peer_count);
    }
}
```

**Important notes:**
- This replaces the old sync TcpListener with async version
- Uses `spawn_p2p_server_with_limit()` from our new `server_async.rs`
- Server runs in background (tokio::spawn)
- Main loop does periodic cleanup

---

### 3. Update `mine_continuous()` call

**Location:** Line ~767 (in Commands::Mine handler)

**CURRENT CODE:**
```rust
mine_continuous(MiningOptions {
    datadir: &datadir,
    payout_script_hex: &payout_script_hex,
    bits_override: bits,
    max_nonce,
    threads,
    limit_blocks,
    network: network_id,
    pow_mode,
    hybrid_weights: weights,
    peers,
})
```

**CRITICAL ISSUE:** Mining is CPU-intensive! Running it directly in async runtime will BLOCK the entire network!

**CHANGE TO:**
```rust
// CRITICAL: Mining is CPU-intensive, must use spawn_blocking!
let datadir_owned = datadir.clone();
let payout_script_owned = payout_script_hex.clone();

let mining_handle = tokio::task::spawn_blocking(move || {
    mine_continuous(MiningOptions {
        datadir: &datadir_owned,
        payout_script_hex: &payout_script_owned,
        bits_override: bits,
        max_nonce,
        threads,
        limit_blocks,
        network: network_id,
        pow_mode,
        hybrid_weights: weights,
        peers,
    })
});

// Wait for mining to complete
mining_handle.await.map_err(|e| Error::Invalid(e.to_string()))??
```

**Key changes:**
- Clone borrowed data (datadir, payout_script_hex)
- Wrap in `tokio::task::spawn_blocking()`
- This runs mining in dedicated thread pool
- Doesn't block the async runtime
- Network layer continues working while mining!

---

### 4. Update main() to call async run_node

**Location:** Line ~720 (Commands::Run handler)

**CURRENT CODE:**
```rust
Commands::Run { config, rpc_bind, p2p_bind } => {
    let network = load_network_from_config(&config)?;
    run_node(&config, rpc_bind.as_deref(), p2p_bind.as_deref(), network)
}
```

**CHANGE TO:**
```rust
Commands::Run { config, rpc_bind, p2p_bind } => {
    let network = load_network_from_config(&config)?;
    run_node(&config, rpc_bind.as_deref(), p2p_bind.as_deref(), network).await
}
```

**Key change:** Add `.await`

---

## 🔧 Additional Updates Needed

### Update imports at top of main.rs

**ADD these imports:**
```rust
use std::sync::Arc;
use tokio::task;
```

**OPTIONAL:** If handle_peer() is still used elsewhere, you may need to update it or remove it.

---

## ⚠️ CRITICAL WARNINGS

### 1. DO NOT run mining directly in async context
❌ **WRONG:**
```rust
async fn main() {
    mine_continuous(...); // BLOCKS ENTIRE RUNTIME!
}
```

✅ **CORRECT:**
```rust
async fn main() {
    tokio::task::spawn_blocking(|| {
        mine_continuous(...); // Runs in thread pool
    }).await??;
}
```

### 2. Clone borrowed data before moving into spawn_blocking
❌ **WRONG:**
```rust
spawn_blocking(move || {
    mine_continuous(MiningOptions {
        datadir: &datadir, // ERROR: borrowed data
    })
})
```

✅ **CORRECT:**
```rust
let datadir_owned = datadir.clone();
spawn_blocking(move || {
    mine_continuous(MiningOptions {
        datadir: &datadir_owned, // OK: owned data
    })
})
```

### 3. Don't forget .await on async functions
❌ **WRONG:**
```rust
run_node(...); // Returns Future, doesn't execute!
```

✅ **CORRECT:**
```rust
run_node(...).await?; // Executes and handles errors
```

---

## 🧪 TESTING CHECKLIST

After making changes, verify:

### 1. Compilation
```bash
cargo check -p bitquan-node
```

Expected: ✅ No errors

### 2. Run node
```bash
cargo run --release --bin bitquan-node -- run
```

Expected output:
```
Starting BitQuan node...
Async P2P server running on 0.0.0.0:18444
Active peers: 0
```

### 3. Test mining
```bash
cargo run --release --bin bitquan-node -- mine ...
```

Expected: Mining runs WITHOUT blocking (you can still accept peers)

### 4. Check logs
Expected: No "blocking runtime" warnings from tokio

---

## 📊 EXPECTED RESULTS

### Before (Sync):
```
Thread 1: Accept peer connections
Thread 2: Handle peer 1
Thread 3: Handle peer 2
...
Thread 1001: Mining (blocks all network if run in main thread)
```

**Problem:** 1000 threads × 8MB = 8GB RAM!

### After (Async):
```
Tokio Runtime:
├─ Task 1: P2P server (accept loop)
├─ Task 2: Peer handler 1 (4KB)
├─ Task 3: Peer handler 2 (4KB)
├─ ...
└─ Task 1000: Peer handler 999 (4KB)

Thread Pool:
└─ Mining (doesn't block async runtime)
```

**Benefits:** 1000 tasks × 4KB = 4MB RAM! (2000x better)

---

## 🎯 SUCCESS CRITERIA

Your changes are correct if:

1. ✅ `cargo check -p bitquan-node` passes
2. ✅ `cargo run --bin bitquan-node -- run` starts successfully
3. ✅ P2P server accepts connections
4. ✅ Mining runs without blocking network
5. ✅ No "blocking the runtime" warnings
6. ✅ Memory usage is reasonable (not 8GB for 1000 peers)

---

## 📝 WHAT TO PROVIDE BACK

Please provide:

1. **Complete updated functions:**
   - `run_node()`
   - `start_p2p_server_async()` (new)
   - Updated `Commands::Run` handler
   - Updated `Commands::Mine` handler

2. **Import changes** at top of file

3. **Compilation output:**
   ```bash
   cargo check -p bitquan-node 2>&1 | tail -20
   ```

4. **Any errors encountered** and how you fixed them

---

## 🆘 IF YOU GET STUCK

### Common Issues:

**Issue:** "borrowed data cannot be moved"
**Fix:** Clone the data before moving into spawn_blocking

**Issue:** "cannot find function `spawn_p2p_server_with_limit`"
**Fix:** Add import: `use bitquan_network::server_async::spawn_p2p_server_with_limit;`

**Issue:** "await is only allowed inside async functions"
**Fix:** Make sure the function is marked `async fn`

**Issue:** Mining blocks network
**Fix:** Wrap in `tokio::task::spawn_blocking()`

---

## 📚 REFERENCE

Files you can reference:
- `crates/network/src/peer_async.rs` - Async peer implementation
- `crates/network/src/server_async.rs` - Async P2P server
- `PHASE2_INTEGRATION_GUIDE.md` - Integration guide
- `ASYNC_MIGRATION_PLAN.md` - Full migration plan

---

## ⚡ PRIORITY ORDER

1. **HIGHEST:** Update run_node() and start_p2p_server_async()
2. **HIGH:** Update Commands::Mine with spawn_blocking
3. **MEDIUM:** Update Commands::Run handler
4. **LOW:** Cleanup old handle_peer() if not used

---

**Good luck! This is the final step to eliminate Slowloris vulnerability! 🚀**

Remember: The goal is to make mining use `spawn_blocking` and P2P use async I/O.
