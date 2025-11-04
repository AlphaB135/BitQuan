# BitQuan Security Policy

## Reporting a Vulnerability
- Email security reports to `security@bitquan.org` with encrypted details (GPG key in `docs/security/keys/`)
- For critical vulnerabilities, also page the on-call maintainer via the emergency contact listed in `MAINTAINERS`
- Response targets:
  - Acknowledge within 24 hours
  - Initial assessment within 72 hours
  - Coordinated disclosure timeline agreed with reporter

## Scope
Security reports include, but are not limited to:
- Consensus failures or chain-halting conditions
- Cryptographic vulnerabilities in PQC primitives or hybrid schemes
- Wallet key compromise pathways or transaction forgery
- P2P networking exploits enabling eclipse, DoS, or relay manipulation
- Build and supply-chain issues affecting reproducibility or binary trust

## Handling Process
1. Form an incident triage pod (Lead Maintainer + security owner + relevant domain expert)
2. Evaluate severity using CVSS and consensus-specific impact metrics
3. Draft mitigation plan including patches, rollout steps, and backport needs
4. Execute coordinated disclosure with responsible parties (exchanges, node operators, researchers)
5. Publish post-mortem after remediation, crediting reporters when permitted

## Prohibited Content
- No acceptance of reports seeking introduction of backdoors, admin keys, or hidden switches
- No bounties for vulnerabilities that rely solely on social engineering without technical impact

## Security Enhancements
- Continuous fuzz testing on consensus critical code paths
- Mandatory code reviews for cryptography and network logic with domain experts
- Dependency scanning for PQC libraries and third-party components
- Regular red-team exercises on wallet, node, and deployment pipelines

## Bounty Program
- Rewards scaled by impact and quality of report, distributed transparently
- Payments occur after patch release and verification by the security team
- Program terms published publicly and updated via BQIP when needed

## Contact
- Primary: `security@bitquan.org`
- GPG Fingerprint (example placeholder): `AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA`
- Backup community channel with limited access shared under NDA during active incidents
## Sprint 3 Security Enhancements (November 2024)

### Error Handling Audit
All production code has been audited for unsafe error handling patterns:

| Crate | unwrap/expect in Prod | Status | Notes |
|-------|----------------------|--------|-------|
| node | 3 | ✅ Justified | HRP encoding constants (compile-time validated) |
| consensus | 0 | ✅ Clean | Test-only panics |
| wallet | 0 | ✅ Clean | Test-only unwraps |
| mempool | 1 | ✅ Justified | Default trait limitation documented |

All remaining `unwrap()`/`expect()` calls have explicit SAFETY comments explaining why they cannot fail.

### Integration Test Coverage
Comprehensive integration tests added:
- **Wallet**: Backup/restore workflows, password rotation, encryption roundtrips
- **Crypto**: Key generation, signing, verification with tampered data tests
- **Mempool**: Transaction lifecycle, policy enforcement, fee validation
- **Types**: Serialization edge cases, network ID handling

### Continuous Monitoring
Nightly CI jobs now include:
- **Miri**: Memory safety checks on core crates
- **Coverage**: Automated coverage report generation
- **Fuzz Ready**: Infrastructure prepared for future fuzzing campaigns

### Audit Readiness
The codebase is now in a stronger position for security audit:
- Clear error boundaries with Result types
- Comprehensive test coverage for critical paths
- Automated safety checks in CI
- Documentation of all intentional panics

## External Audit Preparation (Sprint 4)

### Audit Tooling

BitQuan includes comprehensive audit tooling to support external security reviews:

#### Automated Audit Script

Run the full audit suite:
```bash
bash scripts/audit.sh
```

This script performs:
- **Security Vulnerability Scan** (`cargo audit`): Checks for known CVEs in dependencies
- **License Compatibility** (`cargo deny`): Ensures all dependencies have compatible licenses
- **Unsafe Code Detection** (`cargo geiger`): Identifies unsafe blocks requiring review
- **Code Coverage** (`cargo llvm-cov`): Generates coverage metrics

#### Continuous Integration

Audit checks run automatically:
- **Daily**: Full audit suite runs every night at 3 AM UTC
- **On PR**: License and dependency checks run on Cargo.toml/lock changes
- **Manual**: Can be triggered via GitHub Actions workflow_dispatch

See `.github/workflows/audit.yml` for CI configuration.

### For External Auditors

**Documentation**:
- `docs/ENTROPY_AUDIT.md`: Complete RNG security audit
- `docs/COVERAGE.md`: Code coverage reporting instructions
- `ROADMAP.md`: Development history and sprint summaries

**Test Coverage**:
- Run all tests: `cargo test --all`
- Integration tests: `cargo test --test '*'`
- Entropy sanity: `cargo test entropy`
- Replay protection: `cargo test replay_protection`

**Static Analysis**:
- Clippy (strict): `cargo clippy --all-targets -D warnings`
- Format check: `cargo fmt --all -- --check`
- Dependency tree: `cargo tree --all-features`

**Key Security Properties**:
1. ✅ All RNG uses OsRng (cryptographically secure)
2. ✅ Cross-network replay protection via network_id + genesis_hash
3. ✅ Post-quantum signatures (Dilithium3)
4. ✅ Zero clippy warnings in production code
5. ✅ Comprehensive error handling (no unwrap/expect in critical paths)

