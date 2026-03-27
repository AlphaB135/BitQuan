# BitQuan Threat Model (STRIDE Analysis)

**Version**: 1.0
**Last Updated**: 2026-03-27
**Methodology**: STRIDE (Microsoft)

---

## Threat Categories

### S — Spoofing (Identity Spoofing)

| Threat | Impact | Likelihood | Mitigation | Status |
|--------|--------|------------|------------|--------|
| Fake peer identity in P2P network | Medium | High | Peer handshake protocol, ban scoring | Done |
| Forged JWT tokens for RPC access | High | Low | HMAC-SHA256 JWT signing, secret rotation | Done |
| Impersonated bootstrap nodes | High | Medium | Hardcoded genesis hash, peer verification | Done |
| Spoofed miner submitting blocks | High | Low | PoW validation, signature verification | Done |

### T — Tampering (Data Tampering)

| Threat | Impact | Likelihood | Mitigation | Status |
|--------|--------|------------|------------|--------|
| Malicious block injection | Critical | Medium | Full block validation (consensus rules) | Done |
| Transaction modification in transit | High | Low | Transaction hash integrity, merkle tree | Done |
| UTXO set manipulation | Critical | Low | Hash-based UTXO lookup, append-only storage | Done |
| Chain reorganization attack | High | Medium | ASERT difficulty, reorg depth limit | Done |
| Genesis block tampering | Critical | Low | Hardcoded genesis hash in config | Done |

### R — Repudiation (Non-repudiation)

| Threat | Impact | Likelihood | Mitigation | Status |
|--------|--------|------------|------------|--------|
| Denying a transaction was sent | Medium | Low | Dilithium3 signatures (post-quantum undeniable) | Done |
| Denying block was mined by node | Low | Low | Coinbase signature, block hash proof | Done |
| RPC action cannot be traced | Medium | Low | JWT auth, structured logging | Done |

### I — Information Disclosure

| Threat | Impact | Likelihood | Mitigation | Status |
|--------|--------|------------|------------|--------|
| Private key leakage via logs | Critical | Low | Logging security policy, secret masking | Done |
| Memory dump revealing keys | Critical | Medium | Memory locking (mlock), zeroization on drop | Done |
| Timing attack on KDF | High | Medium | Constant-time comparisons, Argon2id masking | Done |
| Peer address exposure | Low | High | Expected behavior (P2P requires it) | Accepted |
| RPC endpoint data leakage | Medium | Medium | JWT auth, rate limiting, input validation | Done |

### D — Denial of Service (DoS)

| Threat | Impact | Likelihood | Mitigation | Status |
|--------|--------|------------|------------|--------|
| Transaction spam filling mempool | High | High | Mempool size limits, min relay fee, eviction policy | Done |
| Malformed message flood | High | High | Message size limits, rate limiting, ban scoring | Done |
| Resource exhaustion via large blocks | High | Medium | Max block weight (4MW), block size validation | Done |
| Eclipse attack (isolate from honest peers) | Critical | Low | Multiple bootstrap nodes, outbound connection diversity | Partial |
| RandomX mining DoS (memory-intensive) | Medium | Medium | Optional RandomX feature flag, not required for SHA-256d | Done |

### E — Elevation of Privilege

| Threat | Impact | Likelihood | Mitigation | Status |
|--------|--------|------------|------------|--------|
| RPC auth bypass gaining admin access | Critical | Low | JWT validation, least-privilege RPC methods | Done |
| Wallet encryption brute-force | High | Medium | Argon2id KDF (256 MiB), exponential backoff, lockout | Done |
| Arbitrary code execution via deserialization | Critical | Low | No unsafe code, input validation, trusted bincode | Done |
| Miner gaining consensus control | Critical | Low | ASERT difficulty, BurstGuard spike protection | Done |

---

## Trust Boundaries

```
┌─────────────────────────────────────────┐
│           UNTRUSTED (Internet)          │
│                                         │
│  ┌─────────┐  ┌─────────┐  ┌────────┐  │
│  │  Peer   │  │  RPC    │  │ Miner  │  │
│  │  (P2P)  │  │ Client  │  │        │  │
│  └────┬────┘  └────┬────┘  └───┬────┘  │
│       │            │           │        │
├───────┼────────────┼───────────┼────────┤
│       │     TRUST BOUNDARY       │        │
│       │    (authenticated)       │        │
│       v            v           v        │
│  ┌─────────────────────────────────┐    │
│  │        BitQuan Node             │    │
│  │  ┌───────┐ ┌──────┐ ┌────────┐ │    │
│  │  │Network│ │ RPC  │ │Consensus│ │    │
│  │  │ Layer │ │Auth  │ │ Engine │ │    │
│  │  └───────┘ └──────┘ └────────┘ │    │
│  │  ┌───────┐ ┌──────┐ ┌────────┐ │    │
│  │  │Crypto │ │Store │ │Mempool│ │    │
│  │  │Module │ │      │ │        │ │    │
│  │  └───────┘ └──────┘ └────────┘ │    │
│  └─────────────────────────────────┘    │
│                                         │
├─────────────────────────────────────────┤
│         LOCAL (File System)             │
│  ┌──────────┐  ┌──────────────────┐    │
│  │ Wallet   │  │ Chain Data       │    │
│  │ (encrypted)│  │ (append-only)   │    │
│  └──────────┘  └──────────────────┘    │
└─────────────────────────────────────────┘
```

## Attack Tree: Most Critical Path

```
Goal: Steal funds from wallet
├── Method 1: Compromise encrypted keystore
│   ├── Brute-force password [Mitigated: Argon2id, backoff, lockout]
│   ├── Memory dump during unlock [Mitigated: mlock, zeroization]
│   └── Keylogger on host machine [Out of scope: OS-level]
│
├── Method 2: Forge transactions
│   ├── Break Dilithium3 signatures [Mitigated: NIST PQC standard]
│   ├── Replay old transactions [Mitigated: UTXO model, double-spend check]
│   └── Modify transaction in transit [Mitigated: hash integrity]
│
└── Method 3: Manipulate consensus
    ├── 51% hash power attack [Mitigated: ASERT, BurstGuard]
    ├── Chain reorganization [Mitigated: reorg depth limit]
    └── Fake bootstrap nodes [Mitigated: genesis hash pinning]
```

## Assumptions

1. Host OS is trusted (we don't mitigate hardware-level attacks)
2. Rust compiler and dependencies are trusted (supply chain is a separate concern)
3. Quantum computers with sufficient qubits do not exist yet (Dilithium3 provides quantum resistance)
4. Network-level attacks (BGP hijacking) are out of scope
5. Social engineering is out of scope

## Out of Scope

- Physical attacks on hardware (Tempest, side-channel at hardware level)
- Compiler backdoors
- Dependency supply chain attacks (covered by cargo-deny/cargo-audit)
- Social engineering / phishing
- Regulatory / legal attacks
