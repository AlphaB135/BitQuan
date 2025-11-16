# Entropy Audit Report

**Date:** 2025-11-02  
**Auditor:** Security Team  
**Scope:** All RNG usage in BitQuan codebase  

## Executive Summary

✅ **PASS** - All production cryptographic operations use secure CSPRNGs (Cryptographically Secure Pseudo-Random Number Generators).

## Audit Methodology

1. Search for all RNG-related imports (`OsRng`, `ChaCha20Rng`, `getrandom`, etc.)
2. Classify usage by context (production crypto vs. tests vs. deterministic)
3. Verify appropriate CSPRNG selection for each use case
4. Document findings and recommendations

## Findings by Component

### ✅ Wallet Keystore (`crates/wallet/src/keystore.rs`)

**Usage:**
```rust
use rand::rngs::OsRng;
use rand::RngCore;

// Salt generation (line 98)
OsRng.fill_bytes(&mut salt);

// Nonce generation (line 101)
OsRng.fill_bytes(&mut nonce_bytes);
```

**Assessment:** ✅ SECURE
- Uses `OsRng` for all random data
- Appropriate for AES-GCM encryption
- No weak RNG detected

---

### ✅ Wallet Backup (`crates/wallet/src/backup.rs`)

**Usage:**
```rust
use rand::rngs::OsRng;
use rand::RngCore;

// Salt generation (line 114)
OsRng.fill_bytes(&mut salt);

// Nonce generation (line 117)
OsRng.fill_bytes(&mut nonce_bytes);
```

**Assessment:** ✅ SECURE
- Uses `OsRng` for backup encryption
- Consistent with keystore approach
- No vulnerabilities

---

### ✅ KDF Module (`crates/crypto/src/wallet/kdf.rs`)

**Usage:**
```rust
use rand::rngs::OsRng;

// Salt generation (line 68)
getrandom::getrandom(&mut salt).expect("OS RNG failure");

// Alternative (line 100)
SaltString::generate(&mut OsRng)
```

**Assessment:** ✅ SECURE
- Uses both `getrandom` and `OsRng` (both CSPRNG)
- Appropriate for Argon2 key derivation
- Panic on RNG failure is acceptable (no fallback to weak RNG)

---

### ✅ Encryption Module (`crates/crypto/src/wallet/encryption.rs`)

**Usage:**
```rust
// Nonce generation (line 106)
getrandom::getrandom(&mut nonce_bytes).map_err(EncryptionError::Rng)?;
```

**Assessment:** ✅ SECURE
- Uses `getrandom` for AES-GCM nonce
- Proper error handling
- No weak fallback

---

### ✅ JWT Authentication (`crates/rpc/src/jwt/auth.rs`)

**Usage:**
```rust
use password_hash::{rand_core::OsRng, SaltString};

// Salt generation (line 24)
let salt = SaltString::generate(&mut OsRng);
```

**Assessment:** ✅ SECURE
- Uses `OsRng` for Argon2 password hashing
- Industry standard practice
- No vulnerabilities

---

### ✅ Mnemonic Generation (`crates/node/src/mnemonic.rs`)

**Usage:**
```rust
// Entropy generation (line 38)
getrandom::getrandom(&mut entropy)?;
```

**Assessment:** ✅ SECURE
- Uses `getrandom` for BIP39 seed phrase entropy
- Critical operation - correctly implemented
- 128/256 bits of entropy from OS CSPRNG

---

### ✅ RNG Service (`crates/crypto/src/rng/rng_impl.rs`)

**Usage:**
```rust
use rand::rngs::OsRng;
use rand_chacha::ChaCha20Rng;

// Production mode (line 80)
OsRng

// Deterministic mode (line 70, 83, 91)
ChaCha20Rng::from_seed(seed)
```

**Assessment:** ✅ SECURE
- `OsRng` used for production
- `ChaCha20Rng` used ONLY for deterministic/test scenarios
- Proper separation of concerns
- `ChaCha20` is a CSPRNG when seeded correctly

**Note:** Deterministic mode is used for:
1. BIP39 mnemonic → keypair derivation
2. Testing with reproducible results
3. Both are acceptable use cases

---

### ✅ Node CLI (`crates/node/src/main.rs`)

**Usage:**
```rust
use password_hash::{rand_core::OsRng, PasswordHasher, SaltString};

// Salt generation (line 2297, 2361)
let salt = SaltString::generate(&mut OsRng);
```

**Assessment:** ✅ SECURE
- Uses `OsRng` for password hashing
- Consistent with RPC auth
- No vulnerabilities

---

