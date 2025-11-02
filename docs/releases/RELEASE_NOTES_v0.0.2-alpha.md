# BitQuan v0.0.2-alpha Release Notes

**Release Date:** November 2, 2025  
**Status:** Alpha (Devnet Ready)

## Critical Security Updates

This release focuses exclusively on security hardening. Three major vulnerability classes have been addressed:

### 1. Integer Overflow Protection
Arithmetic operations involved in transaction validation, fee calculation, and block assembly now use checked arithmetic. Overflow/underflow conditions surface explicit errors instead of wrapping silently.

### 2. Replay Attack Prevention
Transaction signatures are now bound to a `TxContext { network_id, genesis_hash }` and a domain separator (`BitQuanSigHashV1`). Cross-network and cross-fork replay attacks are prevented by design.

### 3. Entropy Security
All randomness used in key generation, encryption, and authentication is sourced from the operating system CSPRNG (`OsRng`/`getrandom`). Test-only deterministic RNG helpers remain gated behind `#[cfg(test)]`.

## Breaking Changes

**Developers:**
- Update calls to `transaction_sighash()` and `validate_block()` to pass a `TxContext`.
- Recalculate or regenerate any persisted signatures and golden vectors (hash domain changed).
- RPC helpers now use `JwtConfig::default()` via the standard `Default` trait.

**Node Operators:**
- Recompile the node and restart. No configuration changes are required if you rely on default RPC settings.
- Existing wallet/RPC JWT secrets remain valid, but new defaults (or config files) follow the hardened schema.

## Testing Summary

- 320+ tests passing across the workspace (`cargo test --all --locked`)
- 44 new security-focused tests (overflow, replay, entropy)
- `cargo fmt` and `cargo clippy --all-targets --all-features -- -D warnings` are clean

## Next Steps Before Mainnet

- Commission an external security audit (Trail of Bits, Cure53, Zellic, etc.)
- Run an extended public testnet (3–6 months) to gather operational feedback
- Launch a bug bounty programme ($5K–$50K pool) targeting replay/overflow/entropy regressions

## Upgrading

```bash
git pull origin main
cargo build --release --locked
cargo test --all --locked
```

## Support

- Issues: <https://github.com/AlphaB135/BitQuan/issues>
- Security: `security@bitquan.org` (PGP key in `SECURITY.md`)
- Discussions: <https://github.com/AlphaB135/BitQuan/discussions>

## Acknowledgements

Security hardening made possible with AI assistance:

- Claude (Anthropic) – architecture review & test design  
- Cursor – refactoring and diagnostics  
- Codex – codebase analysis & implementation support

Solo developer: Atsadawut Khunthong
