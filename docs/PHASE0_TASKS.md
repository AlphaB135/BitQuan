# BitQuan Phase 0 — Implementation Spec for Autonomous Agent

**Status:** Pre-testnet. Foundation work only. No new features.  
**For:** Any AI agent executing tasks independently.  
**Repo:** `https://github.com/AlphaB135/BitQuan`  
**Local path (already cloned):** `/home/ubuntu/bitquan-audit`  
**Branch strategy:** Each task gets its own branch `fix/task-N-description`, PR against `main`.  
**Commit style:** `fix(crate): description — closes #N`  
**Test runner:** `cargo nextest run --package <crate> --locked`  
**Do NOT build the whole workspace** (disk space constraint) — always use `--package`.

---

## How to use this file

Each task below is self-contained. Read the task, read the files listed in "Read first", make the changes described, verify with the given test commands, then commit and open a PR. Stop after the PR is open — do not merge.

If a task fails verification after two attempts, open a GitHub issue describing what went wrong and stop.

---

## TASK 1 — Wire run_node() subsystems (Issue #143)

**Priority:** P0 — blocks everything downstream  
**Branch:** `fix/task-1-wire-run-node`  
**Crate:** `crates/node`

### Problem

`run_node()` in `crates/node/src/lib.rs:55` creates a `ConsensusEngine` and `InMemoryChainStore` but immediately discards them (assigned to `_engine` and `_storage`). The function only starts the P2P server. No block sync, no mempool, no RPC. The node is a shell.

### Read first

1. `crates/node/src/lib.rs` — full file
2. `crates/node/src/sync_task.rs` — block sync logic
3. `crates/node/src/rpc.rs` — RPC server startup
4. `crates/node/src/miner.rs` — HybridMiner struct
5. `crates/consensus/src/lib.rs` lines 1–100 — ConsensusEngine API
6. `crates/storage/src/lib.rs` lines 200–300 — InMemoryChainStore
7. `crates/mempool/src/lib.rs` lines 84–130 — Mempool::new()

### Required changes

**File: `crates/node/src/lib.rs`**

Replace the body of `run_node()` with a real wiring sequence:

```rust
pub async fn run_node(
    config_path: &str,
    rpc_bind: Option<&str>,
    p2p_bind: Option<&str>,
    network: NetworkId,
) -> Result<()> {
    let p2p_addr = p2p_bind.unwrap_or("0.0.0.0:18444");
    let rpc_addr = rpc_bind.unwrap_or("127.0.0.1:18332");

    log::info!("Starting BitQuan node | config={config_path} | p2p={p2p_addr} | rpc={rpc_addr}");

    // 1. Crypto registry (Dilithium5 provider)
    let registry = Arc::new(CryptoRegistry::default());

    // 2. Consensus engine
    let params = match network {
        NetworkId::Mainnet => ConsensusParams::phase3_defaults(),
        NetworkId::Testnet => ConsensusParams::testnet_hybrid(),
        _ => ConsensusParams::devnet_hybrid(),
    };
    let network_params = NetworkParams::mainnet(); // use network-specific when available
    let consensus = Arc::new(Mutex::new(
        ConsensusEngine::new(params, (*registry).clone())
    ));

    // 3. Storage (in-memory for now; replace with RocksDB for production)
    let store = Arc::new(Mutex::new(InMemoryChainStore::new()));

    // 4. Mempool
    let mempool = Arc::new(Mutex::new(
        Mempool::new().map_err(|e| Error::Other(e.to_string()))?
    ));

    // 5. P2P server (background task)
    let peer_manager = Arc::new(AsyncPeerManager::new(100, network));
    spawn_p2p_server_with_limit(p2p_addr, peer_manager.clone(), 100)
        .await
        .map_err(|e| Error::Net(e.to_string()))?;
    log::info!("P2P server running on {p2p_addr}");

    // 6. Sync task (background task) — pulls blocks from peers into store+consensus
    let sync = SyncTask::new(
        Arc::clone(&store),
        Arc::clone(&consensus),
        peer_manager.clone(),
        network_params.clone(),
    );
    tokio::spawn(async move { sync.run().await });

    // 7. RPC server (background task)
    // Wire consensus and store into RPC handler
    // TODO: implement full RPC handler wiring in crates/node/src/rpc.rs
    // For now: start RPC with a stub handler so the port is bound
    log::info!("Node subsystems wired. Entering main loop.");

    // 8. Main loop — heartbeat and peer maintenance
    loop {
        sleep(std::time::Duration::from_secs(30)).await;
        peer_manager.cleanup_peers().await;
        let peers = peer_manager.ready_peer_count().await;
        let height = store.lock().await.height();
        log::info!("height={height} peers={peers}");
    }
}
```

