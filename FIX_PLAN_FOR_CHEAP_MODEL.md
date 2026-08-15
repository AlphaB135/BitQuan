# BitQuan Vulnerability Fix Plan — Instructions for Cheap Model

**Created by**: Hermes (ซากุระ) 🌸 — Blue Team Lead
**Date**: 2026-08-15
**Purpose**: Step-by-step fix instructions. Read carefully, follow exactly.

---

## RULES

1. Read the file before editing.
2. Make ONLY the changes described — nothing else.
3. Run `cargo test -p <crate>` after each fix.
4. If tests fail, revert and report the error.
5. Commit each fix separately with the message format given.

---

## FIX-001 (CRITICAL): CHAIN-007 — Merkle Root Calculated After Mining

**File**: `crates/node/src/rpc.rs`
**Problem**: The block header is mined with `merkle_root = hash(coinbase_tx_only)`.
After mining, mempool transactions are added and `header.merkle_root` is overwritten with a
new value — making the stored PoW hash invalid. All peers reject the block.

### Exact Change

Find the section inside `generatetoaddress` (around line 640) that looks like:

```rust
let mut merkle_root = [0u8; 32];
merkle_root.copy_from_slice(&txid);
```

This sets merkle_root to just the coinbase txid. Below it is the BlockHeader construction,
the mining loop, and THEN the mempool transaction fetch.

**Move the entire mempool transaction fetch BEFORE the BlockHeader construction.**

The new order must be:

1. Build coinbase transaction (keep as-is)
2. **NEW STEP — fetch mempool transactions HERE** (move from after mining to before)
3. Build the complete `transactions` vec (coinbase + mempool)
4. Calculate merkle_root from ALL transactions using `bitquan_consensus::calculate_merkle_root`
5. Build BlockHeader using the correct merkle_root
6. Run the mining loop
7. Build the Block struct
8. Store the block

**Specifically**:

Find this block of code (currently AFTER the mining loop):
```rust
// Fetch pending transactions from mempool (if available)
let mut transactions = vec![coinbase_tx];

if let Some(mempool) = &self.mempool {
    let mut mp = mempool.lock().await;
    let selected = mp.select_for_block(4_000_000);
    if !selected.is_empty() {
        log::info!("Mining block with {} mempool transactions", selected.len());
        transactions.extend(selected.into_iter().map(|arc_tx| {
            std::sync::Arc::try_unwrap(arc_tx).unwrap_or_else(|arc| (*arc).clone())
        }));
    }
} else {
    log::info!("Warning: No mempool available for mining");
}

// Recalculate merkle root including all transactions
let merkle_root = bitquan_consensus::calculate_merkle_root(&transactions)
    .map_err(|e| RpcError::InternalError(format!("merkle root: {}", e)))?;
header.merkle_root = merkle_root;
```

Move this entire section (minus the `header.merkle_root = merkle_root;` assignment) to
BEFORE the BlockHeader construction. Then use the calculated `merkle_root` directly when
building the BlockHeader — not the coinbase-txid placeholder.

Also remove the now-redundant lines:
```rust
let mut merkle_root = [0u8; 32];
merkle_root.copy_from_slice(&txid);
```

And remove the now-redundant post-mining recalculation:
```rust
let merkle_root = bitquan_consensus::calculate_merkle_root(&transactions)
    .map_err(...)?;
header.merkle_root = merkle_root;
```

**After fix, the code flow should be**:
```rust
// 1. coinbase tx built here (already present)
let txid = coinbase_tx.txid();

// 2. Fetch mempool txs FIRST
let mut transactions = vec![coinbase_tx];
if let Some(mempool) = &self.mempool {
    let mut mp = mempool.lock().await;
    let selected = mp.select_for_block(4_000_000);
    if !selected.is_empty() {
        transactions.extend(selected.into_iter().map(|arc_tx| {
            std::sync::Arc::try_unwrap(arc_tx).unwrap_or_else(|arc| (*arc).clone())
        }));
    }
}

// 3. Calculate merkle root ONCE from all transactions
let merkle_root = bitquan_consensus::calculate_merkle_root(&transactions)
    .map_err(|e| RpcError::InternalError(format!("merkle root: {}", e)))?;

// 4. Build header with correct merkle root
let mut header = BlockHeader {
    ...
    merkle_root,   // ← use the calculated value here
    ...
};

// 5. Mine the block
for nonce in 0..max_nonce { ... }

// 6. Build and store block (no merkle_root mutation needed)
let block = Block { header, uncles: vec![], transactions };
```

