# No Backdoors Policy

## Absolute Commitment

**BitQuan categorically rejects all forms of backdoors, hidden access mechanisms, and privileged control features.**

This document serves as our unequivocal public declaration and technical specification ensuring this commitment.

## Policy Statement

BitQuan development follows these principles:

### 1. Zero Backdoors
- ❌ No admin keys or master keys
- ❌ No hidden transaction approval mechanisms
- ❌ No emergency stop functions
- ❌ No privileged node capabilities
- ❌ No hardcoded addresses with special rights
- ❌ No undocumented protocol features
- ❌ No remote kill switches

### 2. Zero Hidden Access
- ❌ No undocumented RPC commands
- ❌ No secret network protocols
- ❌ No hidden consensus rules
- ❌ No covert data collection
- ❌ No telemetry without explicit consent
- ❌ No phone-home mechanisms

### 3. Zero Trust Requirements
- ✅ All code is open source
- ✅ All builds are reproducible
- ✅ All commits are GPG-signed
- ✅ All cryptography is standard and audited
- ✅ All network traffic is documented
- ✅ All consensus rules are explicit

## Code Review Standards

### Prohibited Patterns

Our automated code review **FAILS the build** if it detects:

```rust
// FORBIDDEN - will not compile
const ADMIN_KEY: [u8; 32] = [...];
const MASTER_ADDRESS: &str = "...";
const BACKDOOR_MODE: bool = ...;
const GOD_MODE: bool = ...;
const EMERGENCY_OVERRIDE: bool = ...;

// FORBIDDEN keywords in production code
#[cfg(feature = "admin")]
#[cfg(feature = "backdoor")]
#[cfg(feature = "master_key")]
```

### Static Analysis Rules

```yaml
# .github/workflows/security-audit.yml
forbidden_keywords:
  - "admin_key"
  - "master_key"
  - "backdoor"
  - "god_mode"
  - "emergency_stop"
  - "kill_switch"
  - "hidden_feature"
  - "secret_mode"
  - "hardcoded_address"

action: FAIL_BUILD
```

## Technical Verification

### 1. Source Code Audit
Anyone can verify our code is backdoor-free:

```bash
# Clone repository
git clone https://github.com/bitquan/bitquan.git
cd bitquan

# Search for prohibited patterns
grep -r "admin_key\|master_key\|backdoor\|god_mode" .
# Should return: No matches

# Check for hardcoded keys
grep -r "const.*KEY.*\[u8; 32\]" . | grep -v "test"
# Review each instance for legitimacy

# Search for special addresses
grep -r "q1[a-z0-9]\{39\}" . | grep -v "test\|example"
# Should only find test/example addresses
```

### 2. Network Traffic Analysis
```bash
# Run node with traffic monitoring
tcpdump -i any -w bitquan-traffic.pcap &
./bitquan-node run

# Analyze captured traffic
# All connections should be:
# - To documented peer nodes
# - Using documented P2P protocol
# - No unexpected external connections
```

### 3. Binary Analysis
```bash
# Extract all string constants
strings target/release/bitquan-node > strings.txt

# Search for suspicious patterns
grep -i "admin\|master\|backdoor\|secret" strings.txt
# Review each match for context
```

## Consensus Rules Transparency

### All Consensus Rules Are Public

Every consensus rule is documented in:
- `docs/spec/consensus.md` - Full specification
- `crates/consensus/src/` - Implementation
- Test vectors in `docs/spec/test-vectors.md`

### No Special Cases

```rust
// ✅ GOOD: Explicit, documented consensus rule
pub fn validate_block(block: &Block, height: u64) -> Result<()> {
    if block.header.version != 1 {
        return Err(ConsensusError::InvalidVersion);
    }
    // ... more documented checks
}

// ❌ FORBIDDEN: Hidden special case
pub fn validate_block(block: &Block, height: u64) -> Result<()> {
    // BACKDOOR: Skip validation for special address
    if block.coinbase_address == ADMIN_ADDRESS {
        return Ok(()); // FORBIDDEN!
    }
    // ...
}
```

## Cryptographic Standards

### Only Standard Algorithms
- ✅ SHA-256 (FIPS 180-4)
- ✅ CRYSTALS-Dilithium (NIST PQC winner)
- ✅ ChaCha20 (RFC 8439)
- ✅ HKDF (RFC 5869)

### No Custom Cryptography
- ❌ No proprietary signature schemes
- ❌ No modified hash functions
- ❌ No custom key derivation
- ❌ No "enhanced" encryption