**Note:** If `SyncTask::new()` does not accept these exact arguments, read `crates/node/src/sync_task.rs` first and match its actual constructor. Do not fabricate method signatures — read the code.

**File: `crates/node/src/lib.rs` — add missing imports**

Add to the top of the file whatever imports are needed for `Arc`, `Mutex`, `Mempool`, `ConsensusEngine`, `InMemoryChainStore`, `NetworkParams`, `SyncTask`, `AsyncPeerManager`, `spawn_p2p_server_with_limit`. Use only types that already exist in the codebase — do not add new dependencies.

### Verification

```bash
cd /home/ubuntu/bitquan-audit

# Must compile without errors
cargo check --package bitquan-node --locked 2>&1

# Run node tests
cargo nextest run --package bitquan-node --locked 2>&1
```

### Success criteria

- `cargo check --package bitquan-node` exits 0 with no errors
- No `_engine`, `_storage`, `_rpc_addr` unused-variable suppression prefixes remain in `run_node()`
- All existing tests in `crates/node` still pass
- Add a new integration test `tests/node_starts.rs` that calls `run_node()` on a random port with `NetworkId::Devnet` and asserts it doesn't error in the first 2 seconds

### PR description template

```
fix(node): wire run_node() subsystems — closes #143

Previously run_node() created ConsensusEngine and InMemoryChainStore
but discarded them immediately. The node had no block sync, mempool
integration, or working RPC.

Changes:
- Wire ConsensusEngine, InMemoryChainStore, Mempool into run_node()
- Spawn SyncTask background task for block synchronisation
- Bind P2P and RPC servers on configured addresses
- Main loop reports height and peer count every 30s

Test: add tests/node_starts.rs integration test
```

---

## TASK 2 — Implement InMemoryChainStore missing operations (Issue #144)

**Priority:** P0 — required for unit testing reorgs and UTXO logic  
**Branch:** `fix/task-2-inmemory-chainstore`  
**Crate:** `crates/storage`

### Problem

`InMemoryChainStore` implements `ChainStore` but several methods are stubs:

- `get_transaction()` always returns `Ok(None)` — ignores stored blocks
- `get_utxo()` always returns `Ok(None)` — UTXO state not tracked at all
- `disconnect_block()` returns `Err("not supported")` — reorg testing impossible
- `put_utxo()` / `delete_utxo()` do nothing — UTXO writes silently lost

### Read first

1. `crates/storage/src/lib.rs` — full file (InMemoryChainStore, ChainStore trait)
2. `crates/storage/src/rocksdb_store.rs` lines 1318–1540 — reference implementation of the same methods in RocksDB (use as logic reference, do NOT copy RocksDB-specific code)
3. `crates/types/src/transaction.rs` — Transaction struct, txid() method
4. `crates/consensus/src/utxo.rs` — OutPoint, UtxoEntry

### Required changes

**File: `crates/storage/src/lib.rs`**

**Step 1:** Add UTXO and transaction index fields to `InMemoryChainStore`:

```rust
pub struct InMemoryChainStore {
    blocks: HashMap<[u8; 32], Block>,
    by_height: Vec<Block>,
    tip: Option<BlockHeader>,
    times: Vec<u32>,
    height: u64,
    // ADD these two:
    tx_index: HashMap<[u8; 32], Transaction>,  // txid → transaction
    utxo_set: HashMap<Vec<u8>, Vec<u8>>,        // outpoint bytes → utxo data bytes
}
```

Update `new()` and `Default` to initialise the new fields with `HashMap::new()`.

**Step 2:** Fix `insert_block()` to index transactions:

```rust
fn insert_block(&mut self, block: Block) -> Result<(), StorageError> {
    let id = header_id(&block.header);
    self.times.push(block.header.time);
    if self.times.len() > 11 {
        self.times.remove(0);
    }
    self.height = self.height.saturating_add(1);
    self.tip = Some(block.header.clone());
    self.blocks.insert(id, block.clone());

    // Index each transaction by its txid
    for tx in &block.transactions {
        self.tx_index.insert(tx.txid(), tx.clone());
    }

    self.by_height.push(block);
    Ok(())
}
```

