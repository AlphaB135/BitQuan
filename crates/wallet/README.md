# BitQuan Wallet

Secure wallet implementation with encrypted keystore using Argon2id + AES-256-GCM.

## Features

- **Secure key derivation**: Argon2id with configurable memory/time costs
- **Strong encryption**: AES-256-GCM with authentication
- **Memory safety**: Uses `zeroize` and `secrecy` crates to protect sensitive data
- **Atomic writes**: Safe keystore file updates with proper permissions (0600 on Unix)

## Cryptographic Parameters

### Recommended Defaults (Production)
- **Memory cost**: 64 MiB (65536 KiB) - protects against GPU/ASIC attacks
- **Time cost**: 3 iterations
- **Parallelism**: 1 lane
- **Salt length**: 16 bytes (128 bits)
- **Nonce length**: 12 bytes (96 bits, AES-GCM standard)

### Rationale
- **Argon2id**: Hybrid algorithm resistant to both side-channel and GPU attacks
- **64 MiB memory**: Balance between security and usability on consumer hardware
- **Time cost = 3**: ~1-2 seconds on typical CPUs; adjust based on target platform
- **AES-256-GCM**: Authenticated encryption preventing tampering

### Lower-spec Devices
For embedded/mobile devices, reduce to:
- Memory: 32 MiB (32768 KiB)
- Time cost: 2

Test performance on target hardware before deployment.

## Usage

### Encrypt a private key
```rust
use wallet::keystore::{encrypt_keystore, DEFAULT_MEM_KIB, DEFAULT_TIME_COST, DEFAULT_PARALLELISM};
use serde_json::json;

let private_key = b"my-secret-private-key-bytes";
let password = "correct horse battery staple";
let metadata = Some(json!({"address": "0x123...", "hint": "main wallet"}));

let keystore = encrypt_keystore(
    private_key,
    password,
    metadata,
    DEFAULT_MEM_KIB,
    DEFAULT_TIME_COST,
    DEFAULT_PARALLELISM
);
```

### Decrypt keystore
```rust
use wallet::keystore::decrypt_keystore;

let plaintext = decrypt_keystore(&keystore, password)
    .expect("decryption failed");
```

### Save/Load from file
```rust
use wallet::keystore::{write_keystore_file_atomic, read_keystore_file};

// Save with atomic write + proper permissions
write_keystore_file_atomic("./data/keystore.json", &keystore)?;

// Load from file
let loaded = read_keystore_file("./data/keystore.json")?;
```

## Integration Checklist

### CLI Commands (TODO)
- [ ] `wallet create --password` - generate keypair + encrypt
- [ ] `wallet unlock --password --keystore <path>` - decrypt to memory
- [ ] `wallet change-pass --old --new` - re-encrypt with new password
- [ ] `wallet export --password` - export plaintext (warn user!)
- [ ] `wallet backup` - copy encrypted keystore

### Security Implementation
- [ ] **File permissions**: Enforce 0600 (owner read/write only) on Unix
- [ ] **Windows ACLs**: Restrict access to owner only
- [ ] **No logging**: Never log private keys or plaintext secrets
- [ ] **Memory protection**: Use `SecretVec`/`Zeroize` for all sensitive data
- [ ] **Rate limiting**: Implement exponential backoff after N failed unlock attempts
- [ ] **Atomic writes**: Always use `write_keystore_file_atomic`

### Migration/Rotation Plan
- [ ] Support legacy keystore format (if exists)
- [ ] Provide migration tool: old → new format
- [ ] Add passphrase rotation command (re-encrypt with updated KDF params)
- [ ] Version field in KeystoreFile for future format changes

### Testing
- [x] Unit tests: roundtrip encryption/decryption
- [x] Unit tests: wrong password rejection
- [x] Unit tests: atomic file write/read
- [ ] Fuzzing: malformed JSON keystore parsing
- [ ] Integration test: full CLI workflow
- [ ] Performance test: KDF timing on target hardware

### Documentation
- [ ] Security audit notes
- [ ] Backup/recovery instructions
- [ ] Key rotation procedure
- [ ] Incident response plan (compromised keystore)

### CI/CD
- [ ] Add `cargo test -p wallet` to GitHub Actions
- [ ] Optional: lighter test params for CI (8 MiB memory, time=1)
- [ ] Security scan with `cargo audit`
- [ ] Reproducible builds verification

## Security Notes

[WARNING] **Critical Practices**:
1. **Never commit keystore files to git** - add `*.keystore` and `keystore.json` to `.gitignore`
2. **Secure backups**: Store encrypted keystores offline (USB, paper backup of password)
3. **Password strength**: Use passphrase ≥ 5 words or password manager-generated 16+ chars
4. **Regular rotation**: Re-encrypt with updated KDF params annually using `rotate_keystore()`
5. **Audit trail**: Log unlock attempts (timestamps only, NEVER passwords)
6. **Windows users**: Store keystore in BitLocker-encrypted or EFS-protected folder (file ACLs not enforced by library)
7. **Memory safety**: Library uses `zeroize` + `secrecy` but integrate carefully - avoid logging/debugging decrypted keys
8. **Constant-time**: Password verification relies on AES-GCM tag check (constant-time by design)

## KDF Profiles

Use pre-configured profiles for different deployment scenarios:

| Profile  | Memory (KiB) | Time | Parallelism | Target Platform    |
|----------|--------------|------|-------------|--------------------|
| `Tight`  | 65536 (64MB) | 3    | 1           | Desktop/Server     |
| `Medium` | 32768 (32MB) | 3    | 1           | Laptop/VM          |
| `Light`  | 16384 (16MB) | 3    | 1           | CI/Low-spec        |
| `Mobile` | 8192 (8MB)   | 3    | 1           | Mobile/Embedded    |

```rust
use wallet::keystore::{KdfProfile, encrypt_keystore};

let (mem, time, par) = KdfProfile::Medium.params();
let ks = encrypt_keystore(secret, password, None, mem, time, par);
```

## Advanced Features

### Password Rotation
```rust
use wallet::keystore::rotate_keystore;

// Re-encrypt with new password and optionally stronger KDF params
let new_ks = rotate_keystore(&old_ks, "old-pass", "new-pass", 
                             65536, 3, 1)?;
write_keystore_file_atomic("keystore.json", &new_ks)?;
```

### Verification (without exposing plaintext)
```rust
// Verify keystore integrity + password correctness
match decrypt_keystore(&ks, password) {
    Ok(_) => println!("Valid keystore + password"),
    Err(e) => eprintln!("Verification failed: {}", e),
}
```

### Threat Model

**Attackers we protect against:**
- **Offline attacker with keystore file**: Argon2id (64MB memory, 3 iterations) makes brute-force ~1-2 seconds per guess on consumer CPU
- **Tampered ciphertext**: AES-GCM authentication tag prevents undetected modifications
- **Side-channel leaks**: `zeroize` clears sensitive memory; `secrecy` prevents accidental exposure

**Out of scope (user responsibility):**
- Weak passwords (recommend passphrase generator or password manager)
- Compromised password (no cryptography can save this)
- Malware on user's machine (use hardware wallet for high-value keys)
- Social engineering

## Dependencies

- `argon2` (0.5): Password hashing
- `aes-gcm` (0.10): Authenticated encryption
- `rand` (0.8): Cryptographically secure randomness
- `base64` (0.22): Encoding
- `serde` (1.0): Serialization
- `zeroize` (1.8): Memory clearing
- `secrecy` (0.8): Secret type wrappers

## License

Same as parent project (see repository root).
