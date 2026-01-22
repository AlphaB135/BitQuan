# Main.rs Module Refactoring - Split 4,714 Lines into Organized Modules

**Date**: 2026-01-19
**Context**: Refactoring monolithic main.rs (4,714 lines) into organized command modules for maintainability

## What We Learned

### 1. Module Organization Pattern
แยก functions ออกจาก main.rs ตามความรับผิดชอบ:
```
crates/node/src/
├── cli.rs                  # Shared helper functions
├── commands/
│   ├── mod.rs              # Module exports
│   ├── wallet.rs           # 13 wallet functions
│   ├── rpc.rs              # 7 RPC/JWT functions
│   ├── node.rs             # 6 node utility functions
│   ├── mining.rs           # 8+ mining functions
│   └── p2p.rs              # 10+ P2P functions
```

### 2. Import Resolution Hell
**Problem**: `wallet::address` vs `crate::address` - 2 separate modules!
- `wallet::address` → local module → **WRONG** (doesn't exist)
- `crate::address` → standalone module → **CORRECT**

**Pattern**: Always use `crate::` prefix for sibling modules:
```rust
// ❌ WRONG - assumes external crate
use wallet::{WalletKeypair, WalletAlgorithm};

// ✅ CORRECT - local module
use crate::wallet::{WalletKeypair, WalletAlgorithm};
```

### 3. Function Visibility After Move
เมื่อย้าย function ไป module ใหม่ ต้องใส่ `pub`:
```rust
// commands/wallet.rs
pub fn wallet_gen(...) -> Result<()> { ... }
```

แล้ว import ใน main.rs:
```rust
use commands::wallet::wallet_gen;
```

### 4. AsyncStoreWrapper + TcpStream Conversion
**Problem**: Peer::new_inbound() requires `std::net::TcpStream` แต่ tokio gives `tokio::net::TcpStream`

**Solution**:
```rust
// Convert tokio stream to std stream
let std_stream = stream.into_std()?;
let peer = Peer::new_inbound(std_stream, peer_addr, magic, &noise_config)?;
```

### 5. WorkerContext Integration Pattern
New worker architecture requires:
- ConsensusEngine (with ConsensusParams)
- BanManager (with BanConfig)
- ForkChoice
- Arc<dyn AsyncChainStore>
- Mempool
- PeerManager

```rust
let worker_ctx = Arc::WorkerContext::new(
    peer_manager.clone(),
    store.clone(),
    mempool.clone(),
    consensus.clone(),
    fork_choice.clone(),
    ban_manager.clone(),
    network,
    GENESIS_HASH_BYTES,
);
```

### 6. Type Consistency Issues
**Problem**: `rl_burst: u32` vs `usize` mismatch

**Root Cause**: CLI args give `u32` but struct used `usize`

**Solution**: Standardize on `u32` for rate limiting:
```rust
pub struct RpcServerOptions<'a> {
    pub rl_burst: u32,        // changed from usize
    pub rl_refill_per_sec: u32,
    ...
}
```

### 7. Duplicate File Detection
Found `chain_state.rs` AND `chainstate.rs`:
- `chain_state.rs` → old version, simple (no Arc wrap)
- `chainstate.rs` → new version, proper (Arc + tip_hash)

**Fix**: Delete old, update references:
```rust
// lib.rs
pub mod chainstate;  // was: chain_state
pub use chainstate::ChainState;

// block_submit.rs
pub chain_state: Option<Arc<crate::chainstate::ChainState>>,
```

### 8. Security Audit Pattern
**Linus-Style Review** - ผ่าน 100%:
- 8 `panic!` calls → All in `#[test]` blocks ✅
- 4 `unwrap()` calls → All fixed or in test code ✅
- Final fix: `NoiseConfig::generate().unwrap()` → `.map_err(...)?`

```rust
// BEFORE (production code):
NoiseConfig::generate().unwrap()

// AFTER:
NoiseConfig::generate().map_err(|e| {
    Error::Invalid(format!("Failed to generate Noise config: {}", e))
})?
```

## Why It Matters

1. **Maintainability**: 2,265 lines is still large but manageable vs 4,714
2. **Compilation Speed**: Module changes don't force full rebuild
3. **Testing**: Each module can be tested independently
4. **Onboarding**: New devs can understand structure quickly
5. **Code Discovery**: Functions are logically grouped

## How To Apply

### For Future Refactoring
1. **Create cli.rs first** - extract shared helpers (invalid, format_bq, etc.)
2. **Group by responsibility** - wallet functions → wallet.rs, etc.
3. **Update imports systematically** - use `crate::module::` for local modules
4. **Test incrementally** - `cargo check` after each module move
5. **Run cargo fix** - auto-fix 48% of warnings

### Avoid These Mistakes
- ❌ Assuming `wallet::` refers to local module (it's external crate check)
- ❌ Forgetting `pub` on moved functions
- ❌ Not checking type mismatches (u32 vs usize) across module boundaries
- ❌ Leaving duplicate files (use `find` + `grep` to detect)

### Detection Commands
```bash
# Find duplicate files
find src -name "*similar*"

# Check for unused imports
cargo clippy

# Fix automatically
cargo fix --bin "bitquan-node" --allow-dirty

# Count lines per file
wc -l src/**/*.rs | sort -n | tail -20
```

## Key Files Modified

| File | Change | Lines Changed |
|------|--------|---------------|
| `src/cli.rs` | Created | +100 |
| `src/commands/mod.rs` | Created | +5 |
| `src/commands/wallet.rs` | Created | +700 |
| `src/commands/rpc.rs` | Created | +350 |
| `src/commands/node.rs` | Created | +325 |
| `src/commands/mining.rs` | Created | +900 |
| `src/commands/p2p.rs` | Created | +800 |
| `src/main.rs` | Refactored | -450 (net) |
| `src/chain_state.rs` | Deleted | -80 |
| `src/lib.rs` | Updated | +2 |

## Results

- **Errors**: 56 → 0 ✅
- **Warnings**: 68 → 35 (-48%)
- **Main.rs**: 4,714 → 2,265 lines (-52%)
- **Compilation**: ✅ PASS
- **Production Scan**: ✅ PASS (0 P0 issues)

## Tags
`refactoring` `module-system` `rust` `organization` `main.rs` `code-quality`
