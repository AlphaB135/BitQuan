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

### Architecture & Development
- **[CLAUDE.md](../CLAUDE.md)** - AI assistant guidelines and development workflows
- **[README](../README.md)** - Project overview and quick start

### Security
- **[SECURITY-HARDENING.md](./security/SECURITY-HARDENING.md)** - Security hardening guidelines
- **[WALLET_SECURITY_AUDIT_REPORT.md](./security/audit-reports/WALLET_SECURITY_AUDIT_REPORT.md)** - Wallet security audit
- **[SECURITY-SCANNING.md](./security/SECURITY-SCANNING.md)** - Automated security scanning

### Protocol & Upgrades
- **[ASYNC_MIGRATION_PLAN.md](./migrations/ASYNC_MIGRATION_PLAN.md)** - Async architecture migration plan
- **[ASYNC_MIGRATION_STATUS.md](./migrations/ASYNC_MIGRATION_STATUS.md)** - Migration status tracker

### Operations
- **[DISASTER-RECOVERY.md](./operations/DISASTER-RECOVERY.md)** - Disaster recovery procedures
- **[ROADMAP](./operations/ROADMAP.md)** - Project roadmap and milestones

### Testing
- **[TRANSACTION_TESTING_GUIDE.md](./testing/TRANSACTION_TESTING_GUIDE.md)** - Transaction testing guide
- **[TRANSACTION_TEST_FINAL_REPORT.md](./testing/TRANSACTION_TEST_FINAL_REPORT.md)** - Test results summary

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

BitQuan is licensed under the **MIT License**. See **[LICENSE](../LICENSE)** for details.

---

*Documentation last updated: 2026-01-22*
