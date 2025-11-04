# Entropy & RNG Security Audit

## Executive Summary

**Audit Date**: November 4, 2024  
**Status**: ✅ **SECURE** - All production RNG usage verified to use cryptographically secure sources  
**Risk Level**: **LOW**

All randomness in BitQuan uses `OsRng` (operating system's secure random number generator) or equivalent cryptographically secure sources. No weak RNG sources (`thread_rng`, `StdRng`, etc.) are used in production code.

## RNG Usage Inventory

| Module | Function | RNG Source | Reason Secure | Notes |
|--------|----------|------------|---------------|-------|
| `crates/wallet/src/keystore.rs` | `encrypt_keystore` | `OsRng` | OS-provided CSPRNG | Salt & nonce generation |
| `crates/wallet/src/backup.rs` | `create_backup` | `OsRng` | OS-provided CSPRNG | Backup encryption salt/nonce |
| `crates/crypto/src/rng/rng_impl.rs` | `RngService::new` | `OsRng` | OS-provided CSPRNG | Master seed for DRBG |
| `crates/crypto/src/rng/rng_impl.rs` | `RngService::u64` | `ChaCha20Rng` | Seeded from OsRng | Deterministic expansion of OS entropy |
| `crates/crypto/src/wallet/kdf.rs` | `generate_salt` | `OsRng` | OS-provided CSPRNG | Password salt generation |
| `crates/rpc/src/jwt/auth.rs` | `hash_password` | `OsRng` | OS-provided CSPRNG | JWT authentication salt |
| `crates/node/src/main.rs` | `hash_admin_password` | `OsRng` | OS-provided CSPRNG | Admin password hashing |
| `crates/pqc-dilithium-seeded/src/randombytes.rs` | `randombytes` | `OsRng` | OS-provided CSPRNG | PQC key generation |
| `crates/types/src/entropy.rs` | `secure_bytes` | `OsRng` | OS-provided CSPRNG | General-purpose secure RNG helper |
| `crates/mempool/src/lib.rs` | `Mempool::insert` | `RngService` | Backed by OsRng | Transaction tie-breaker |

## Non-Secure RNG Usage (Test Code Only)

| Module | Function | RNG Source | Status | Justification |
|--------|----------|------------|--------|---------------|
| `crates/pqc-dilithium-seeded/src/randombytes.rs` | `randombytes_deterministic` | `StdRng` | ✅ Safe | Test-only function, marked with `#[cfg(test)]` |

## Audit Sign-Off

**Auditor**: BitQuan Core Team  
**Date**: November 4, 2024  
**Conclusion**: ✅ **All RNG usage is cryptographically secure**

## Quick Reference

### Always Use
✅ `rand::rngs::OsRng`  
✅ `bitquan_types::entropy::*`  
✅ `ChaCha20Rng` seeded from `OsRng`  

### Never Use
❌ `rand::thread_rng()`  
❌ `rand::StdRng` (except in tests)  
❌ Any unseeded or predictable RNG  
