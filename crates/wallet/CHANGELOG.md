# Wallet Crate Changelog

## [0.1.0] - 2024-11-01

### Added
- Secure keystore implementation using Argon2id + AES-256-GCM
- Magic header + version field for format validation (BQK1 v1)
- KDF profiles (Tight/Medium/Light/Mobile) for different platforms
- Password rotation via rotate_keystore() function
- Atomic file writes with Unix permissions (0600)
- Memory safety using zeroize and secrecy crates
- 9 comprehensive unit tests (all passing)

### Documentation
- README.md: Usage guide + KDF parameters
- SECURITY.md: Threat model + audit checklist
- INTEGRATION.md: CLI patterns + rate limiting examples

### Known Limitations
- Deprecation warnings from aes-gcm 0.10 (upstream, no security impact)
- Windows ACLs not enforced (user must use BitLocker/EFS)

### Dependencies
argon2 0.5, aes-gcm 0.10, rand 0.8, base64 0.22, serde 1.0, zeroize 1.8, secrecy 0.8
