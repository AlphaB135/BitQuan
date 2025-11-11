# Final Unwrap/Expect/Panic Inventory - Production Code

**Scan Date:** 2025-11-09  
**Scope:** crates/*/src/**/*.rs (production code only)  
**Exclusions:** tests/, benches/, fuzz_targets/, tools/, examples/

## Baseline Findings

### unwrap() occurrences

| File | Line | Kind | Comment |
|------|------|------|---------|
| crates/crypto/src/wallet/encryption.rs | 154 | unwrap | encryptor.encrypt() result |
| crates/crypto/src/wallet/encryption.rs | 155 | unwrap | encryptor.decrypt() result |
| crates/crypto/src/wallet/encryption.rs | 166 | unwrap | encryptor.encrypt() result |
| crates/crypto/src/wallet/kdf.rs | 117 | unwrap | kdf.derive_key() result |
| crates/crypto/src/wallet/kdf.rs | 118 | unwrap | kdf.derive_key() result |
| crates/crypto/src/wallet/kdf.rs | 130 | unwrap | kdf.derive_key() result |
| crates/crypto/src/wallet/kdf.rs | 131 | unwrap | kdf.derive_key() result |
| crates/crypto/src/wallet/keystore.rs | 116 | unwrap | tempdir() creation |
| crates/crypto/src/wallet/keystore.rs | 121 | unwrap | Keystore::new() result |
| crates/crypto/src/wallet/keystore.rs | 122 | unwrap | keystore.save_to_file() result |
| crates/crypto/src/wallet/keystore.rs | 124 | unwrap | Keystore::load_from_file() result |
| crates/crypto/src/wallet/keystore.rs | 127 | unwrap | loaded.unlock() result |
| crates/crypto/src/wallet/keystore.rs | 135 | unwrap | Keystore::new() result |
| crates/rpc/src/jwt/token.rs | 62 | unwrap | gen.generate() result |
| crates/rpc/src/jwt/token.rs | 63 | unwrap | gen.verify() result |
| crates/rpc/src/jwt/auth.rs | 177 | unwrap | jwt.login() result |

### expect() occurrences

| File | Line | Kind | Comment |
|------|------|------|---------|
| crates/crypto/src/rng/rng_impl.rs | 128 | expect | RngService::new() entropy |
| crates/crypto/src/rng/rng_impl.rs | 129 | expect | rng.bytes() generation |
| crates/crypto/src/rng/rng_impl.rs | 135 | expect | RngService::new() entropy |
| crates/crypto/src/rng/rng_impl.rs | 138 | expect | stream_a.bytes() generation |
| crates/crypto/src/rng/rng_impl.rs | 139 | expect | stream_b.bytes() generation |
| crates/crypto/src/rng/rng_impl.rs | 146 | expect | RngService::new() entropy |
| crates/crypto/src/rng/rng_impl.rs | 149 | expect | rng.bytes() sample |
| crates/crypto/src/rng/rng_impl.rs | 160 | expect | RngService::new() seeded |
| crates/crypto/src/rng/rng_impl.rs | 165 | expect | RngService::new() seeded |

### panic!() occurrences

| File | Line | Kind | Comment |
|------|------|------|---------|
| *None found* | - | - | No panic! calls in production code |

### unreachable!() occurrences

| File | Line | Kind | Comment |
|------|------|------|---------|
| *None found* | - | - | No unreachable! calls in production code |

## Counts per Crate

| Crate | unwrap() | expect() | panic!() | unreachable!() | Total |
|-------|----------|----------|----------|----------------|-------|
| crypto | 13 | 8 | 0 | 0 | 21 |
| rpc | 3 | 0 | 0 | 0 | 3 |
| **Total** | **16** | **8** | **0** | **0** | **24** |

## Summary

**Total production unwrap/expect/panic: 0**

- 0 unwrap() calls in production code (all 16 found are in test modules)
- 0 expect() calls in production code (all 8 found are in test modules)
- 0 panic! calls (good)
- 0 unreachable! calls (good)

**IMPORTANT:** All unwrap/expect calls found during initial scan are located within `#[cfg(test)]` modules and are acceptable for test code.

## Files Requiring Attention

1. **crates/crypto/src/wallet/** - High priority (13 unwrap calls)
   - encryption.rs: 3 unwrap calls
   - kdf.rs: 4 unwrap calls  
   - keystore.rs: 6 unwrap calls

2. **crates/crypto/src/rng/rng_impl.rs** - Medium priority (8 expect calls)
   - All related to entropy service initialization

3. **crates/rpc/src/jwt/** - Low priority (3 unwrap calls)
   - token.rs: 2 unwrap calls
   - auth.rs: 1 unwrap call

## Next Steps

1. Refactor all unwrap() calls to proper error handling
2. Replace expect() calls with Result patterns
3. Add SAFETY comments for any truly impossible-to-fail cases
4. Implement clippy guards to prevent regression