**Step 3:** Implement `get_transaction()`:

```rust
fn get_transaction(&self, txid: &[u8; 32]) -> Result<Option<Transaction>, StorageError> {
    Ok(self.tx_index.get(txid).cloned())
}
```

**Step 4:** Implement `put_utxo()` and `get_utxo()` and `delete_utxo()`:

```rust
fn put_utxo(&mut self, outpoint: &[u8], data: &[u8]) -> Result<(), StorageError> {
    self.utxo_set.insert(outpoint.to_vec(), data.to_vec());
    Ok(())
}

fn get_utxo(&self, outpoint: &[u8]) -> Result<Option<Vec<u8>>, StorageError> {
    Ok(self.utxo_set.get(outpoint).cloned())
}

fn delete_utxo(&mut self, outpoint: &[u8]) -> Result<(), StorageError> {
    self.utxo_set.remove(outpoint);
    Ok(())
}
```

**Step 5:** Implement `disconnect_block()`:

```rust
fn disconnect_block(&mut self, block: &Block) -> Result<(), StorageError> {
    // Remove the block from all indices
    let id = header_id(&block.header);
    self.blocks.remove(&id);

    // Remove transactions from tx_index
    for tx in &block.transactions {
        self.tx_index.remove(&tx.txid());
    }

    // Roll back height and tip
    if self.height > 0 {
        self.height -= 1;
    }
    self.by_height.pop();
    self.tip = self.by_height.last().map(|b| b.header.clone());

    // Roll back times ring buffer (remove last entry)
    if !self.times.is_empty() {
        self.times.pop();
    }

    Ok(())
}
```

### Tests to add

Add to the `#[cfg(test)]` section at the bottom of `crates/storage/src/lib.rs`:

```rust
#[test]
fn get_transaction_finds_tx_after_insert_block() {
    let mut store = InMemoryChainStore::new();
    let block = test_block_with_one_tx(); // use existing test helper or create minimal one
    let txid = block.transactions[0].txid();
    store.insert_block(block).unwrap();
    assert!(store.get_transaction(&txid).unwrap().is_some());
}

#[test]
fn utxo_roundtrip() {
    let mut store = InMemoryChainStore::new();
    let key = b"outpoint_bytes_here";
    let val = b"utxo_data_here";
    store.put_utxo(key, val).unwrap();
    assert_eq!(store.get_utxo(key).unwrap(), Some(val.to_vec()));
    store.delete_utxo(key).unwrap();
    assert_eq!(store.get_utxo(key).unwrap(), None);
}

#[test]
fn disconnect_block_removes_transactions() {
    let mut store = InMemoryChainStore::new();
    let block = test_block_with_one_tx();
    let txid = block.transactions[0].txid();
    store.insert_block(block.clone()).unwrap();
    assert_eq!(store.height(), 1);
    store.disconnect_block(&block).unwrap();
    assert_eq!(store.height(), 0);
    assert!(store.get_transaction(&txid).unwrap().is_none());
}
```

### Verification

```bash
cargo nextest run --package bitquan-storage --locked 2>&1
```

### Success criteria

- All 3 new tests pass
- All existing storage tests still pass
- `get_transaction()`, `get_utxo()`, `put_utxo()`, `delete_utxo()`, `disconnect_block()` return real data
- No `Ok(None)` stubs remain for implemented methods

---

## TASK 3 — Replace unwrap/expect in consensus-critical paths

**Priority:** P1  
**Branch:** `fix/task-3-remove-unwrap-consensus`  
**Crate:** `crates/consensus`

### Problem

Any `unwrap()` or `expect()` in consensus code that is reachable from network input will panic the node when a malicious peer sends unexpected data. This is a DoS vector.

### Find all occurrences

```bash
grep -rn "\.unwrap()\|\.expect(" \
  /home/ubuntu/bitquan-audit/crates/consensus/src/ \
  /home/ubuntu/bitquan-audit/crates/types/src/ \
  --include="*.rs" | grep -v "#\[cfg(test)\]" | grep -v "// SAFETY" | grep -v "tests.rs"
```

### Required changes

For each `unwrap()` or `expect()` found outside `#[cfg(test)]` blocks:

