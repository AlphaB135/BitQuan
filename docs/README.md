# BitQuan Documentation

Welcome to the BitQuan blockchain documentation. This directory contains comprehensive documentation covering architecture, security, operations, and development guides.

## Quick Links

- **[Architecture](./guides/)** - System architecture and design patterns
- **[Security](./security/)** - Security features, audits, and best practices
- **[Migrations](./migrations/)** - Protocol upgrades and migration guides
- **[Operations](./operations/)** - Deployment, monitoring, and disaster recovery
- **[Testing](./testing/)** - Testing guides and reports

## Documentation Structure

```
docs/
├── guides/           # Developer guides and integration docs
├── security/         # Security documentation
│   └── audit-reports/    # Security audit reports
├── migrations/       # Protocol upgrade and migration guides
├── operations/       # Operations and deployment docs
├── testing/          # Testing documentation and reports
└── reports/          # Various project reports
```

## Core Documentation

### Getting Started
- **[Quick Start Guide](QUICK_START_GUIDE.md)** - Get up and running in 5 minutes
- **[Start Here](START_HERE.md)** - New contributor onboarding
- **[Developer Guide](DEVELOPER_GUIDE.md)** - Build, test, and contribute
- **[Release Notes v0.2.0](RELEASE_NOTES_v0.2.0.md)** - Latest release

### Security Audits (2026-08-17)
- **[Security Audit Complete](security/SECURITY_AUDIT_COMPLETE.md)** - Final audit report ✅ Testnet approved
- **[ShipProof Report](security/SHIPPROOF_REPORT.md)** - Production risk scan (250 findings, 0 blocking)
- **[SP203 Fixed](security/SP203_FIXED.md)** - GitHub Actions pinning (189 actions secured)
- **[Security Audit Report](security/SECURITY_AUDIT_REPORT.md)** - Comprehensive security analysis
- **[Security Arsenal](security/README_SECURITY_ARSENAL.md)** - Security tools and methodologies

### Security Deep Dive
- **[SECURITY-HARDENING.md](./security/SECURITY-HARDENING.md)** - Security hardening guidelines
- **[WALLET_SECURITY_AUDIT_REPORT.md](./security/audit-reports/WALLET_SECURITY_AUDIT_REPORT.md)** - Wallet security audit
- **[SECURITY-SCANNING.md](./security/SECURITY-SCANNING.md)** - Automated security scanning
- **[Executive Summary](security/EXECUTIVE_SUMMARY.md)** - High-level security overview
- **[Threat Model](THREAT_MODEL.md)** - Attack surface analysis

### Testing & Quality Assurance
- **[Test Specification Matrix](testing/MODULE_1_TEST_SPECIFICATION_MATRIX.md)** - Comprehensive test coverage
- **[Test Runbooks](testing/MODULE_2_TEST_RUNBOOKS.md)** - Step-by-step testing procedures
- **[Testnet Launch SOP](testing/MODULE_3_TESTNET_LAUNCH_SOP.md)** - Launch standard operating procedures
- **[Production Readiness Signoff](testing/MODULE_4_PRODUCTION_READINESS_SIGNOFF.md)** - Go-live checklist
- **[TRANSACTION_TESTING_GUIDE.md](./testing/TRANSACTION_TESTING_GUIDE.md)** - Transaction testing guide
- **[TRANSACTION_TEST_FINAL_REPORT.md](./testing/TRANSACTION_TEST_FINAL_REPORT.md)** - Test results summary

### Protocol & Upgrades
- **[ASYNC_MIGRATION_PLAN.md](./migrations/ASYNC_MIGRATION_PLAN.md)** - Async architecture migration plan
- **[ASYNC_MIGRATION_STATUS.md](./migrations/ASYNC_MIGRATION_STATUS.md)** - Migration status tracker

### Operations & Deployment
- **[Production Deployment](PRODUCTION_DEPLOYMENT.md)** - Mainnet deployment guide
- **[Launch Checklist](LAUNCH_CHECKLIST.md)** - Pre-launch verification
- **[IBD_PROGRESS_TRACKING.md](./operations/IBD_PROGRESS_TRACKING.md)** - Initial Block Download progress monitoring
- **[DISASTER-RECOVERY.md](./operations/DISASTER-RECOVERY.md)** - Disaster recovery procedures
- **[ROADMAP](./operations/ROADMAP.md)** - Project roadmap and milestones

### Funding
- **[Funding](FUNDING.md)** - Support BitQuan development

## Key Concepts

### Post-Quantum Cryptography
BitQuan uses **Dilithium5** for all signatures, providing quantum-resistant security:
- Signature size: 4,595 bytes
- Public key size: 2,592 bytes
- Security level: NIST Level 5 (highest)

### Consensus
- **Proof-of-Work**: SHA-256d, RandomX, and Ethash hybrid mining
- **ASERT**: Asymptotically schedule-difficulty retarget
- **Burst Guard**: Protection against rapid difficulty attacks

### P2P Network
- **Noise Protocol**: `Noise_XX_25519_ChaChaPoly_BLAKE2s` for encrypted connections
- **DoS Protection**: Rate limiting, message size caps, peer banning
- **Sync**: Headers-first sync with block locator exponential backoff

### Security Features
- **UTXO Double Spend Prevention**: HashSet tracking within blocks
- **Coinbase Maturity**: 100-block maturity requirement
- **Input Validation**: Strict validation with descriptive error messages
- **File Permissions**: 0o600 for sensitive files (JWT, keystore)

## Development Quick Start

```bash
# Build the project
make build

# Run tests
make test

# Run linter
make clippy

# Format code
make fmt

# Run pre-commit checks
make ck
```

## For Contributors

See **[CONTRIBUTING.md](../CONTRIBUTING.md)** for contribution guidelines.

## License

BitQuan is licensed under the **Apache License 2.0**. See **[LICENSE](../LICENSE)** for details.

---

*Documentation last updated: 2026-01-22*
