# Dependency Security Update Log

**Date**: 2026-08-18  
**Auditor**: Hermes 🌸  
**Trigger**: 13 Dependabot security alerts

---

## Updates Applied

### ✅ openssl: 0.10.75 → 0.10.81
**Severity**: MEDIUM (8 CVEs fixed)  
**Vulnerabilities Fixed**:
- #39: Unchecked callback length in PSK/cookie trampolines
- #40: MdCtxRef::digest_final() buffer overflow
- #41: Incorrect bounds assertion in AES key wrap
- #42: Out-of-bounds read in PEM password callback
- #43: Deriver::derive buffer overflow on OpenSSL 1.1.1
- #46: Undefined behavior in X509Ref::ocsp_responders for non-UTF-8 URLs
- #48: Heap buffer overflow in AES key-wrap-with-padding
- #49: Out-of-bounds write in CipherCtxRef::cipher_update_inplace

**Impact**: Transitive dependency via `reqwest`. Not directly exploitable in BitQuan but fixed for defense-in-depth.

---

### ✅ time: 0.3.35 → 0.3.55
**Severity**: MEDIUM  
**Vulnerability Fixed**: Stack exhaustion DoS attack  
**Impact**: Transitive dependency via `rcgen`. Not exploitable (no user input to time parsing).

---

### ✅ rand: 0.9.2 → 0.9.5
**Severity**: LOW  
**Vulnerability Fixed**: Unsound with custom logger using rand::rng()  
**Impact**: Low risk (requires custom panic handler + specific API usage pattern).

---

## Dependent Package Updates

The following packages were updated automatically:
- `openssl-sys`: 0.9.115 → 0.9.117
- `deranged`: 0.3.11 → 0.5.8
- `num-conv`: 0.1.0 → 0.2.2
- `time-core`: 0.1.2 → 0.1.9
- `time-macros`: 0.2.18 → 0.2.32

**Total**: 8 packages updated

---

## Verification Status

### ✅ Cargo.lock Updated
```
openssl:    0.10.75 → 0.10.81 ✅
time:       0.3.35  → 0.3.55  ✅
rand (0.9): 0.9.2   → 0.9.5   ✅
```

### 🔄 Test Suite
**Status**: Running  
**Command**: `cargo test --workspace --lib`  
**Expected**: All tests pass (no regressions)

---

## Remaining Dependabot Alerts

### ✅ Already Fixed (3 alerts)
- `lru 0.12.5` → Using 0.16.3 (patched)
- `rand 0.8.6` → Already at patched version

### ⚠️ False Positive (1 alert)
- `quinn-proto` → Not in dependency tree
- **Action Required**: Dismiss alert #50 on GitHub

---

## Expected Security Posture After This Update

**Before**: 13 open Dependabot alerts  
**After**: 1 false positive remaining (quinn-proto)

**Risk Level**:
- Before: 🟡 LOW-MEDIUM
- After: 🟢 LOW (only false positive remains)

---

## Next Steps

1. ✅ Updates applied via `cargo update`
2. 🔄 Verify test suite passes
3. ⏳ Build release binary
4. ⏳ Dismiss quinn-proto false positive on GitHub
5. ⏳ Commit Cargo.lock updates
6. ⏳ Push to GitHub

---

**Updated By**: Hermes 🌸  
**Status**: In Progress
