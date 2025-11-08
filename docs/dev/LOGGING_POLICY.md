# Logging Security Policy

## Overview

This document defines security requirements for logging in BitQuan to prevent sensitive data leakage.

## ❌ NEVER Log These

### 1. Cryptographic Material
- ❌ Private keys (any format)
- ❌ Secret keys
- ❌ Mnemonic phrases / seed words
- ❌ Wallet passwords
- ❌ Backup passwords
- ❌ Keystore encryption keys

### 2. Authentication Credentials
- ❌ JWT tokens (full)
- ❌ RPC passwords
- ❌ API keys
- ❌ Session tokens

### 3. User Secrets
- ❌ Passphrases
- ❌ Recovery phrases
- ❌ PIN codes

## ✅ Safe to Log

### Identifiers (Non-sensitive)
- ✅ Public keys (addresses)
- ✅ Transaction IDs
- ✅ Block hashes
- ✅ Peer IDs
- ✅ Network addresses (IP:port)

### Metadata
- ✅ Timestamps
- ✅ Block heights
- ✅ Transaction counts
- ✅ Network statistics
- ✅ Version information

### Fingerprints
- ✅ Key fingerprints (first 8 bytes of hash)
- ✅ File checksums
- ✅ Derivation paths (without keys)

## Safe Logging Patterns

### 1. Use Fingerprints

```rust
// ❌ BAD
println!("Private key: {:?}", private_key);

// ✅ GOOD
use crate::logging::fingerprint;
println!("Key loaded: {}", fingerprint(&private_key));
```

### 2. Mask Secrets

```rust
// ❌ BAD
println!("Token: {}", jwt_token);

// ✅ GOOD
use crate::logging::mask_secret;
println!("Token: {}", mask_secret(&jwt_token, 4));
// Output: "eyJh...ture"
```

### 3. Sanitize User Input

```rust
// ❌ BAD - Log injection vulnerability
println!("Username: {}", username);

// ✅ GOOD
use crate::logging::sanitize_for_log;
println!("Username: {}", sanitize_for_log(&username));
```

### 4. Conditional Display

```rust
// Only show secrets when explicitly requested
if show_mnemonic {
    eprintln!("⚠️  SECURITY WARNING: Secret will be displayed!");
    println!("Mnemonic: {}", mnemonic);
} else {
    println!("Mnemonic generated (hidden for security)");
}
```

## Log Levels

### Production Defaults

- **Default**: `INFO`
- **Allowed**: `INFO`, `WARN`, `ERROR`
- **Forbidden**: `DEBUG`, `TRACE` (may leak internals)

### Development

- `DEBUG`: Allowed (but still no secrets)
- `TRACE`: Allowed (but still no secrets)

### Rules

```rust
// ✅ GOOD - No secrets at any level
log::debug!("Processing transaction {}", tx_id);
log::trace!("Block validation step: header_check");

// ❌ BAD - Never log secrets even in DEBUG
log::debug!("Private key: {:?}", key); // NEVER
```

## File Output

### Console Output (stdout/stderr)
- Use `println!` for user-facing messages
- Use `eprintln!` for warnings/errors
- Never redirect sensitive output to files

### Log Files
- If implemented, must:
  - Default to `INFO` level
  - Never contain secrets
  - Be rotated regularly
  - Have secure permissions (600)

## Error Messages

### User-Facing Errors

```rust
// ✅ GOOD - Generic error
Err(anyhow!("Authentication failed"))

// ❌ BAD - Leaks token
Err(anyhow!("Invalid token: {}", token))
```

### Debug Errors

```rust
// ✅ GOOD - Fingerprint
Err(anyhow!("Key validation failed: {}", fingerprint(key)))

// ❌ BAD - Full key
Err(anyhow!("Key validation failed: {:?}", key))
```

## Terminal Output Security

### Interactive Commands

```rust
// Mnemonic generation
if show_mnemonic {
    eprintln!("⚠️  SECURITY WARNING:");
    eprintln!("   - Do NOT log terminal output");
    eprintln!("   - Do NOT screenshot");
    println!("Mnemonic: {}", mnemonic);
}
```

### Password Input

```rust
// Use rpassword for secure input
use rpassword;
let password = rpassword::read_password()?;
// Never echo password to terminal
```

## Testing

### Test Logs

```rust
#[test]
fn test_key_generation() {
    let key = generate_key();
    // ✅ GOOD
    println!("Key fingerprint: {}", fingerprint(&key));
    
    // ❌ BAD
    println!("Key: {:?}", key);
}
```

### CI/CD

- Review CI logs for leaked secrets
- Use tools like `truffleHog` or `gitleaks`
- Never commit logs to repository

## Common Vulnerabilities

### 1. Debug Formatting

```rust
// ❌ BAD - Debug may expose internals
println!("Keystore: {:?}", keystore);

// ✅ GOOD - Manual formatting
println!("Keystore loaded: {} keys", keystore.count());
```

### 2. Log Injection

```rust
// ❌ BAD - Attacker can inject \n
println!("User: {}", untrusted_input);

// ✅ GOOD
println!("User: {}", sanitize_for_log(untrusted_input));
```

### 3. Panic Messages

```rust
// ❌ BAD - May leak via panic
.expect(&format!("Failed with key: {:?}", key))

// ✅ GOOD
.expect("Key operation failed")
```

## Audit Checklist

Before each release:

- [ ] Search codebase for `println!.*password`
- [ ] Search codebase for `println!.*secret`
- [ ] Search codebase for `println!.*private.*key`
- [ ] Search codebase for `println!.*mnemonic`
- [ ] Search codebase for `println!.*token`
- [ ] Review all `eprintln!` for sensitive data
- [ ] Check test output for secrets
- [ ] Verify production log level is INFO

## Tools

### Audit Script

```bash
#!/bin/bash
# audit-logs.sh

echo "Checking for secret leaks in logs..."

grep -rn "println!\|eprintln!" crates/ --include="*.rs" \
  | grep -i "password\|secret\|private\|mnemonic\|key\|token" \
  | grep -v "password:" \
  | grep -v "Enter password"

echo "Done. Review any findings above."
```

### Pre-commit Hook

```bash
#!/bin/bash
# .git/hooks/pre-commit

if git diff --cached --name-only | grep -q '\.rs$'; then
    ./scripts/audit-logs.sh
    if [ $? -ne 0 ]; then
        echo "⚠️  Potential secret leak detected!"
        echo "Review changes before committing."
        exit 1
    fi
fi
```

## References

- [OWASP Logging Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Logging_Cheat_Sheet.html)
- [CWE-532: Information Exposure Through Log Files](https://cwe.mitre.org/data/definitions/532.html)

---

**Policy Version**: 1.0  
**Last Updated**: 2025-11-02  
**Review Date**: Every major release
