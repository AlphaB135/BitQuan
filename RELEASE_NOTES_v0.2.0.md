# BitQuan v0.2.0 - Security Hardened Release

**Release Date**: 2026-08-16  
**Testing**: 3 rounds penetration testing, 27 vulnerabilities fixed

---

## What's New

This release contains **comprehensive security hardening** after 3 rounds of penetration testing.

### Security Improvements

**27 vulnerabilities discovered and fixed**:
- 15 vulnerabilities (Round 1): TOCTOU races, memory exhaustion, integer overflows
- 10 vulnerabilities (Round 2): Wallet cache race, CORS exposure, eclipse attacks
- 2 vulnerabilities (Round 3): Atomic ordering, cache overflow

**Live attack testing**:
- Survived 10,000+ attack requests
- No crashes, no memory leaks
- Stress tested with 50 concurrent workers
- Validated against real-world CVEs (Bitcoin Core, Ethereum)

### Critical Fixes

- **CHAIN-NEW-001**: Wallet cache TOCTOU race condition (complete refactor)
- **CHAIN-NEW-005**: CORS allow_any_origin() replaced with whitelist
- **NEW-001/002**: Eclipse attack prevention (subnet diversity enforced)
- **CHAIN-NEW-002**: Faucet rate limiter TOCTOU (atomic Entry API)
- **CHAIN-NEW-011**: Atomic memory ordering (19 occurrences: Relaxed → SeqCst)
- **CHAIN-NEW-012**: Integer overflow in cache accounting (saturating arithmetic)

### Attack Resistance Verified

✓ Resource exhaustion (CVE-2025-54604 style)  
✓ Integer overflow (CVE-2026-34219 style)  
✓ Eclipse attacks (subnet diversity)  
✓ Rate limit bypass (50/50 requests blocked)  
✓ Protocol downgrade (JSON-RPC 2.0 enforced)  
✓ Serialization bomb (nested JSON rejected)

---

## Documentation

New audit reports and documentation:
- `audit-reports/session-report.html` - Comprehensive HTML report
- `audit-reports/` - 17 detailed security reports
- `audit-scripts/` - Attack validation scripts
- `AUDIT_SESSION_SUMMARY.md` - Session summary
- `README.md` - Rewritten (technical, no-fluff style)

---

## Breaking Changes

None. This is a security hardening release with no API changes.

---

## Upgrade Notes

This release is **highly recommended** for all nodes:
- Fixes critical security vulnerabilities
- Improves network stability (eclipse attack prevention)
- Enhances DoS protection (rate limiting, memory bounds)

### How to Upgrade

```bash
# Pull latest code
git pull origin main

# Rebuild
cargo build --release

# Restart node
./target/release/bitquan-node run --config config/testnet.toml
```

---

## Known Limitations

**Inherent risks** (cannot be eliminated):
- 51% attack (inherent to all PoW chains)
- Sybil attack (mitigated with subnet diversity)
- Quantum attacks on SHA-256d PoW (future concern)

**Before mainnet launch**:
- Multi-node P2P network testing recommended
- External security audit recommended
- Long-running fuzzing campaign recommended

---

## Commits

```
2057dae - docs: add comprehensive security audit reports and README update
2d6218a - security: fix CHAIN-NEW-011 and CHAIN-NEW-012 from round 3
dcd2bf3 - security: fix 10 NEW vulnerabilities from round 2 audit
55953e0 - fix(tests): align reward engine and async mock store
706ae07 - fix(stratum): use blocking_lock in check_rate_limit
24115f9 - fix(chainstate): update tip_hash before height
e8bce80 - fix(reward_engine): exclude coinbase from fee estimate
68efb98 - fix(reward_engine): cap blocks Vec to prevent OOM
c902957 - fix(rate_limiter): decay violations on window reset
f8c94dc - fix(rate_limiter): implement remove_peer
fe13600 - fix(chainstate): eliminate O(height) inner loop
988b034 - fix(sync): return Err instead of silently dropping blocks
604aed9 - fix(network): reject handshake messages exceeding u16::MAX
b31bab4 - fix(network): re-check subnet diversity after handshake
07914d3 - fix(rpc): calculate merkle root before mining
0f5e1dc - fix(rpc): wire submitblock to storage layer
608aa37 - fix(sync): replace DefaultHasher with header_hash
7a234c1 - fix(script): preserve op_count across execute calls
```

---

## Full Changelog

See `CHANGELOG.md` for complete list of changes.

---

## Verification

**SHA256 checksums** (Linux x86_64):
```
# To be generated after build
sha256sum target/release/bitquan-node
sha256sum target/release/bitquan-cli
```

**Security audit reports**:
- All reports available in `audit-reports/` directory
- HTML report: `audit-reports/session-report.html`

---

## Support

- **Issues**: https://github.com/AlphaB135/BitQuan/issues
- **Security**: See SECURITY.md
- **Documentation**: See docs/ directory

---

## Credits

Security audit conducted 2026-08-15 to 2026-08-16.

All vulnerabilities have been fixed and verified under live attack conditions.

---

*"Code doesn't lie. Whitepapers do."*
