# Security Documentation

**Last Updated: 2025-01-07**

This section contains security policies, audit reports, vulnerability disclosure procedures, and security best practices for BitQuan.

## 🔒 Security Policy

**[Main Security Policy](../../SECURITY.md)** - Vulnerability disclosure and security contact

### Reporting Security Issues

**DO NOT** open public GitHub issues for security vulnerabilities.

📧 **Email**: security@bitquan.network (or maintainer contact)  
🔐 **PGP Keys**: See [keys/](./keys/) directory

Expected response time: 48 hours

## 📋 Security Audits

### Completed Audits
- **[Audit Summary](./AUDIT_SUMMARY.md)** - Overview of all audits
- **[Entropy Audit](./ENTROPY_AUDIT.md)** - CSPRNG and key generation review
- **[Audit Handoff](./AUDIT_HANDOFF.md)** - Security hardening completion
- **[Audit Handoff Checklist](./AUDIT_HANDOFF_CHECKLIST.md)** - Task checklist

### External Audits
⏳ **Status**: Pending pre-mainnet professional audit

Full audit reports available in [audits/](./audits/) directory.

## 🐛 Bug Bounty Program

**[Bug Bounty Details](./BUG_BOUNTY.md)**

### Bounty Tiers
- **Critical**: Up to $10,000 - Consensus bypass, private key extraction
- **High**: Up to $5,000 - DoS, signature forgery
- **Medium**: Up to $1,000 - Information disclosure
- **Low**: Up to $250 - Configuration issues

See [BUG_BOUNTY.md](./BUG_BOUNTY.md) for complete program details.

## 🛡️ Security Features

### Post-Quantum Cryptography
- CRYSTALS-Dilithium3 signatures (NIST FIPS 204)
- 50+ year quantum resistance
- 3293-byte PQC signatures

### Code Security
- Integer overflow protection (checked arithmetic)
- Memory safety (Rust)
- Panic safety in critical paths
- Fuzzing coverage
- Reproducible builds

### Operational Security
- TLS for RPC endpoints
- JWT authentication
- [No Backdoors](./NO_BACKDOORS.md)
- [GPG Signed Releases](./GPG_SIGNING.md)
- [Reproducible Builds](./REPRODUCIBILITY.md)

## 📦 Release Security

All releases include:
- GPG signatures (keys in [keys/](./keys/))
- SHA-256 checksums
- Reproducibility attestations ([attestations/](./attestations/))

## 🚨 Incident Response

- **[Emergency Procedures](./EMERGENCY_PROCEDURES.md)** - Complete emergency response guide
- **[Emergency Quick Reference](./EMERGENCY_QUICK_REFERENCE.md)** - Fast action reference
- **On-Call Roster**: [oncall.md](./oncall.md)
- **Incident Post-Mortems**: `SA-YYYY-NN.md` files
- **Emergency Contact**: security@bitquan.network

## 📖 Best Practices

### Node Operators
- Keep software updated
- Enable firewall, TLS, JWT
- Run as non-root
- Regular encrypted backups

### Developers
- Use checked arithmetic
- No panics in production paths
- Run `cargo audit` and `cargo clippy`
- Review dependencies

### Wallet Users
- Backup mnemonic phrase offline
- Never share private keys
- Verify addresses before sending
- Use hardware wallets for large amounts

## 📚 Related Documentation

- [Operations Guide](../ops/) - Production deployment
- [Development Guide](../dev/) - Build and test
- [Main Security Policy](../../SECURITY.md)

---

*Updated on: 2025-01-07*
