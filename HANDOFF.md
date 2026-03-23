# BitQuan Project Handoff

**Last updated**: 2026-03-23
**Project**: BitQuan - Post-Quantum Proof-of-Work Blockchain
**Repo**: `AlphaB135/BitQuan`
**Working branch**: `main`

---

## Current State

### Build & Tests

```
cargo check   -- PASS (warnings only, 0 errors)
cargo clippy  -- PASS (17 warnings: dead_code from new Stratum security, ethash cfg)
cargo test    -- PASS (787 tests, 0 failures, 12 ignored)
```

All 5 feature branches have been merged into main. Working tree is clean.
71 local commits ahead of `origin/main` -- **not yet pushed**.

### Commit History (recent)

```
5e9afd0 merge: integrate consensus-mining-protocol (6 commits)
602e6d8 merge: integrate ci/final-audit-and-release (2 commits)
db945b4 Merge remote-tracking branch 'origin/fix/logging-migration-final'
9847c19 merge: integrate fix/ci-issue (21 commits)
abca6eb merge: integrate fix/master-data-integrity (34 commits)
af80955 chore: remove tracked junk files and add gitignore rules
e2a9aa0 feat(consensus): add GHOST protocol with uncle validation and tokenomics
```

### Merged Branches (safe to delete)

| Branch | Commits | Content |
|--------|---------|---------|
| `fix/master-data-integrity` | 34 | C1-C7 data integrity fixes, height validation, UTXO cleanup |
| `fix/ci-issue` | 21 | CI workflow fixes, clippy, dead code cleanup |
| `fix/logging-migration-final` | 1 | println! to log:: macros |
| `ci/final-audit-and-release` | 2 | Audit reports, v0.0.2-alpha bump |
| `consensus-mining-protocol` | 6 | ASERT config, Stratum security, mining GUI |

### Unmerged Remote Branches (NOT yet handled)

| Branch | Type | Action Needed |
|--------|------|---------------|
| `origin/chores/ci-and-quality` | Feature | Review and merge or close |
| `origin/ci/code-audit-md-cleanup` | Cleanup | Review and merge or close |
| `origin/copilot/*` (4 branches) | Copilot | Low priority, review individually |
| `origin/dependabot/*` (12 branches) | Deps | Review and merge dependency updates |

---

## Architecture

### Crate Structure (18 crates)

```
crates/
  types/       -- Core types: Block, BlockHeader, Transaction, TxIn, TxOut, Witness
  consensus/   -- Validation: ASERT difficulty, PoW, fork choice, GHOST protocol
  crypto/      -- CRYSTALS-Dilithium5 signatures, constant-time ops
  network/     -- P2P protocol, TCP transport, message handling
  storage/     -- ChainStore (RocksDB + in-memory), AsyncChainStore
  node/        -- CLI binary, miner, stratum server, wallet, sync
  mempool/     -- Transaction pool with ancestry/descendant limits
  rpc/         -- JSON-RPC server (rocksdb-backend feature)
  wallet/      -- Keystore, mnemonic, multisig, address generation
  faucet/      -- Testnet faucet
  layer2/      -- ZK-rollup scaffolding (early stage)
  shard/       -- Sharding research (early stage)
  channels/    -- Payment channels (early stage)
  common/      -- Shared utilities
  bq-sdk/      -- SDK for third-party integration
  tools/       -- bitquan-gui (Tauri mining dashboard)
```

### Key Design Decisions

1. **DifficultyParams uses flat fields** -- `params.difficulty.target_block_time` and `params.difficulty.difficulty_half_life` (NOT nested `AsertParams`). The `consensus-mining-protocol` branch introduced a nested `AsertParams` struct but we kept the flat approach because it's what the entire codebase uses.

2. **Phase 4 block time is 120s** -- `DifficultyParams::mainnet()` uses `target_block_time: 120` (2 minutes), not 600s. This was a deliberate Phase 4 decision.

3. **ASERT uses integer fixed-point math** -- 32.32 format (FP_SCALE = 2^32). Zero floating-point in consensus calculations. See `crates/consensus/src/asert.rs`.