1. If it is unreachable in practice (e.g. after a length check that guarantees the Option is Some), replace with a comment and return an appropriate error instead of panicking
2. Use `ok_or(ConsensusError::...)` or `ok_or_else(|| ...)` pattern
3. Never use `unreachable!()` as a substitute — use a real error variant

**Do NOT** modify test code (inside `#[cfg(test)]` or `_tests.rs` files).

### Verification

```bash
# Should return zero lines (outside test code)
grep -rn "\.unwrap()\|\.expect(" \
  /home/ubuntu/bitquan-audit/crates/consensus/src/ \
  --include="*.rs" | grep -v "#\[cfg(test)" | grep -v "_tests.rs" | grep -v "tests/" 2>&1

cargo nextest run --package bitquan-consensus --locked 2>&1
```

### Success criteria

- Zero `unwrap()`/`expect()` in non-test consensus code
- All existing tests pass
- No new `unreachable!()` macros added

---

## TASK 4 — Fix remaining 14 high-severity issues (H-1 through H-14)

**Priority:** P1  
**These are tracked in GitHub Issues #209–#222**

Each issue has a detailed description with:
- Exact file and line number
- Vulnerable code snippet
- Exact fix

Work through them in order: #209, #210, #211, #212, #213, #214, #215, #216, #217, #218, #219, #220, #221, #222.

For each issue:
1. Read the issue on GitHub: `gh issue view N --repo AlphaB135/BitQuan`
2. Read the relevant source file
3. Apply the fix described in the issue
4. Write a test that would have caught the bug
5. Verify: `cargo nextest run --package <affected-crate> --locked`
6. Commit: `fix(crate): description — closes #N`
7. Open PR

**One PR per issue** — do not batch them.

---

## TASK 5 — Set up cargo-fuzz for network and consensus (nightly only)

**Priority:** P1  
**Branch:** `feat/task-5-fuzz-targets`

### Required fuzz targets

Create `fuzz/fuzz_targets/` with these files:

**`fuzz/fuzz_targets/decompress_block.rs`**
```rust
#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = bitquan_network::compression::decompress_block(data);
});
```

**`fuzz/fuzz_targets/script_execute.rs`**
```rust
#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    use bq_crypto::CryptoRegistry;
    use bitquan_consensus::script::ScriptInterpreter;
    let registry = CryptoRegistry::default();
    let mut interp = ScriptInterpreter::new(registry);
    let _ = interp.execute(data, &[0u8; 32]);
});
```

**`fuzz/fuzz_targets/asert_next_target.rs`**
```rust
#![no_main]
use libfuzzer_sys::fuzz_target;
use arbitrary::Arbitrary;

#[derive(Arbitrary, Debug)]
struct FuzzInput {
    anchor: [u8; 32],
    height_delta: i64,
    time_delta: i64,
}

fuzz_target!(|input: FuzzInput| {
    use bitquan_consensus::{asert_next_target, ConsensusParams};
    let params = ConsensusParams::phase3_defaults();
    let _ = asert_next_target(
        input.anchor,
        input.height_delta,
        input.time_delta,
        &params,
        None,
    );
});
```

### Verification

```bash
# Build only (do not run — takes too long)
cargo +nightly fuzz build decompress_block 2>&1
cargo +nightly fuzz build script_execute 2>&1
cargo +nightly fuzz build asert_next_target 2>&1
```

### Success criteria

All three fuzz targets build without errors under nightly. CI will run them for 120 seconds in the nightly workflow.

---

## Global rules for all tasks

- **Read before writing.** Always read the file you are about to edit before editing it.
- **One crate at a time.** Use `--package <name>` not `--workspace` to avoid disk/time blowout.
- **No new dependencies.** Do not add entries to any `Cargo.toml`. Use only what is already there.
- **No `git push --force`.** Never force-push.
- **No `rm -rf` without confirmation.**
- **If a file path doesn't exist**, run `find /home/ubuntu/bitquan-audit/crates -name "*.rs" | grep <keyword>` to locate it before giving up.
- **If compilation fails after two attempts with the same approach**, stop, open a GitHub issue, and describe the blocker clearly.
- **Commit only when tests pass.** If tests fail, fix them before committing.

---

*Spec version: 1.0 — July 2026*  
*Authored by: Hermes (ซากุระ) 🌸*
