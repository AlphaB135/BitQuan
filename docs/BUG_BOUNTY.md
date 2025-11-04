# BitQuan Bug Bounty Program

**Status**: Active (Testnet Phase)  
**Launch Date**: 2024-11-04  
**Version**: 1.0

---

## Program Overview

BitQuan is committed to the security of our post-quantum blockchain. We welcome responsible security researchers to help identify vulnerabilities and improve the protocol. This bug bounty program rewards ethical hackers for disclosing security issues through coordinated disclosure.

**Key Principles**:

- 🔒 Responsible disclosure (30-day window)
- 💰 Fair rewards based on severity and impact
- 🤝 Collaborative remediation process
- 📢 Public recognition (optional)

---

## Scope

### In-Scope Components

✅ **Consensus Layer** (Critical)

- ASERT difficulty adjustment logic
- BurstGuard spike protection mechanism
- Block validation rules
- Chain reorganization handling
- Timestamp validation
- **Example**: Bypass BurstGuard, manipulate difficulty

✅ **Cryptographic Implementation** (Critical)

- SPHINCS+ signature verification
- Dilithium signature verification
- Entropy sources and RNG usage
- Key derivation functions
- Hash functions (SHA-256, BLAKE2)
- **Example**: Forge signature, predict private keys

✅ **Cross-Network Replay Protection** (Critical)

- Network ID validation
- Genesis hash binding
- Transaction replay across testnet/mainnet
- **Example**: Replay testnet tx on mainnet

✅ **Wallet & Keystore** (High)

- Private key storage and encryption
- Seed phrase generation (BIP39)
- Multi-signature logic (m-of-n)
- Key derivation (BIP32-like)
- Passphrase handling
- **Example**: Extract keys from encrypted keystore

✅ **Mempool & Transaction Policy** (High)

- Transaction validation and propagation
- Double-spend detection
- Fee calculation and manipulation
- RBF (Replace-By-Fee) logic
- Resource limits and DoS protection
- **Example**: DoS via malformed transactions

✅ **RPC Security** (High)

- Authentication and authorization
- Input validation and sanitization
- Rate limiting bypass
- Command injection
- Information disclosure
- **Example**: Execute unauthorized RPC calls

✅ **Network Protocol (P2P)** (Medium)

- Handshake and peer authentication
- Message parsing and validation
- Connection limits and DoS protection
- Eclipse attack vectors
- Sybil attack resistance
- **Example**: Crash node via malformed P2P message

✅ **Storage Layer** (Medium)

- Database integrity
- Backup and recovery
- Data corruption attacks
- **Example**: Corrupt blockchain database

### Out-of-Scope

❌ **Not Eligible for Rewards**:

- Social engineering attacks
- Physical attacks on hardware
- Denial of service (DDoS) attacks on infrastructure
- Issues in third-party dependencies (report to upstream)
- UI/UX bugs without security impact
- Testnet faucet abuse (rate limiting expected)
- Block explorer bugs (external tool)
- Vulnerabilities requiring physical access
- Previously known issues (check GitHub Issues)
- Spam or automated vulnerability scanner output
- Best practices without demonstrated exploit

---

## Severity Levels & Rewards

### Critical (Up to 50,000 BQ + Recognition)

**Impact**: Consensus break, total loss of funds, network-wide compromise

**Examples**:

- Bypass consensus rules to create invalid blocks
- Forge signatures for arbitrary addresses
- Steal private keys from encrypted keystore
- Replay protection bypass (cross-network)
- Mint arbitrary coins (inflation bug)
- Double-spend with high confidence
- Remote code execution on nodes

**Requirements**:

- Proof-of-concept demonstrating full exploit
- Detailed write-up of root cause
- Suggested remediation

### High (Up to 20,000 BQ)

**Impact**: Partial loss of funds, node crash, mempool DoS

**Examples**:

- Mempool DoS causing network congestion
- RPC authentication bypass
- Crack keystore encryption with reasonable resources
- Wallet private key leak through side-channel
- Break multi-signature wallet logic
- BurstGuard bypass under specific conditions
- Eclipse attack against honest nodes

**Requirements**:

- Reproducible exploit steps
- Impact analysis
- Code pointers to vulnerable components

### Medium (Up to 5,000 BQ)

**Impact**: Information disclosure, minor DoS, policy violations

**Examples**:

- Leak sensitive information through RPC
- Bypass rate limiting
- Transaction policy violations
- Peer banning bypass
- Minor information leakage (e.g., node version)
- Time-based side-channel leaks

**Requirements**:

- Clear reproduction steps
- Evidence of successful exploit

### Low (Up to 1,000 BQ)

**Impact**: Best practice violations, minor information leaks

**Examples**:

- Weak default configurations
- Logging sensitive data
- Insecure error messages
- Missing security headers
- Suboptimal cryptographic parameters

**Requirements**:

- Description of issue
- Suggested improvement

### Informational (Recognition Only)

**Impact**: Code quality, documentation improvements

**Examples**:

- Code style inconsistencies
- Missing documentation
- Typos in error messages
- Performance optimizations without security impact

---

## Reward Payment

### Payment Process

1. **Validation**: Team confirms vulnerability and severity
2. **Remediation**: Fix developed and tested (7-30 days)
3. **Verification**: Researcher verifies fix
4. **Disclosure**: Public advisory published (30 days after fix)
5. **Payment**: BQ tokens sent to provided address

### Payment Mechanism

**Testnet Phase**: Rewards paid in testnet BQ (symbolic)  
**Mainnet Phase**: Rewards paid from multi-signature dev fund

**Dev Fund Address** (Testnet): `[TBD - 3-of-5 multisig address]`  
**Mainnet Address**: `[TBD - after mainnet launch]`

### Bonus Multipliers

- **First reporter**: 1.0x base reward
- **High quality write-up**: +25% bonus
- **Suggested fix included**: +15% bonus
- **Critical timing (pre-mainnet)**: +50% bonus

**Example**: Critical bug with excellent write-up and fix suggestion:

- Base: 50,000 BQ
- Quality bonus: +12,500 BQ (25%)
- Fix bonus: +7,500 BQ (15%)
- **Total**: 70,000 BQ

---

## Submission Guidelines

### How to Report

1. **GitHub Security Advisory** (Preferred for Critical/High)
   - Repository: https://github.com/AlphaB135/BitQuan/security/advisories
   - Click "Report a vulnerability"
   - Fill out template

2. **PGP-Encrypted Email** (For sensitive disclosures)
   - Email: [TBD - security contact]
   - PGP Key: [TBD - public key fingerprint]
   - Subject: `[BOUNTY] BitQuan Vulnerability Report`

3. **GitHub Issue** (For Low/Informational)
   - Create issue with label: `security`, `bug-bounty`
   - Public disclosure acceptable for low-severity issues

### Report Template

```markdown
# Vulnerability Report: [Short Title]

## Summary

[Brief description in 1-2 sentences]

## Severity Assessment

Critical / High / Medium / Low / Informational

## Affected Component

- **Module**: consensus / wallet / rpc / p2p / crypto / storage
- **File**: [path to vulnerable file]
- **Function**: [affected function name]

## Vulnerability Details

### Description

[Detailed explanation of the vulnerability]

### Root Cause

[Why does this vulnerability exist?]

### Attack Scenario

[Step-by-step attack scenario]

### Prerequisites

[What does attacker need? Network access? Running node?]

## Proof of Concept

### Environment

- BitQuan version: v1.0.0-rc1
- Operating System: Ubuntu 22.04
- Rust version: 1.82.0

### Reproduction Steps

1. [Step 1]
2. [Step 2]
3. [Observe: Expected vs Actual behavior]

### Code/Script

\`\`\`bash

# PoC exploit script

[Include runnable proof-of-concept]
\`\`\`

### Evidence

[Screenshots, logs, or other evidence]

## Impact Analysis

### Worst-Case Scenario

[What's the maximum damage?]

### Likelihood

High / Medium / Low - [Why?]

### Affected Users

[Who is impacted? All nodes? Wallet users only?]

## Suggested Fix

### Proposed Solution

[High-level fix strategy]

### Code Patch (Optional)

\`\`\`rust
// Suggested code changes
\`\`\`

### Mitigation

[Temporary workaround until patch deployed?]

## References

- [Related CVEs]
- [Academic papers]
- [Similar vulnerabilities in other projects]

## Disclosure Timeline

- [Date] - Vulnerability discovered
- [Date] - Reported to BitQuan team
- [Target] - Expected public disclosure (30 days after fix)

## Researcher Information

- **Name**: [Your name or pseudonym]
- **Contact**: [Email or Twitter]
- **Payment Address**: [BQ address for reward]
- **Public Recognition**: Yes / No / Pseudonym only
```

---

## Responsible Disclosure Policy

### Timeline

```
Day 0:    Report submitted
Day 1-2:  Acknowledgment (within 48 hours)
Day 3-7:  Initial triage and severity assessment
Day 7-30: Remediation and patch development
Day 30:   Patch released, researcher verification
Day 60:   Public disclosure (30 days after patch)
```

