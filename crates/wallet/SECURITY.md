# Security Considerations

## Overview

This wallet implementation uses **Argon2id** for key derivation and **AES-256-GCM** for authenticated encryption. This document outlines the security model, known limitations, and deployment best practices.

## Cryptographic Design

### Key Derivation: Argon2id
- **Algorithm**: Argon2id (hybrid mode, resistant to side-channel + GPU attacks)
- **Default params**: 64 MiB memory, 3 iterations, parallelism=1
- **Salt**: 16 bytes (128 bits), randomly generated per keystore
- **Output**: 32-byte (256-bit) AES key

**Rationale**: Argon2id won the Password Hashing Competition (2015). The memory-hard design forces attackers to use expensive ASIC/GPU setups, with 64 MiB costing ~1-2 seconds per guess on modern CPUs.

### Encryption: AES-256-GCM
- **Algorithm**: AES-GCM (Galois/Counter Mode with 256-bit key)
- **Nonce**: 12 bytes (96 bits), randomly generated per encryption
- **Authentication**: 16-byte GCM tag prevents tampering
- **AAD**: Empty (can be extended for metadata authentication)

**Rationale**: AES-GCM provides both confidentiality and authenticity. The authentication tag ensures any modification to ciphertext is detected during decryption.

## Threat Model

### Protected Against
1. **Offline brute-force attacks**: Argon2id's memory cost makes each password guess expensive (~1-2s on consumer hardware)
2. **Ciphertext tampering**: GCM authentication tag rejects modified ciphertexts
3. **Memory dumps**: `zeroize` clears sensitive data (keys, plaintext) after use
4. **Accidental logging**: `secrecy` crate prevents `Debug` output of secrets
5. **Nonce reuse**: Fresh random nonce per encryption
6. **Side-channel timing**: Argon2id + AES-GCM are designed to be constant-time

### NOT Protected Against (User Responsibility)
1. **Weak passwords**: If password is "123456", no KDF can save you
   - **Mitigation**: Enforce minimum 16 characters or passphrase ≥ 5 words
2. **Compromised passwords**: Keylogger, phishing, shoulder-surfing
   - **Mitigation**: Use hardware wallet for high-value keys
3. **Malware on user's machine**: Keylogger can capture password during unlock
   - **Mitigation**: Air-gapped signing for critical transactions
4. **Physical theft + weak password**: Attacker with device + weak password can brute-force
   - **Mitigation**: Strong password + timely key rotation after device loss
5. **Social engineering**: Tricking user into revealing password
   - **Mitigation**: User education

## Implementation Notes

### Memory Safety
- **Zeroizing**: All key material (`key_bytes`) is zeroized via `Zeroize` trait after use
- **Secret wrappers**: Passwords stored in `SecretVec<u8>` (auto-zeroizes on drop)
- **No copies**: Avoid unnecessary copies of plaintext/keys in memory

### File Security
- **Unix permissions**: Keystore files written with `0600` (owner read/write only)
- **Windows**: **WARNING** - ACLs not enforced by library. Users MUST store keystores in BitLocker/EFS-encrypted folders
- **Atomic writes**: Temporary file + rename prevents corruption on crash

### Constant-Time Operations
- **Password verification**: No explicit password comparison; AES-GCM tag check is inherently constant-time
- **Argon2**: Constant-time by design (data-independent memory access)

### Rate Limiting (TODO - Application Layer)
This library does NOT implement rate limiting. Integrators should add:
- Exponential backoff after N failed unlock attempts (e.g., N=5 → 2^n seconds delay, capped at 60s)
- Persistent lockout counter (survive process restarts)
- Logging of failed attempts (timestamp only, NEVER log passwords)

## Known Limitations

### 1. Version Field
- Current version: `1`
- Future versions may change crypto (e.g., switch to Argon2 v0x14, ChaCha20-Poly1305)
- **Mitigation**: `decrypt_keystore()` checks version and rejects unsupported formats

### 2. Metadata Not Authenticated
- `meta` field (address, hint) is encrypted but NOT included in AAD (Additional Authenticated Data)
- **Impact**: Attacker with keystore can swap metadata (but cannot decrypt secrets)
- **Mitigation**: Future version could add metadata to AAD

### 3. Windows File Permissions
- Library does NOT enforce Windows ACLs (complex + no standard Rust crate)
- **Mitigation**: Documented requirement for BitLocker/EFS; warning printed at runtime

### 4. KDF Parameters Stored in Keystore
- Attacker knows exact KDF params (memory, time, parallelism)
- **Impact**: Attacker can optimize brute-force setup
- **Non-issue**: This is standard practice (params must be stored for decryption)

### 5. No Hardware Security Module (HSM) Support
- Keys exist in process memory during decryption
- **Mitigation**: For high-value keys, integrate with HSM/hardware wallet (outside scope of this library)

## Deployment Checklist

### Before Production
- [ ] Enforce minimum password strength (≥ 16 chars or passphrase)
- [ ] Add rate limiting + lockout (application layer)
- [ ] Test KDF performance on target hardware (ensure < 5s unlock time)
- [ ] Audit all code paths for accidental logging of secrets
- [ ] Add monitoring for failed unlock attempts (alert on anomalies)
- [ ] Document backup/recovery procedure for users
- [ ] Test atomic file writes under disk-full conditions
- [ ] Verify file permissions on target OS (Unix: 0600, Windows: warn user)

### Operational Security
- [ ] Store backup keystores offline (USB, paper backup of password)
- [ ] Rotate passwords annually (use `rotate_keystore()`)
- [ ] After device loss, rotate keys immediately
- [ ] Use separate keystores for different risk levels (hot wallet vs. cold storage)
- [ ] Run `cargo audit` regularly to check for vulnerable dependencies
- [ ] Monitor for upstream advisories (Argon2, AES-GCM crates)

### Incident Response
If keystore is compromised:
1. Assume password will be brute-forced within hours/days (depending on strength)
2. Immediately transfer funds to new wallet with fresh keystore
3. Rotate all associated credentials (API keys, etc.)
4. Investigate how keystore was accessed (malware, insider, etc.)

## Audit Status

**Current Status**: Not externally audited

**Self-Audit Checklist**:
- [x] Argon2id parameters reviewed (64 MiB reasonable for 2024 hardware)
- [x] AES-GCM nonce uniqueness (random 12-byte per encryption)
- [x] Zeroize coverage (all `key_bytes` zeroized)
- [x] No plaintext logging in library code
- [x] Atomic file writes (temp + rename)
- [x] Version/magic field validation
- [ ] Fuzzing keystore JSON parser (TODO)
- [ ] Third-party security audit (TODO)
- [ ] Side-channel analysis (Argon2/AES-GCM assumed safe)

## References

1. [Argon2 Specification](https://github.com/P-H-C/phc-winner-argon2/blob/master/argon2-specs.pdf)
2. [NIST SP 800-38D: GCM Mode](https://csrc.nist.gov/publications/detail/sp/800-38d/final)
3. [OWASP Password Storage Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Password_Storage_Cheat_Sheet.html)
4. [Zeroize Crate Documentation](https://docs.rs/zeroize/)
5. [RustCrypto: AES-GCM](https://github.com/RustCrypto/AEADs/tree/master/aes-gcm)

## Contact

For security issues, please email: [INSERT SECURITY CONTACT]

Do NOT open public GitHub issues for vulnerabilities. Use responsible disclosure.
