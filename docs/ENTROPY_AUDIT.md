# BitQuan Entropy & RNG Audit

_Last updated: 2025-11-02_

This document summarises the current state of randomness usage across the BitQuan
codebase after the November 2025 hardening work (Task K).

---

## 1. RNG Sources in Production

| Area | File(s) | RNG Source | Notes |
|------|---------|------------|-------|
| PQC Dilithium wrapper | `crates/pqc-dilithium-seeded/src/randombytes.rs` | `rand::rngs::OsRng` | Used for Dilithium key generation; replaced `thread_rng()` with `OsRng` to ensure CSPRNG. |
| Wallet keystore & backup | `crates/wallet/src/keystore.rs`, `crates/wallet/src/backup.rs` | `OsRng` | Generates salts, nonces, and encryption material. |
| Wallet KDF | `crates/crypto/src/wallet/kdf.rs` | `OsRng` | Salt generation for Argon2id. |
| RNG service | `crates/crypto/src/rng/rng_impl.rs` | `OsRng` → `ChaCha20Rng` | Master seed sourced from OS CSPRNG, substreams derived via HKDF. |
| JWT auth & password flows | `crates/rpc/src/jwt/auth.rs`, `crates/node/src/main.rs` | `OsRng` | Salt generation for password hashing. |
| BIP39 mnemonic generation | `crates/node/src/mnemonic.rs` | `getrandom::getrandom` | Uses the OS CSPRNG directly. |

**Test-only & deterministic RNG usage**

- `StdRng::seed_from_u64` is used only inside `#[cfg(test)]` helpers for deterministic test vectors.
- `ChaCha20Rng::from_seed` is seeded either by `OsRng` or HKDF expansion; no manual/weak seeding remains.

---

## 2. Tests Covering RNG Behaviour

| Component | Tests | Purpose |
|-----------|-------|---------|
| Dilithium randombytes | `test_randombytes_produces_different_output`, `test_randombytes_not_all_zero`, `test_randombytes_not_all_same`, `test_randombytes_fills_correct_length` | Validate `OsRng` integration and ensure buffers are populated correctly. |
| Deterministic helper (tests only) | `test_deterministic_helper_same_seed_same_output`, `test_deterministic_helper_different_seed_different_output` | Guarantees deterministic vectors when required. |
| RNG service | `crates/crypto/src/rng/rng_impl.rs` tests (`not_all_zero`, `substream_differs`, proptests) | Ensures HKDF-derived streams differ and randomness is not degenerate. |

All above tests are part of the regular `cargo test` runs for their respective crates.

---

## 3. Remaining Actions

- Run `cargo test --all --locked` before release to reconfirm workspace integration.
- Keep this document in sync when new randomness-dependent modules are added (e.g., hardware wallet support, hybrid PoW research).

---

## 4. Audit Conclusion

1. **Primary RNG Source** – Every production path now relies on the OS CSPRNG (`OsRng` or `getrandom`).  
2. **Deterministic RNG** – Allowed only in test code or when seeded from CSPRNG + HKDF (e.g., `RngService`).  
3. **Documentation & Testing** – Unit tests assert randomness properties; audit log (this file) captures scope and status.  
4. **Risk Level** – With the current changes applied, entropy generation is considered **low risk** for release.

Any future changes touching randomness must update this audit and re-run relevant tests.

---
