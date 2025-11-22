# Integration Guide

## Quick Start

### 1. Add to your project
```toml
[dependencies]
wallet = { path = "../wallet" }
```

### 2. Create encrypted keystore
```rust
use wallet::keystore::{
    encrypt_keystore, write_keystore_file_atomic,
    DEFAULT_MEM_KIB, DEFAULT_TIME_COST, DEFAULT_PARALLELISM
};
use serde_json::json;

// Your private key bytes (e.g., 32-byte ed25519 key)
let private_key = b"your-32-byte-private-key-here...";
let password = "correct horse battery staple";  // Use strong passphrase!

// Optional metadata (NOT authenticated, only encrypted)
let metadata = Some(json!({
    "address": "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb",
    "created_at": "2024-01-15T10:30:00Z",
    "hint": "Main wallet"
}));

// Encrypt
let keystore = encrypt_keystore(
    private_key,
    password,
    metadata,
    DEFAULT_MEM_KIB,
    DEFAULT_TIME_COST,
    DEFAULT_PARALLELISM
);

// Save to file (atomic write + 0600 permissions on Unix)
write_keystore_file_atomic("./data/keystore.json", &keystore)?;
```

### 3. Unlock wallet
```rust
use wallet::keystore::{read_keystore_file, decrypt_keystore};
use secrecy::{SecretVec, ExposeSecret};

// Load keystore
let keystore = read_keystore_file("./data/keystore.json")?;

// Decrypt (this is slow by design - ~1-2s with default params)
let plaintext = decrypt_keystore(&keystore, password)
    .map_err(|e| format!("Unlock failed: {}", e))?;

// Wrap in SecretVec for auto-zeroize
let private_key = SecretVec::new(plaintext);

// Use the key...
sign_transaction(private_key.expose_secret());

// Key is automatically zeroized when `private_key` goes out of scope
```

## CLI Integration Pattern

### Secure Password Input
```rust
#[cfg(feature = "cli")]
use rpassword::prompt_password;

fn unlock_wallet_interactive(keystore_path: &str) -> Result<SecretVec<u8>, String> {
    let keystore = read_keystore_file(keystore_path)
        .map_err(|e| format!("Failed to read keystore: {}", e))?;

    // Prompt without echoing to terminal
    let password = prompt_password("Enter password: ")
        .map_err(|e| format!("Password input failed: {}", e))?;

    // Decrypt
    let plaintext = decrypt_keystore(&keystore, &password)?;

    // Wrap in SecretVec
    Ok(SecretVec::new(plaintext))
}
```

### Password Change
```rust
use wallet::keystore::{rotate_keystore, KdfProfile};

fn change_password(keystore_path: &str) -> Result<(), String> {
    let keystore = read_keystore_file(keystore_path)?;

    let old_password = prompt_password("Current password: ")?;
    let new_password = prompt_password("New password: ")?;
    let confirm = prompt_password("Confirm new password: ")?;

    if new_password != confirm {
        return Err("Passwords don't match".to_string());
    }

    // Optionally upgrade KDF params
    let (mem, time, par) = KdfProfile::Tight.params();
    let new_keystore = rotate_keystore(&keystore, &old_password, &new_password, mem, time, par)?;

    // Atomic replace
    write_keystore_file_atomic(keystore_path, &new_keystore)?;

    println!("Password changed successfully");
    Ok(())
}
```

## Rate Limiting (Recommended)

The library does NOT implement rate limiting. Add application-layer protection:

```rust
use std::time::{Duration, Instant};
use std::collections::HashMap;
use std::sync::Mutex;

struct RateLimiter {
    attempts: Mutex<HashMap<String, (u32, Instant)>>,
    max_attempts: u32,
    lockout_base_secs: u64,
}

impl RateLimiter {
    fn new(max_attempts: u32) -> Self {
        Self {
            attempts: Mutex::new(HashMap::new()),
            max_attempts,
            lockout_base_secs: 2,
        }
    }

    fn check_and_increment(&self, keystore_path: &str) -> Result<(), String> {
        let mut map = self.attempts.lock().unwrap();
        let entry = map.entry(keystore_path.to_string()).or_insert((0, Instant::now()));

        if entry.0 >= self.max_attempts {
            let lockout_duration = Duration::from_secs(
                self.lockout_base_secs.pow(entry.0.saturating_sub(self.max_attempts))
                    .min(60)  // Cap at 60 seconds
            );

            if entry.1.elapsed() < lockout_duration {
                return Err(format!("Too many failed attempts. Try again in {} seconds",
                    lockout_duration.as_secs()));
            } else {
                // Reset after lockout period
                entry.0 = 0;
            }
        }

        entry.0 += 1;
        entry.1 = Instant::now();
        Ok(())
    }

    fn reset(&self, keystore_path: &str) {
        self.attempts.lock().unwrap().remove(keystore_path);
    }
}

// Usage
static RATE_LIMITER: Mutex<Option<RateLimiter>> = Mutex::new(None);

fn unlock_with_rate_limit(keystore_path: &str, password: &str) -> Result<Vec<u8>, String> {
    let limiter = RATE_LIMITER.lock().unwrap()
        .get_or_insert_with(|| RateLimiter::new(5));

    limiter.check_and_increment(keystore_path)?;

    let keystore = read_keystore_file(keystore_path)?;
    match decrypt_keystore(&keystore, password) {
        Ok(plaintext) => {
            limiter.reset(keystore_path);
            Ok(plaintext)
        }
        Err(e) => Err(format!("Unlock failed: {}", e))
    }
}
```

