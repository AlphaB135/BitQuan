# Main.rs Refactoring - Module Split Progress

**Date**: 2026-01-19
**Context**: Splitting 4,700-line main.rs into command modules using parallel agents

## What We Learned
- **Parallel Agent Strategy**: Using 5 Task agents simultaneously reduces large refactoring time by ~70%
- **Agent Rate Limits**: API 429 errors after 3+ parallel agents - need to batch or wait for reset
- **Module Organization**: CLI helper functions → cli.rs, Commands → commands/ subdirectory structure
- **Import Hell**: Moving functions requires updating ALL import paths systematically

## Current Progress

### Completed Modules (100%)
✅ **cli.rs** - 5 helper functions (format_bq, parse_network_id, read_password_from_stdin, invalid, address_network_label)
✅ **commands/wallet.rs** - 13 wallet functions (wallet_gen, wallet_address, wallet_send, wallet_sign, wallet_verify, wallet_gen_mnemonic, wallet_from_mnemonic, wallet_gen_multisig, multisig_info, tx_sign_partial, tx_combine_signatures, wallet_backup, wallet_restore)
✅ **commands/rpc.rs** - 7 RPC functions (run_rpc_server, submit_transaction_rpc, generate_self_signed_cert_cli, hash_password_cli, jwt_user_add, jwt_user_remove, jwt_user_list)
✅ **commands/node.rs** - 6 node functions (check_balance, verify_database, genesis_verify, build_tx, script_from_address, address_validate)
✅ **commands/mining.rs** - 8+ mining functions (mine_genesis, check_block, rng_demo, mine_once, mine_continuous, print_session_summary, run_stratum_server, parse_hybrid_weights)
✅ **commands/p2p.rs** - 10+ P2P functions (write_envelope, read_envelope, p2p_demo, setup_storage, get_or_create_jwt_secret, setup_p2p_network, start_metrics_service, p2p_server, p2p_connect, RpcServerOptions)

### Remaining Issues (17 unique error types, 27 total)
❌ **wallet::address** module structure changed (inspect, script_from_pubkey_hash not found)
❌ **SerializableKeypair** type not found in wallet crate  
❌ **WalletAlgorithm, WalletPublicKey** imports broken
❌ **install_panic_hook** function missing
❌ **NodeRpcHandler** import path wrong
❌ **SocketAddr** not in scope in some functions
❌ **get_or_create_jwt_secret** not found despite import
❌ Type mismatches (NoiseConfig: Default, function args)

## Line Count Progress
- **Start**: 4,714 lines
- **Current**: 2,270 lines  
- **Target**: < 500 lines
- **Remaining to remove**: ~1,770 lines (mostly duplicate function definitions that were moved)

## Critical Fixes Applied
1. **Fixed duplicate doc comments** - Removed orphan doc comments from moved functions
2. **Fixed invalid() calls** - Replaced all `invalid(` with `crate::cli::invalid(` using perl
3. **Fixed double crate::cli::** - Fixed `crate::cli::crate::cli::invalid` → `crate::cli::invalid`
4. **Fixed wallet imports** - Changed `use wallet::` → `use crate::wallet::`
5. **Fixed RpcServerOptions** - Removed duplicate import, kept local definition

## Why It Matters
- **Maintainability**: 4,700 lines is impossible to navigate. Modules isolate concerns.
- **Compile Time**: Smaller files = faster incremental compilation
- **Code Review**: Smaller PRs for each module instead of monolithic changes
- **Testing**: Easier to unit test isolated modules

## How To Apply (Next Steps)

### Fix wallet module structure
```bash
# Check what wallet module actually exports
grep "^pub " crates/node/src/wallet.rs

# Fix imports in commands/wallet.rs
use crate::wallet::{address, WalletKeypair, WalletAlgorithm, WalletPublicKey};
```

### Fix remaining import errors
```bash
# Compile and categorize errors
cargo build -p bitquan-node 2>&1 | grep "error\[" | sort -u

# Fix each category systematically:
# 1. Type not found → check module exports
# 2. Function not found → use crate::module::function
# 3. Method not found → check trait imports
```

### Remove duplicate function definitions
```bash
# After imports are fixed, remove local copies of moved functions
# Search for patterns like:
# - "^fn wallet_gen" (should be removed, using commands::wallet::wallet_gen)
# - "^pub fn run_rpc_server" (should be removed, using commands::rpc::run_rpc_server)
```

### Final cleanup
```bash
# Count remaining lines
wc -l crates/node/src/main.rs

# Target: < 500 lines (only CLI arg parsing, main(), integration)
```

## Patterns That Worked
1. **Parallel Agents**: Launch 5 agents for different domains simultaneously
2. **Module Structure**: commands/{wallet, rpc, node, mining, p2p}.rs
3. **Public Exports**: All moved functions must be `pub`
4. **Helper Functions**: Shared utilities in cli.rs with `crate::cli::` prefix
5. **Systematic Import Updates**: Use perl for bulk replacements:
   ```bash
   perl -i -pe 's/old/new/g' file.rs
   ```

## Anti-Patterns to Avoid
- ❌ **Moving functions without updating imports** - Causes 45+ compilation errors
- ❌ **Leaving duplicate definitions** - E0255 "defined multiple times" errors
- ❌ **Forgetting to make functions public** - Commands module can't access them
- ❌ **Hardcoded import paths** - Use `crate::` prefix for sibling modules

## Time Tracking
- **Session Start**: 06:21 (post-SSD failure recovery)
- **Agents Launched**: ~10 min into session
- **Rate Limit Hit**: 11:57 UTC reset wait
- **Manual Work**: Fixing imports, removing duplicates
- **Current State**: Module structure complete, fixing import errors

## Related Files
- `/Volumes/ACASIS Media/BitQuan/crates/node/src/main.rs` - Main file being refactored
- `/Volumes/ACASIS Media/BitQuan/crates/node/src/cli.rs` - Helper functions
- `/Volumes/ACASIS Media/BitQuan/crates/node/src/commands/mod.rs` - Module exports
- `/Volumes/ACASIS Media/BitQuan/crates/node/src/commands/*.rs` - Individual command modules

## Tags
`refactoring` `module-organization` `parallel-agents` `rust` `main.rs-split` `import-hell` `code-cleanup`
