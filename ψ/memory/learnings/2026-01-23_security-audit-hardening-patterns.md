# Lesson Learned: Security Audit Hardening Patterns

**Date:** 2026-01-23
**Type:** Security Patterns
**Impact:** High - Prevents credential leaks and production vulnerabilities

---

## The Pattern

**Systematic Security Hardening** - Parallel scan → prioritize → fix systematically → verify → commit

---

## Discovery Context

Full security audit session using 3 tools:
- `security-scanner` agent - Found 4 credential issues
- `repo-auditor` agent - False positive on target/ size
- `audit.sh` - Built-in security checks

Fixed all 4 issues in ~30 minutes with systematic approach.

---

## Why This Matters

1. **Credential leaks are catastrophic** - Even test passwords can be misused
2. **Placeholders must be obvious** - "CHANGE_ME" gets overlooked, "MUST_REPLACE" doesn't
3. **Validation prevents runtime failures** - Catch bad config at load time, not during requests
4. **Environment variables > hardcoded** - Overrideable, documented, auditable

---

## The Anti-Pattern

```
❌ WRONG:
// Hardcoded password scattered in tests
let password = "secure_password_123";

// Generic placeholder that might be missed
secret = "CHANGE_THIS_SECRET_IN_PRODUCTION";

// Credentials in shell scripts
WALLET_PASS="test_password_123"

// Default passwords that look real
GRAFANA_ADMIN_PASSWORD=securepassword123
```

---

## The Correct Pattern

```
✅ CORRECT:

// 1. Test passwords - helper function with security docs
fn test_password(seed: &str) -> String {
    format!("test_pw_{}_for_unit_tests_only", seed)
}
// ⚠️ SECURITY NOTE: TEST-ONLY, NEVER used in production

// 2. JWT validation at load time
pub fn validate_secret(&self) -> Result<(), String> {
    const FORBIDDEN: &[&str] = &["MUST_REPLACE_...", "secret", "password"];
    if FORBIDDEN.contains(&self.secret.as_str()) {
        return Err("Placeholder detected".to_string());
    }
    if self.secret.len() < 32 {
        return Err("Too short".to_string());
    }
    Ok(())
}

// 3. Shell scripts - environment variables with warnings
# ⚠️ SECURITY WARNING: TESTING ONLY
WALLET_PASS="${TEST_WALLET_PASSWORD:-test_only_password_do_not_use_in_prod}"

// 4. Config templates - explicit placeholders
# ⛔ REPLACE WITH STRONG PASSWORD
GRAFANA_ADMIN_PASSWORD=MUST_REPLACE_WITH_STRONG_RANDOM_PASSWORD_MIN_16_CHARS
```

---

## Application Rules

1. **Test Credentials**: Always use helper functions with "test_only" in generated values
2. **JWT Secrets**: Validate at load time, reject placeholders, enforce minimum length
3. **Shell Scripts**: Move to environment variables, add warning headers
4. **Config Templates**: Use "MUST_REPLACE" prefix, minimum length hints

---

## Examples from BitQuan

### Test Password Fix
**File:** `crates/wallet/tests/backup_restore_tests.rs`

Before:
```rust
let password = "secure_password_123";
let password = "test_password";
```

After:
```rust
fn test_password(seed: &str) -> String {
    format!("test_pw_{}_for_unit_tests_only", seed)
}
let password = &test_password("roundtrip");
```

### JWT Validation
**File:** `crates/rpc/src/jwt/config.rs`

Added:
```rust
pub fn validate_secret(&self) -> Result<(), String> {
    const FORBIDDEN_SECRETS: &[&str] = &[
        "MUST_REPLACE_WITH_64_CHAR_HEX...",
        "CHANGE_THIS_SECRET_IN_PRODUCTION...",
        "secret", "password", "jwtsecret",
    ];
    // Check exact matches
    // Enforce minimum 32 bytes
}
```

Tests: 4 new tests (placeholder rejected, short rejected, valid accepted)

---

## Meta-Lesson

**Parallel audit execution** is force multiplier. Running security-scanner + repo-auditor simultaneously saved ~10 minutes vs sequential. Specialized agents each do deep analysis in their domain.

**False positive detection** is crucial. Repo-auditor claimed target/ was "in repo" but only measured disk size. Always verify with `git ls-files` before acting on auditor claims.

**User's systematic approach** (one fix → commit → verify) prevented cascading failures. Each fix was isolated and reversible.

---

## Related Patterns

- **"Code Over Issues"** (2026-01-23) - Verify code reality before fixing
- **Test Helper Pattern** (2026-01-23) - Centralize test data generation
- **Environment Variable Defaults** - `${VAR:-default}` with warnings

---

**Tags:** security, credentials, audit, validation, hardening, jwt, shell-scripts
