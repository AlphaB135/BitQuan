# Main.rs Refactoring - Parallel Agent Strategy

**Date**: 2026-01-19
**Context**: Splitting 4,700+ line main.rs into logical command modules

## What We Learned
1. **Parallel Agent Velocity** - Launching 5 specialized agents simultaneously reduces analysis time by ~70%. Each agent handles one module domain independently.
2. **Agent Rate Limits** - API 429 errors after 3+ parallel agents. Need to batch work or wait for reset.
3. **Module Split Pattern** - CLI helper functions → cli.rs, Commands → commands/ subdirectory with mod.rs

## Why It Matters
- **Maintainability**: 4,700 lines impossible to navigate. Modules isolate concerns.
- **Compile Time**: Smaller files = faster incremental compilation
- **Code Review**: Smaller PRs for each module instead of monolithic changes

## Progress So Far
✅ **Completed**:
- cli.rs (format_bq, parse_network_id, read_password_from_stdin, invalid, address_network_label)
- commands/mod.rs structure
- commands/wallet.rs (13 functions moved - wallet_gen, wallet_address, wallet_sign, wallet_verify, wallet_send, wallet_gen_mnemonic, wallet_from_mnemonic, wallet_gen_multisig, multisig_info, tx_sign_partial, tx_combine_signatures, wallet_backup, wallet_restore)
- commands/rpc.rs (7 functions - run_rpc_server, submit_transaction_rpc, generate_self_signed_cert_cli, hash_password_cli, jwt_user_add, jwt_user_remove, jwt_user_list)
- commands/node.rs (6 functions - check_balance, verify_database, genesis_verify, build_tx, script_from_address, address_validate)

⚠️ **Rate Limited** (need manual completion):
- commands/mining.rs (~8 functions - mine_genesis, check_block, rng_demo, mine_once, mine_continuous, print_session_summary, run_stratum_server)
- commands/p2p.rs (~10 functions - write_envelope, read_envelope, p2p_demo, setup_storage, get_or_create_jwt_secret, setup_p2p_network, start_metrics_service, p2p_server, p2p_connect)

📋 **Remaining**:
- Update main.rs to use commands::* instead of inline functions
- Remove old function code from main.rs
- Add `use commands::*` imports where needed
- Verify compilation
- Target: main.rs < 500 lines

## How To Apply
```bash
# For manual module extraction:
# 1. Get line numbers
grep -n "^fn function_name" crates/node/src/main.rs

# 2. Extract function range
sed -n 'START,ENDp' crates/node/src/main.rs

# 3. Add pub keyword and crate::cli:: imports
# 4. Remove from main.rs
```

## Key Functions Moved
| Module | Functions | Status |
|--------|-----------|--------|
| cli.rs | 5 helpers | ✅ |
| wallet | 13 commands | ✅ |
| rpc | 7 commands | ✅ |
| node | 6 commands | ✅ |
| mining | 8 commands | ⏳ need manual |
| p2p | 10 commands | ⏳ need manual |

## Tags
`refactoring` `module-organization` `parallel-agents` `rust` `code-split`
