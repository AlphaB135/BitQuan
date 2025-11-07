# Audit Handoff Checklist

This document provides external auditors with a structured entry point and artifact specification for BitQuan security audits.

## Core Documentation

Auditors must review the following documents in priority order:

1. **[SECURITY.md](../SECURITY.md)** - Security policy, threat model, and reporting procedures
2. **[docs/PRELAUNCH_CHECKLIST.md](../ops/PRELAUNCH_CHECKLIST.md)** - Pre-launch validation gates
3. **[docs/ENTROPY_AUDIT.md](./ENTROPY_AUDIT.md)** - Cryptographic entropy sources and RNG analysis
4. **[docs/CONSENSUS_ECON.md](../concepts/CONSENSUS_ECON.md)** - Economic security model and incentive analysis
5. **[docs/TESTNET_README.md](../README.md)** - Testnet results and stress test metrics
6. **[SECURITY_AUDIT_REPORT.md](../SECURITY_AUDIT_REPORT.md)** - Previous audit findings (if applicable)

## Codebase Focus Areas

Auditors should prioritize:

- **Consensus:** `crates/consensus/` (PoW validation, ASERT difficulty, fork choice)
- **Crypto:** `crates/crypto/`, `crates/pqc-dilithium-seeded/` (key derivation, PQC signatures)
- **Network:** `crates/network/` (P2P protocol, DoS protection, propagation)
- **RPC/Stratum:** `crates/rpc/`, `crates/node/src/stratum_server.rs` (authentication, rate limiting)
- **Mempool:** `crates/mempool/` (fee estimation, eviction, double-spend protection)

## Required Artifacts

Upon completion, auditors must produce:

### 1. Audit Report (JSON)
**File:** `auditor_report.json`  
**Schema:**
```json
{
  "status": "pass" | "fail",
  "findings": [
    {
      "severity": "critical" | "high" | "medium" | "low" | "info",
      "title": "string",
      "description": "string",
      "file": "string",
      "line": "number (optional)",
      "recommendation": "string"
    }
  ],
  "sha": "git commit SHA of audited code",
  "tag": "release tag (e.g., v1.0.0-rc1)",
  "auditor": "string",
  "date": "ISO 8601 timestamp"
}
```

### 2. Differential Analysis
**File:** `auditor_diff.md`  
Changes since last audit (if applicable), with risk assessment for each delta.

### 3. Attestation Signature
**File:** `attestation.sig`  
PGP/GPG signature of `auditor_report.json` for authenticity verification.

## Delivery

Submit artifacts via:
- **Secure channel:** Encrypted email or shared secure storage link
- **CI integration:** Upload via GitHub Actions `audit-report.yml` workflow (see `.github/workflows/audit-report.yml`)

## Contact

For questions or clarifications:
- **Security Team:** See [SECURITY.md](../SECURITY.md) for contact details
- **Maintainers:** See [MAINTAINERS](../MAINTAINERS) file

## Scope Boundaries

**In scope:**
- Core consensus and cryptography
- Network protocol and DoS resistance
- RPC/Stratum authentication and authorization
- Mempool integrity

**Out of scope:**
- Third-party dependencies (covered by `cargo audit`)
- UI/UX in wallet bindings
- Performance tuning (unless security-impacting)