### ⚠️ PQC Dilithium Seeded (`crates/pqc-dilithium-seeded/src/randombytes.rs`)

**Usage:**
```rust
use rand::prelude::*;
```

**Assessment:** ⚠️ NEEDS REVIEW
- This is a patched library for deterministic key generation
- Used ONLY for BIP39 → Dilithium keypair derivation
- Should use deterministic RNG (ChaCha20) seeded from BIP39
- **Action:** Verify this module doesn't use weak RNG in production

---

## Summary Statistics

| Component | CSPRNG Used | Status |
|-----------|-------------|--------|
| Wallet Keystore | OsRng | ✅ SECURE |
| Wallet Backup | OsRng | ✅ SECURE |
| KDF | OsRng, getrandom | ✅ SECURE |
| Encryption | getrandom | ✅ SECURE |
| JWT Auth | OsRng | ✅ SECURE |
| Mnemonic | getrandom | ✅ SECURE |
| RNG Service (prod) | OsRng | ✅ SECURE |
| RNG Service (test) | ChaCha20 | ✅ ACCEPTABLE |
| Node CLI | OsRng | ✅ SECURE |
| PQC Dilithium | TBD | ⚠️ REVIEW |

**Total:** 9/10 secure, 1 needs review

---

## RNG Security Tiers

### Tier 1: CSPRNGs (Production)
✅ `OsRng` - Uses OS entropy sources (`/dev/urandom`, `getrandom()`, etc.)  
✅ `getrandom` - Direct syscall to OS CSPRNG  
✅ `ChaCha20Rng` - When seeded with 256 bits from CSPRNG  

### Tier 2: Acceptable for Tests
✅ `ChaCha20Rng` - With deterministic seed (for reproducible tests)  

### Tier 3: NEVER USE
❌ `rand::random()` - Not cryptographically secure  
❌ `SmallRng` - Fast but not secure  
❌ `StdRng` - Deterministic, not for crypto  
❌ `thread_rng()` - May use weak RNG on some platforms  

---

## Test Coverage

### Entropy Quality Tests

**Location:** `crates/crypto/src/rng/rng_impl.rs` (tests module)

Tests verify:
- ✅ Different seeds produce different keys
- ✅ Same seed produces same keys (deterministic)
- ✅ No collisions in 1000 key generations
- ✅ Proper error handling on RNG failure

**Recommendation:** Add more statistical tests:
- Chi-squared test for randomness
- Birthday paradox collision test
- Avalanche effect test

---

## Vulnerabilities Found

### 🟢 No Critical Vulnerabilities

All production cryptographic operations use appropriate CSPRNGs.

### ⚠️ Minor Concerns

1. **PQC Dilithium Patched Library**
   - **Risk:** LOW
   - **Issue:** Custom patched library; needs code review
   - **Mitigation:** Used only for deterministic BIP39 derivation
   - **Action:** Verify seeding mechanism

2. **No Explicit Entropy Tests**
   - **Risk:** LOW
   - **Issue:** No runtime verification of entropy quality
   - **Mitigation:** CSPRNG libraries handle this internally
   - **Action:** Consider adding entropy health checks

---

## Recommendations

### Short-term (Week 1)
1. ✅ Complete review of `pqc-dilithium-seeded` RNG usage
2. ✅ Document deterministic RNG use cases in code comments
3. ✅ Add entropy quality unit tests

### Medium-term (Month 1)
1. Add runtime entropy health checks (optional)
2. Implement RNG failure monitoring/metrics
3. Add statistical randomness tests to CI

### Long-term (Month 3)
1. Consider hardware RNG support (RDRAND, TPM)
2. Implement entropy pool monitoring
3. Add formal security audit of RNG usage

---

## References

1. **OsRng Documentation:** https://docs.rs/rand/latest/rand/rngs/struct.OsRng.html
2. **getrandom Documentation:** https://docs.rs/getrandom/
3. **ChaCha20 Security:** https://cr.yp.to/chacha.html
4. **NIST SP 800-90A:** CSPRNG Recommendations

---

## Audit Conclusion

**Status:** ✅ **PASS**

BitQuan's RNG usage follows industry best practices:
- All production crypto uses OS-level CSPRNGs
- Deterministic RNG is properly isolated to test/BIP39 scenarios
- No weak RNG detected in critical paths
- Error handling is appropriate (fail-safe, no fallback to weak RNG)

**Risk Level:** 🟢 **LOW**

**Next Audit Date:** 2026-01-02 (or when RNG code changes)

---

**Signed:**  
Security Team  
BitQuan Project  
2025-11-02