## KDF Profile Selection

Choose based on deployment target:

```rust
use wallet::keystore::KdfProfile;

fn get_kdf_params(profile_name: &str) -> (u32, u32, u8) {
    match profile_name {
        "tight" => KdfProfile::Tight.params(),      // 64 MB, desktop/server
        "medium" => KdfProfile::Medium.params(),    // 32 MB, laptop
        "light" => KdfProfile::Light.params(),      // 16 MB, VM/CI
        "mobile" => KdfProfile::Mobile.params(),    // 8 MB, mobile
        _ => KdfProfile::Tight.params(),
    }
}

// CLI arg: --kdf-profile tight
let (mem, time, par) = get_kdf_params(&args.kdf_profile);
let keystore = encrypt_keystore(secret, password, metadata, mem, time, par);
```

## Backup & Recovery

### Export encrypted backup
```rust
use std::fs;

fn backup_keystore(src: &str, dest: &str) -> std::io::Result<()> {
    // Simple copy (keystore is already encrypted)
    fs::copy(src, dest)?;

    // Optionally verify
    let original = read_keystore_file(src)?;
    let backup = read_keystore_file(dest)?;
    assert_eq!(original.ciphertext_b64, backup.ciphertext_b64);

    println!("Backup created: {}", dest);
    println!("Store this file AND your password in separate secure locations!");
    Ok(())
}
```

### Verify backup integrity
```rust
fn verify_backup(path: &str, password: &str) -> Result<(), String> {
    let keystore = read_keystore_file(path)
        .map_err(|e| format!("Cannot read file: {}", e))?;

    // Check magic + version
    if keystore.magic != "BQK1" {
        return Err("Invalid keystore format".to_string());
    }

    // Decrypt (but don't expose plaintext)
    decrypt_keystore(&keystore, password)?;

    println!("✓ Backup is valid and password is correct");
    Ok(())
}
```

## Testing Your Integration

```rust
#[cfg(test)]
mod integration_tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn full_lifecycle() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.keystore");

        // Create
        let secret = b"test-private-key-32-bytes!!!";
        let password = "test-password-123";
        let ks = encrypt_keystore(secret, password, None, 8*1024, 1, 1);
        write_keystore_file_atomic(&path, &ks).unwrap();

        // Unlock
        let loaded = read_keystore_file(&path).unwrap();
        let decrypted = decrypt_keystore(&loaded, password).unwrap();
        assert_eq!(decrypted, secret);

        // Change password
        let new_ks = rotate_keystore(&loaded, password, "new-pass", 8*1024, 1, 1).unwrap();
        write_keystore_file_atomic(&path, &new_ks).unwrap();

        // Verify new password works
        let final_ks = read_keystore_file(&path).unwrap();
        let final_pt = decrypt_keystore(&final_ks, "new-pass").unwrap();
        assert_eq!(final_pt, secret);

        // Old password fails
        assert!(decrypt_keystore(&final_ks, password).is_err());
    }
}
```

## Production Checklist

- [ ] Password strength enforced (≥ 16 chars or passphrase)
- [ ] Rate limiting implemented (5 attempts → exponential backoff)
- [ ] Keystore files added to `.gitignore`
- [ ] File permissions verified (Unix: 0600, Windows: encrypted folder)
- [ ] Backup procedure documented for users
- [ ] KDF params tested on target hardware (unlock time < 5s)
- [ ] No secrets logged (audit all logging code)
- [ ] Memory safety verified (SecretVec used for keys)
- [ ] Error messages don't leak sensitive info
- [ ] Recovery procedure tested (restore from backup)

## Known Issues

### Deprecation Warnings (aes-gcm)
Current version shows warnings about `generic-array 0.14` deprecation:
```
warning: use of deprecated associated function `GenericArray::<T, N>::from_slice`
```

**Status**: Upstream `aes-gcm` 0.10.x depends on `generic-array 0.14`. The deprecation is harmless (functionality identical). Will be resolved when `aes-gcm` updates to 1.0.

**Workaround**: Suppress with `#[allow(deprecated)]` if needed, or wait for upstream update.

### Windows ACLs
Library does NOT enforce Windows file ACLs (complex, no standard crate).

**Mitigation**: Documentation warns users to store keystores in BitLocker/EFS-encrypted folders. Runtime warning printed on Windows.

## Support

For integration issues:
1. Check README.md for basic usage
2. Check SECURITY.md for threat model
3. Review tests in `src/keystore.rs` for examples
4. Open GitHub issue with `[integration]` tag

Do NOT post keystores or passwords in issues (even test ones)!