### Test after fix
```bash
cargo test -p bitquan-node 2>&1 | tail -20
```

### Commit message
```
fix(rpc): calculate merkle root before mining to prevent PoW invalidation (CHAIN-007)

Previously, generatetoaddress mined the block header with a merkle root
derived from the coinbase transaction only. Mempool transactions were
fetched and the merkle root recalculated AFTER mining, mutating the
already-mined header. This made the stored PoW hash invalid — all peers
would reject the block because hash(header) != the target that was met.

Fix: move mempool fetch and merkle root calculation before BlockHeader
construction so the header mined matches the block that is stored.
```

---

## FIX-002 (CRITICAL): CHAIN-005 — DefaultHasher in Checkpoint Validation

**File**: `crates/network/src/sync.rs`
**Problem**: `compute_header_hash` uses `std::collections::hash_map::DefaultHasher` which is
non-cryptographic, platform-specific, and has no collision resistance. It only hashes 3
fields (time, bits, nonce) and stores 8 bytes in a 32-byte array. This is used for
checkpoint comparison — meaning an attacker can find collisions offline and serve a fake chain
that passes all checkpoints.

### Exact Change

Find function at ~line 768:
```rust
fn compute_header_hash(&self, header: &BlockHeader) -> [u8; 32] {
    // In production, this would use the actual block hashing algorithm
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    header.time.hash(&mut hasher);
    header.bits.hash(&mut hasher);
    header.nonce.hash(&mut hasher);
    let hash = hasher.finish();
    let mut result = [0u8; 32];
    result[..8].copy_from_slice(&hash.to_le_bytes());
    result
}
```

**Replace the entire function body** with a call to the proper block hashing function:

```rust
fn compute_header_hash(&self, header: &BlockHeader) -> [u8; 32] {
    bitquan_consensus::header_hash(header)
}
```

`bitquan_consensus::header_hash` is already used elsewhere in the codebase
(e.g., in `crates/node/src/rpc.rs` at the `let block_hash = bitquan_consensus::header_hash(&header)` line).
It uses SHA-256d which is the correct cryptographic hash.

Also check: at the top of `sync.rs`, verify `bitquan_consensus` is already in scope or add:
```rust
use bitquan_consensus::header_hash;
```
and call `header_hash(header)` directly.

**IMPORTANT**: After this change, the hardcoded checkpoint hashes in `SyncManager::new`
or wherever checkpoints are defined must also use SHA-256d hashes. Check if there are
hardcoded `[u8; 32]` checkpoint hash values — if they were derived from DefaultHasher,
they will now be wrong. If checkpoints are `[0u8; 32]` placeholders, leave them as-is
(the comparison will just never match, which is safe behavior).

### Test after fix
```bash
cargo test -p bitquan-network 2>&1 | tail -20
```

### Commit message
```
fix(sync): replace DefaultHasher with header_hash for checkpoint validation (CHAIN-005)

compute_header_hash was using std::collections::hash_map::DefaultHasher —
a non-cryptographic hasher with no collision resistance. This was used to
validate checkpoint hashes, meaning an attacker could find hash collisions
offline (birthday attack) and serve a fake chain that passes checkpoints.

Fix: delegate to bitquan_consensus::header_hash which uses SHA-256d,
the same algorithm used everywhere else for block identity.
```

---

## FIX-003 (MEDIUM): CHAIN-012 — Script op_count Reset in execute_continue

**File**: `crates/consensus/src/script.rs`
**Problem**: `execute_continue` resets `op_count = 0` before running scriptPubKey.
This means scriptSig can use up to MAX_OPS (201) ops AND scriptPubKey can also use
up to 201 ops — allowing 402 ops per input instead of 201. A block full of such
transactions can DoS validators with excess CPU.

### Exact Change

Find `execute_continue` at ~line 139:
```rust
pub fn execute_continue(&mut self, script: &[u8], message: &[u8]) -> Result<bool, ScriptError> {
    // Do NOT clear the stack — scriptSig values must be visible to scriptPubKey
    self.op_count = 0;    // ← THIS LINE is the bug
    self.execute_inner(script, message)
}
```

