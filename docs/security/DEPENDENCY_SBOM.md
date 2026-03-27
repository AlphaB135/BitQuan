# Software Bill of Materials (SBOM)

**Generated**: 2026-03-27
**Format**: SPDX 2.3 compatible
**Total dependencies**: 270 (540 entries in lockfile, many are duplicate versions)

---

## Direct Dependencies (Workspace)

| Crate | Version | License | Purpose |
|-------|---------|---------|---------|
| `tokio` | 1.x | MIT | Async runtime |
| `serde` / `serde_json` | 1.x | MIT/Apache-2.0 | Serialization |
| `sha2` | 0.10.x | MIT/Apache-2.0 | SHA-256 hashing |
| `blake3` | 1.x | Apache-2.0 | Address hashing |
| `argon2` | 0.5.x | MIT/Apache-2.0 | Key derivation (Argon2id) |
| `aes-gcm` | 0.10.x | Apache-2.0 | Symmetric encryption |
| `bincode` | 1.3.3 | MIT | Binary serialization |
| `secp256k1` | 0.29.x | Apache-2.0 | ECDSA (legacy compat) |
| `bip39` | 2.x | MIT | Mnemonic generation |
| `prometheus` | 0.13.x | Apache-2.0 | Metrics |
| `ratatui` | 0.29.x | MIT | TUI framework |
| `reqwest` | 0.12.x | MIT/Apache-2.0 | HTTP client |
| `jsonwebtoken` | 9.x | MIT | JWT authentication |
| `chacha20poly1305` | 0.10.x | Apache-2.0 | AEAD cipher |
| `ed25519-dalek` | 2.x | Apache-2.0 | Ed25519 signatures |
| `rayon` | 1.x | MIT/Apache-2.0 | Parallel processing |
| `tracing` | 0.1.x | MIT | Structured logging |
| `lazy_static` | 1.x | MIT/Apache-2.0 | Static initialization |
| `uuid` | 1.x | Apache-2.0 | Unique identifiers |
| `rand` / `rand_core` | 0.8.x | MIT/Apache-2.0 | Random number generation |

## Post-Quantum Cryptography

| Crate | Version | License | Purpose |
|-------|---------|---------|---------|
| `pqc-dilithium-seeded` | local | Apache-2.0 | CRYSTALS-Dilithium3 signatures |

## Known Vulnerability Exceptions

| Advisory | Crate | Reason | Risk |
|----------|-------|--------|------|
| RUSTSEC-2025-0141 | bincode 1.3.3 | Unmaintained, pinned for format stability | Low (controlled input) |
| RUSTSEC-2024-0437 | protobuf 2.28.0 | Transitive via prometheus | Low (internal use) |
| RUSTSEC-2023-0071 | rsa | Transitive via jsonwebtoken, local JWT only | Low (not network-exposed) |
| RUSTSEC-2026-0009 | time | Transitive via rcgen, no user RFC 2822 input | Low (no external parsing) |
| RUSTSEC-2024-0436 | paste | Transitive via ratatui, no upgrade path | Low (UI only) |

Full ignore list: `deny.toml`

## Security-Critical Dependencies

| Dependency | What It Protects | Audit Status |
|------------|-----------------|-------------|
| `argon2` | Wallet encryption KDF | Reviewed (OWASP-compliant params) |
| `aes-gcm` | Wallet data encryption | Reviewed (256-bit key, GCM auth tag) |
| `sha2` | Block/tx hashing, PoW | Reviewed (standard implementation) |
| `blake3` | Address generation | Reviewed (standard implementation) |
| `getrandom` | All random number generation | Reviewed (OS CSPRNG) |
| `subtle` | Constant-time comparisons | Reviewed (timing-attack prevention) |
| `zeroize` | Memory sanitization | Reviewed (Drop implementations) |
| `pqc-dilithium-seeded` | Post-quantum signatures | **Needs third-party review** |

## License Summary

All dependencies use permissive licenses (MIT, Apache-2.0, BSD-3-Clause).
Enforced by `cargo deny check licenses` in CI.

```bash
# Verify licenses
cargo deny check licenses

# Verify advisories
cargo audit
```

## Reproducibility

- `Cargo.lock` committed and version-controlled
- `rust-toolchain.toml` pins Rust version
- `deny.toml` enforces dependency policies
- CI builds from lockfile on every PR

```bash
# Verify lockfile integrity
cargo verify-project
```
