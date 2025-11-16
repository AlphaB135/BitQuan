# Security Audits

This directory contains reports from independent security audits of BitQuan.

## Audit Policy

Before mainnet launch, BitQuan will undergo:
- Minimum 2 independent security audits
- Focus on consensus, cryptography, and network security
- Public disclosure of all findings
- Remediation before launch

## Planned Audits

### Pre-Mainnet (Required)
- [ ] **Audit 1**: Cryptography & PQC Implementation
  - Focus: Dilithium verification, key derivation, RNG
  - Status: Not started
  - Target: Q2 2026

- [ ] **Audit 2**: Consensus & Network Security
  - Focus: PoW, difficulty adjustment, P2P protocol
  - Status: Not started
  - Target: Q2 2026

### Post-Launch (Ongoing)
- [ ] **Annual Security Review**: Yearly comprehensive audit
- [ ] **Bug Bounty Program**: Continuous community testing
- [ ] **Focused Audits**: For major protocol upgrades

## Completed Audits

### None Yet
BitQuan is in active development. First audits will occur before mainnet launch.

## Bug Bounty Program

Coming soon - see `SECURITY.md` for current vulnerability reporting.

Planned rewards:
- **Critical**: $50,000 - $100,000
- **High**: $10,000 - $50,000
- **Medium**: $1,000 - $10,000
- **Low**: $100 - $1,000

## How to Request an Audit

Community members can propose audits:
1. Open GitHub issue with "Audit Request" label
2. Specify scope and justification
3. Steering Committee reviews quarterly
4. Approved audits funded by community donations

## Audit Scope

Typical audit coverage:
- Consensus rules and difficulty adjustment
- Post-quantum cryptography implementation
- Network protocol security
- Mempool and DoS protection
- Wallet and key management
- Build and release process

## Disclosure Policy

- **Immediate**: Critical vulnerabilities (privately to security@bitquan.org)
- **90 Days**: High severity findings (coordinated disclosure)
- **Public**: All findings after fixes deployed
- **Full Report**: Published to this directory after remediation

---

**Security Contact**: security@bitquan.org  
**PGP Key**: See `docs/security/keys/security-team.asc`