### Backdoor-Free Libraries
```toml
[dependencies]
# All crypto libraries are:
# - Well-known and audited
# - Open source
# - Reproducibly buildable
sha2 = "0.10"              # SHA-256
pqc_dilithium = "0.2"      # NIST PQC
rand_chacha = "0.3"        # ChaCha20 RNG
hkdf = "0.12"              # HKDF
```

## Build System Security

### Reproducible Builds Prevent Backdoors
```bash
# Official binary checksum
sha256sum bitquan-node-v1.0.0
# abc123...

# Your independent build
cargo build --release
sha256sum target/release/bitquan-node
# abc123... (MUST MATCH!)

# If checksums don't match → potential backdoor
```

### Multi-Party Build Verification
- Minimum 3 independent builders verify each release
- Attestations published at `docs/security/attestations/`
- Any mismatch triggers security audit

## Network Protocol Transparency

### All Network Messages Documented
```rust
// Every P2P message is documented
pub enum Message {
    /// Request block headers
    GetHeaders { ... },
    /// Send block headers
    Headers { ... },
    /// Request full blocks
    GetBlocks { ... },
    // ... etc
}

// ❌ FORBIDDEN: Undocumented message types
pub enum Message {
    // ...
    #[doc(hidden)]  // FORBIDDEN!
    AdminCommand { ... },
}
```

### Wireshark Dissector Available
- Complete packet dissector at `tools/wireshark/bitquan.lua`
- Decode all network traffic
- No encrypted/hidden channels (except optional TLS)

## Configuration Transparency

### No Hidden Settings
```toml
# config/bitquan.toml - GOOD EXAMPLE
[network]
listen_addr = "0.0.0.0:8333"
max_peers = 125

[consensus]
block_time = 600  # 10 minutes

# ❌ FORBIDDEN: Hidden features
# [admin]
# backdoor_enabled = false  # FORBIDDEN!
```

### All Features Optional and Explicit
```bash
# No features enabled by default that could be backdoors
cargo build --release
# vs
cargo build --release --features admin  # ← Would fail CI!
```

## Third-Party Audit Trail

### External Audits Required
- Minimum 2 independent security audits before mainnet
- Audit reports published in `docs/security/audits/`
- Focus areas:
  - Backdoor detection
  - Consensus vulnerabilities
  - Cryptographic implementation
  - Network security

### Continuous Monitoring
```bash
# Automated security scanning
cargo audit          # CVE database check
cargo clippy         # Lint for suspicious patterns
cargo deny check     # License and security policy
```

## Whistleblower Protection

### Report Backdoor Suspicions
If you suspect a backdoor exists:

1. **Confidential Report**: security@bitquan.org (PGP: see `docs/security/keys/`)
2. **Public Disclosure**: After 90 days or immediate if no response
3. **Reward**: Up to $100,000 for confirmed backdoors
4. **Legal Protection**: We will not pursue legal action against good-faith reporters

## Enforcement

### Developer Code of Conduct
Any developer who:
- Introduces backdoors intentionally
- Hides functionality
- Obfuscates code without justification
- Violates this policy

**Will be**:
- Immediately removed from project
- Publicly disclosed
- Reported to authorities if criminal

### Commit Signing Requirement
```bash
# All commits must be GPG-signed
git config commit.gpgSign true

# Unsigned commits are rejected
git push
# ❌ ERROR: Commit abc123 is not signed
```

## Verification Checklist

Before each release, we verify:
- [ ] No forbidden keywords in codebase
- [ ] All consensus rules documented
- [ ] Reproducible build successful
- [ ] 3+ independent build attestations
- [ ] Network traffic analysis clean
- [ ] Binary strings analysis clean
- [ ] External audit completed
- [ ] All tests passing
- [ ] Code review by 3+ maintainers

## Open Source Commitment

```text
BitQuan is licensed under Apache-2.0

Every line of code is:
- Public on GitHub
- Reviewable by anyone
- Forkable by anyone
- Auditable by anyone

NO EXCEPTIONS.
```

## Contact

- **Security Team**: security@bitquan.org
- **Transparency Questions**: transparency@bitquan.org
- **Public Forum**: https://github.com/bitquan/bitquan/discussions

## Revision History

- **v1.0.0** (2025-10-25): Initial policy
- **Next Review**: 2026-01-25 (Quarterly review)

---

**This policy is legally binding on all BitQuan contributors and maintainers.**

**Signed**: BitQuan Core Team
**Date**: 2025-10-25
**GPG Signature**: [To be added]