4. **GHOST Protocol implemented** -- Uncle blocks (max 2 per block, depth 1-7) with reward formula `(8-depth)/8 * base_subsidy`. Nephew bonus: `1/32 * base_subsidy per uncle`. See `crates/consensus/src/lib.rs` lines 580-608.

5. **BlockHeader has `uncles_hash` field** -- Added to types. All BlockHeader initializers must include `uncles_hash: [0u8; 32]` and Block initializers must include `uncles: vec![]`.

6. **validate_block takes 10 arguments** -- Signature: `(block, height, params, registry, network_id, genesis_hash, total_fees, median_time_past, uncles_ctx, past_uncle_hashes)`. The `total_fees` parameter is mandatory (no loose validation).

7. **Stratum server has security enhancements** -- `MinerSession::new()` takes 4 args including `client_ip: String`. `StratumConfig` has security fields: `require_auth`, `max_connections_per_ip`, `max_share_rate`, `connection_timeout`, `max_connections`, `enable_rate_limiting`.

8. **Functions moved to modules** -- `main.rs` was truncated to ~566 lines. Mining functions are in `commands::mining`, node functions in `commands::node`, CLI helpers in `cli` module.

### Key Files

| File | Purpose |
|------|---------|
| `crates/consensus/src/lib.rs` | Core validation, GHOST protocol, coinbase checks |
| `crates/consensus/src/asert.rs` | ASERT difficulty with integer fixed-point math |
| `crates/consensus/src/pow.rs` | PoW engines: SHA-256d, RandomX, Ethash |
| `crates/node/src/chainstate.rs` | Chain state with validated headers cache for IBD |
| `crates/node/src/worker.rs` | Node worker with uncle pre-fetch (lines 719-774) |
| `crates/node/src/stratum_server.rs` | Stratum mining server with security |
| `crates/node/src/main.rs` | CLI entrypoint, run_node() |
| `crates/types/src/block.rs` | Block, BlockHeader (with uncles_hash, uncles) |
| `.gitignore` | Blocks tools/ output files, oracle patterns |

---

## Immediate TODO (Priority Order)

### 1. Push to Remote

71 commits are local only. Push when ready:

```bash
git push origin main
```

### 2. Handle Untracked Files (2 files)

These exist on disk but are not committed:

- `crates/network/src/compression.rs` (92 lines) -- BQIP-0006 zstd compression for P2P
- `crates/types/src/aggregation.rs` (129 lines) -- BQIP-0008 same-sender input aggregation

Both are complete implementations that need to be:
1. Reviewed for correctness
2. Integrated into their respective crate's `mod.rs`
3. Committed

### 3. Review and Merge Remaining Branches

- `origin/chores/ci-and-quality` -- Check what it adds, merge if useful
- `origin/ci/code-audit-md-cleanup` -- Likely cleanup, merge if safe
- `origin/dependabot/*` -- Dependency updates (12 branches). Review carefully:
  - `bincode-3.0.0`, `dashmap-6.1.0`, `primitive-types-0.14.0` -- Breaking?
  - `rand_chacha-0.9.0`, `rand_core-0.9.3` -- Usually safe
  - `rocksdb-0.24.0` -- Check API changes
  - `thiserror-2.0.17` -- Usually safe
  - GitHub Actions updates -- Safe to merge
- `origin/copilot/*` -- Low priority, review individually

### 4. Code Audit & Vulnerability Check

This was the planned next step. Key areas to audit:

- **GHOST Protocol** (`consensus/src/lib.rs:580-608`) -- Uncle validation logic
- **ASERT precision** (`consensus/src/asert.rs`) -- Integer fixed-point edge cases
- **Stratum security** (`node/src/stratum_server.rs`) -- New security methods are dead_code, need wiring
- **Fork choice** (`consensus/src/fork.rs`) -- Reorg handling with invalid blocks
- **Chain store** (`storage/`) -- Data integrity after C1-C7 fixes
- **P2P sync** (`network/src/sync.rs`) -- IBD and block propagation

### 5. Wire Up Dead Code (Stratum Security)

The `consensus-mining-protocol` merge added security methods that are currently unused:

```rust
// In StratumServer:
is_ip_allowed(), is_connection_limit_exceeded(), is_total_connection_limit_exceeded(),
register_connection(), unregister_connection(), ban_ip()

// In MinerSession:
authenticate(), is_authorized(), update_activity(), is_timed_out(), check_rate_limit()

// In StratumAuth:
new(), verify_password()

// In ShareRateLimiter:
check_share_rate()
```

These need to be called from the connection handling flow in `handle_client()`.

### 6. Add `ethash` Feature to Cargo.toml

Clippy warns: `unexpected cfg condition value: ethash`. The ethash code in `pow.rs` is gated on `#[cfg(feature = "ethash")]` but the feature isn't defined in `Cargo.toml`. Either:
- Add `ethash` feature with `ethash` and `ethereum-types` dependencies
- Or remove the ethash code if not planned for use

### 7. Clean Up Merge Warnings

Minor issues to address:
- `crates/node/src/stratum_server.rs` -- 13 dead_code warnings (security methods)
- `crates/node/src/mnemonic.rs` -- Unused constants and functions
- `crates/consensus/src/pow.rs` -- `ethash` cfg warnings (3)

---

## Technical Reference

### ASERT Difficulty (Phase 4)

```
target_block_time: 120s (2 min)
difficulty_half_life: 14,400s (4 hours)
burst_guard_window: 11 blocks
burst_guard_floor_ratio: 0.33 (fixed-point 1417339207)
burst_guard_cooldown: 5 blocks
```

### Reward Schedule

```
initial_subsidy: 50 BQ (50 * 10^18 qbits)
halving_interval: 210,000 blocks
tail_emission: 0.5 BQ per block (floor)
uncle_reward: (8 - depth) / 8 * base_subsidy (depth 1-7)
nephew_reward: 1/32 * base_subsidy per uncle
max_uncles_per_block: 2
```

### Weight Formula (BQIP-0002)

```
tx_weight = base_bytes * 4 + witness_bytes * 1
block_weight = sum(tx_weights)
max_block_weight = 4,000,000 WU
```

### Build Commands

```bash
cargo build --release           # Production build
cargo build --release --locked  # Reproducible build (no Cargo.lock update)
cargo test                     # Run all tests (787 tests)
cargo clippy -- -D warnings    # Strict lint (will fail on dead_code)
cargo test -p bitquan-consensus # Test single crate
cargo fuzz                     # Fuzz testing (needs nightly)
```

### Features

```bash
cargo build --features testing       # Mock PoW for development
cargo build --features randomx       # Enable RandomX mining
cargo build --features rocksdb-backend # Enable RocksDB + JSON-RPC
```

---

## Coding Standards

- **No emojis** in code
- **No panic!/unwrap** in production code (Linus Rule)
- **Checked arithmetic** for all financial calculations
- **Max 3 nesting levels** -- extract functions when deeper
- **Descriptive variable names** -- no AI-compressed abbreviations
- **Warnings = errors** -- fix all warnings
- **No TODO/FIXME** -- ship complete code only
- **rustfmt.toml**: max_width=80, tab_spaces=2, brace_style=AlwaysNextLine
- **Commit style**: conventional commits (`feat:`, `fix:`, `chore:`, `merge:`)
- **No Co-Authored-By** in commits (project rule)

---

## Known Issues & Gotchas

1. **ASERT integer math has ~1% precision** for small anchors with short time windows. Test with `anchor >= 50000` and `height_delta >= 20` for reliable results.

2. **`find_headers_after_async`** does a linear scan to find block heights -- O(n) per locator. This is fine for now but will need indexing for production.

3. **The `ethash` feature code exists but isn't wired** -- it uses `ethash` crate types that aren't in Cargo.toml dependencies.

4. **`tools/` directory is partially gitignored** -- `tools/bitquan-gui/` exists but needs `-f` flag to add. Tool output files (`*.txt`, `*.json`) are gitignored.

5. **BlockHeader::uncles_hash is always [0u8; 32]** for now -- the actual uncle hash computation isn't implemented yet (GHOST protocol validates uncles but doesn't compute the hash field).

6. **Stratum security methods are dead code** -- the connection handling in `handle_client()` doesn't call `register_connection()`, `authenticate()`, etc. yet.