**Remove** the line `self.op_count = 0;` from `execute_continue`.

The final function should be:
```rust
pub fn execute_continue(&mut self, script: &[u8], message: &[u8]) -> Result<bool, ScriptError> {
    // Do NOT clear the stack — scriptSig values must be visible to scriptPubKey
    // Do NOT reset op_count — the combined scriptSig+scriptPubKey budget is MAX_OPS total
    self.execute_inner(script, message)
}
```

### Test after fix
```bash
cargo test -p bitquan-consensus 2>&1 | tail -20
```

### Commit message
```
fix(script): preserve op_count across execute/execute_continue (CHAIN-012)

execute_continue was resetting op_count to 0, giving scriptPubKey a fresh
op budget independent of scriptSig. This doubled the effective operation
limit per input to 402 ops, enabling CPU-exhaustion DoS via crafted txs.

Fix: remove the op_count reset from execute_continue so the combined
scriptSig + scriptPubKey execution stays within MAX_OPS total.
```

---

## FIX-004 (HIGH): CHAIN-009 — submitblock Returns Ok Without Validation

**File**: `crates/node/src/rpc.rs`
**Problem**: `submitblock` parses the block but performs zero validation and always
returns `Ok(true)`. The block is never stored, never connected to the chain, never
broadcast. Miners submitting blocks via RPC get false success while their work is wasted.

### What to do

This fix requires connecting `submitblock` to the storage layer. The function already
parses the block into `_block`. Rename `_block` to `block` and insert it via
`self.store.insert_block(block).await`.

**Find the submitblock function (~line 289)**:
```rust
async fn submitblock(&self, block_hex: String) -> Result<bool, RpcError> {
    let block_bytes = Vec::from_hex(&block_hex)
        .map_err(|_| RpcError::InvalidParams("block must be hex-encoded".into()))?;

    let _block: bitquan_types::Block = bitquan_storage::serialize::from_bytes(&block_bytes)
        .map_err(|e| RpcError::InvalidParams(format!("failed to parse block: {}", e)))?;

    // ...comments...
    log::info!("Received block submission via RPC");
    Ok(true)
}
```

**Replace with**:
```rust
async fn submitblock(&self, block_hex: String) -> Result<bool, RpcError> {
    let block_bytes = Vec::from_hex(&block_hex)
        .map_err(|_| RpcError::InvalidParams("block must be hex-encoded".into()))?;

    let block: bitquan_types::Block = bitquan_storage::serialize::from_bytes(&block_bytes)
        .map_err(|e| RpcError::InvalidParams(format!("failed to parse block: {}", e)))?;

    log::info!("Received block submission via RPC, height unknown");

    self.store
        .insert_block(block)
        .await
        .map_err(Self::storage_error_to_rpc)?;

    Ok(true)
}
```

This at minimum stores the block. Note: full PoW/consensus validation at the RPC layer
requires the `ConsensusEngine` — that is a larger refactor. The storage `insert_block`
call itself performs basic integrity checks. Document what is still missing.

### Test after fix
```bash
cargo test -p bitquan-node 2>&1 | tail -20
```

### Commit message
```
fix(rpc): wire submitblock to storage layer (CHAIN-009)

submitblock was parsing the block but discarding it, always returning
Ok(true). Miners received false success while their blocks were never
stored or broadcast. Fix connects the block to insert_block(). Full
consensus validation (PoW, timestamp, difficulty) remains a TODO
requiring ConsensusEngine integration.
```

---

## VERIFICATION CHECKLIST

After all 4 fixes, run:

```bash
cd /home/ubuntu/bitquan-audit

# 1. All crate tests
cargo test -p bitquan-consensus 2>&1 | tail -10
cargo test -p bitquan-network 2>&1 | tail -10
cargo test -p bitquan-node 2>&1 | tail -10
cargo test -p bitquan-mempool 2>&1 | tail -10

# 2. Clippy
cargo clippy -p bitquan-consensus -- -D warnings 2>&1 | tail -10
cargo clippy -p bitquan-node -- -D warnings 2>&1 | tail -10
cargo clippy -p bitquan-network -- -D warnings 2>&1 | tail -10

# 3. Git log to verify commits
git log --oneline -6
```

Report ALL output. If any test fails, do NOT commit that fix.

---

**End of fix plan — Hermes (ซากุระ) 🌸**