### Researcher Expectations

✅ **DO**:

- Report vulnerabilities promptly
- Provide detailed reproduction steps
- Give reasonable time for remediation (30 days minimum)
- Communicate clearly and professionally
- Verify fixes before public disclosure

❌ **DON'T**:

- Publicly disclose before coordinated release
- Exploit vulnerabilities for personal gain
- Attack production infrastructure
- Access data belonging to others
- Demand ransom or make threats

### Safe Harbor

BitQuan commits to:

- ✅ Not pursue legal action against security researchers acting in good faith
- ✅ Work collaboratively to understand and fix issues
- ✅ Provide credit and recognition (if desired)
- ✅ Pay rewards fairly and promptly

---

## Exclusions & Limitations

### Not Eligible

- ❌ Vulnerabilities found through automated scanners without validation
- ❌ Issues already reported by another researcher
- ❌ Publicly known vulnerabilities (check GitHub Issues first)
- ❌ Theoretical attacks without proof-of-concept
- ❌ Vulnerabilities in test/example code not used in production
- ❌ Social engineering, phishing, or physical attacks
- ❌ Third-party dependency issues (report upstream first)

### Rate Limits

- Maximum 3 submissions per researcher per week
- Duplicate reports: First valid report receives reward
- Similar vulnerabilities: May be grouped for single reward

---

## FAQ

### Q: Can I test on the public testnet?

**A**: Yes! Testnet is explicitly for testing. However:

- Don't attack infrastructure (nodes, faucet)
- Don't spam network (stay within rate limits)
- Don't steal testnet funds from others

### Q: What if my report is rejected?

**A**: You'll receive explanation for rejection. Common reasons:

- Out of scope
- Duplicate report
- Insufficient severity
- Not reproducible

You can appeal or provide additional evidence.

### Q: How long until I receive payment?

**A**: Typically 7-30 days after fix verification. Complex issues may take longer.

### Q: Can I remain anonymous?

**A**: Yes, pseudonyms accepted. Provide valid BQ address for payment.

### Q: What if I find multiple vulnerabilities?

**A**: Submit separate reports for each. Each evaluated independently.

### Q: Is there a limit to rewards?

**A**: Program cap: 500,000 BQ per year (testnet). Mainnet limits TBD.

### Q: What about vulnerabilities in dependencies?

**A**: Report to upstream project first. If they're unresponsive and it impacts BitQuan, report to us.

---

## Recognition

### Hall of Fame

Security researchers who help secure BitQuan will be listed in:

- `docs/SECURITY.md` - Hall of Fame section
- GitHub Security Advisories (public credit)
- Release notes for patched versions

**Format**:

```
- [Researcher Name/Pseudonym] - [Vulnerability Type] - [Date] - [Severity]
```

### Swag & Perks

Top contributors receive:

- 🏆 BitQuan Contributor Badge (GitHub)
- 👕 Limited edition BitQuan swag
- 📣 Shoutout on social media (if desired)
- 🎤 Invitation to present findings (conferences, blog post)

---

## Legal

### Terms & Conditions

By participating in this bug bounty program:

- You agree to our responsible disclosure policy
- You confirm the vulnerability is original research
- You grant BitQuan right to use your findings for remediation
- You agree to payment terms and reward structure

### Safe Harbor Agreement

BitQuan provides safe harbor for security researchers acting in good faith, provided:

- Testing is limited to your own accounts/nodes
- You don't access data belonging to others
- You report findings promptly
- You don't publicly disclose before coordinated release

---

## Contact

**Security Team**: [TBD - email or contact form]  
**PGP Key**: [TBD - public key fingerprint]  
**Response Time**: 48 hours for acknowledgment  
**GitHub**: https://github.com/AlphaB135/BitQuan/security

---

## Updates to This Program

This bug bounty program may be updated as BitQuan evolves:

- Scope adjustments (mainnet launch)
- Reward structure changes
- New vulnerability categories

**Current Version**: 1.0 (Testnet Phase)  
**Last Updated**: 2024-11-04  
**Next Review**: After mainnet launch

---

## Acknowledgments

BitQuan thanks the security community for helping build a more secure post-quantum blockchain. Your contributions are invaluable to the future of decentralized finance.

**Happy Hunting! 🔍🐛**

---

**Program Status**: ✅ Active  
**Testnet Rewards**: Active  
**Mainnet Rewards**: TBD (after mainnet launch)
