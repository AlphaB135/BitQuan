# BitQuan Security Policy

## Reporting a Vulnerability

### Primary Contact Methods

**1. GitHub Security Advisories (RECOMMENDED)**
- Report via: https://github.com/AlphaB135/BitQuan/security/advisories
- This is the **preferred method** for vulnerability reports
- Provides secure, private communication with maintainers
- Automatic tracking and notification to core team

**2. Email**
- Email: `atsadawut.02@icloud.com`
- Include "SECURITY:" in subject line
- **Note**: Personal contact for BitQuan project
- If no response within 48 hours, please use GitHub Security Advisories

**3. Emergency Contact**
- For critical vulnerabilities only, page the on-call maintainer via the emergency contact listed in `MAINTAINERS`

### Response SLA
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

## Network Layer Security

### Slowloris Attack Protection

**Vulnerability:** Attackers send data very slowly (1 byte every 29 minutes)
to exhaust server resources.

**Our Protection:**
- `tokio::time::timeout` wraps entire message read
- Timeout does NOT reset on partial reads
- 30-second total limit enforced
- Connections auto-closed if exceeded

**Test:**
```bash
python tools/test_slowloris.py --target localhost:8333
# Expected: All slow connections closed after 30s
```

### Resource Limits

- Max peers: 100 (configurable)
- Max connections: 100 (configurable)
- Timeout per message: 30 seconds
- Memory per peer: ~4KB (async tasks)

### Network Testing Tools

**Slowloris Simulation:**
```bash
# Test Slowloris protection
python tools/test_slowloris.py --port 18444 --connections 100
```

**Load Testing:**
```bash
# Test connection handling capacity
python tools/load_test.py --connections 1000 --duration 10
```

**Expected Results:**
- Slowloris: 100% connections closed after timeout
- Load Test: <100MB RAM for 1000 connections (vs 8GB with sync)

## Bounty Program
- Rewards scaled by impact and quality of report, distributed transparently
- Payments occur after patch release and verification by the security team
- Program terms published publicly and updated via BQIP when needed

## Contact

### Security Disclosure Channels (in order of preference)

1. **GitHub Security Advisories** (RECOMMENDED)
   - URL: https://github.com/AlphaB135/BitQuan/security/advisories
   - Private, secure, tracked
   - Automatically notifies core maintainers

2. **Email**
   - Address: `atsadawut.02@icloud.com`
   - Include "SECURITY:" in subject line
   - **Note**: Personal contact for BitQuan project
   - **Fallback**: If no response within 48 hours, use GitHub Security Advisories

3. **Emergency**
   - See `MAINTAINERS` file for on-call contact information
   - For **critical** vulnerabilities only (active exploits, chain halting conditions)

### For Non-Security Questions
- General inquiries: Use GitHub Issues or Discussions
- Development questions: See `CONTRIBUTING.md`
- Documentation issues: Open a pull request

## Cryptographic Lifespan

BitQuan's post-quantum cryptography is designed for **50+ year security resilience** against both classical and quantum computers.

### Algorithm Security Estimates

| Algorithm | Type | NIST Standard | Classical Security | Quantum Security | Expected Lifespan |
|-----------|------|---------------|-------------------|------------------|-------------------|
| **Dilithium3** | Signatures | FIPS 204 | 192-bit | NIST Level 3 (≈AES-192) | 50+ years |
| **SHA-256** | Hashing | FIPS 180-4 | 256-bit | 128-bit (Grover) | 30+ years |
| **AES-256-GCM** | Encryption | NIST approved | 256-bit | 128-bit (Grover) | 30+ years |

### Security Assumptions

1. **Quantum Threat Model**: Based on NIST PQC standardization estimates
   - Large-scale quantum computers expected by 2030-2040
   - Dilithium3 provides security margin beyond current projections

2. **Conservative Design**: Chosen NIST Level 3 (not Level 1) for extra margin
   - Level 3 ≈ AES-192 equivalent security
   - Balances security vs. signature size (3,293 bytes)

3. **Upgrade Path**: Protocol designed for future algorithm updates
   - Version field in transactions allows smooth transitions
   - Hybrid schemes possible if needed (Dilithium + classical ECDSA)

### Cryptographic Audit Trail

- **Dilithium**: NIST FIPS 204 (2023) — Final PQC signature standard
- **SHA-256**: NIST FIPS 180-4 (2015) — Industry-proven hash function
- **AES-GCM**: NIST SP 800-38D (2007) — Authenticated encryption standard
- **HMAC-SHA512**: NIST FIPS 198-1 (2008) — Key derivation standard
- **ChaCha20**: RFC 8439 (2018) — CSPRNG for key generation

**Recommendation**: External cryptographic audit required before mainnet launch to validate implementation correctness.

## Sprint 3 Security Enhancements (November 2024)

### Error Handling Audit
All production code has been audited for unsafe error handling patterns:

| Crate | unwrap/expect in Prod | Status | Notes |
|-------|----------------------|--------|-------|
| node | 3 | [DONE] Justified | HRP encoding constants (compile-time validated) |
| consensus | 0 | [DONE] Clean | Test-only panics |
| wallet | 0 | [DONE] Clean | Test-only unwraps |
| mempool | 1 | [DONE] Justified | Default trait limitation documented |

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
1. [DONE] All RNG uses OsRng (cryptographically secure)
2. [DONE] Cross-network replay protection via network_id + genesis_hash
3. [DONE] Post-quantum signatures (Dilithium3)
4. [DONE] Zero clippy warnings in production code
5. [DONE] Comprehensive error handling (no unwrap/expect in critical paths)
6. [DONE] TLS 1.3 encryption for all RPC communications
7. [DONE] JWT-based authentication with Argon2id password hashing

## RPC Security Architecture

### TLS/HTTPS Configuration

BitQuan RPC server supports full TLS 1.3 encryption using `rustls`:

**Production (Mainnet) Requirements**:
- TLS is **mandatory** (`--rpc-tls-cert` and `--rpc-tls-key` required)
- Self-signed certificates are **rejected** (must use CA-signed certificates)
- HSTS enabled with 1-year max-age
- HTTP/2 and HTTP/1.1 ALPN support

**Development (Testnet/Devnet)**:
- TLS is optional but recommended
- Self-signed certificates allowed via `--rpc-allow-insecure`
- Can generate dev certificates: `bitquan-node generate-cert --output ./certs`

**Example Production Setup**:
```bash
# Mainnet with Let's Encrypt certificate
bitquan-node p2p-server \
  --rpc-listen 0.0.0.0:8332 \
  --rpc-tls-cert /etc/letsencrypt/live/node.example.com/fullchain.pem \
  --rpc-tls-key /etc/letsencrypt/live/node.example.com/privkey.pem \
  --jwt-config jwt.toml
```

**Example Development Setup**:
```bash
# Generate self-signed certificate for development
bitquan-node generate-cert --output ./certs

# Start node with self-signed cert
bitquan-node p2p-server \
  --rpc-listen 127.0.0.1:8332 \
  --rpc-tls-cert ./certs/cert.pem \
  --rpc-tls-key ./certs/key.pem \
  --rpc-allow-insecure \
  --jwt-secret "dev-secret-change-in-production"
```

### JWT Authentication

BitQuan uses **JWT (JSON Web Tokens)** for RPC authentication, replacing traditional Basic Auth:

**Key Features**:
- Argon2id password hashing (resistant to GPU cracking)
- Access tokens (1 hour expiry) and refresh tokens (7 days)
- Role-based access control (admin, miner, readonly)
- No credentials sent with each request (token-based)

**Configuration via jwt.toml**:
```toml
secret = "CHANGE_THIS_TO_A_STRONG_RANDOM_SECRET"

[[users]]
username = "admin"
password_hash = "$argon2id$v=19$m=19456,t=2,p=1$..." # Use 'bitquan-node hash-password'
role = "admin"

[[users]]
username = "miner"
password_hash = "$argon2id$v=19$m=19456,t=2,p=1$..."
role = "miner"
```

**User Management**:
```bash
# Hash a password
bitquan-node hash-password

# Add a user
bitquan-node jwt-user-add --config jwt.toml --username alice --role admin

# List users
bitquan-node jwt-user-list --config jwt.toml

# Remove a user
bitquan-node jwt-user-remove --config jwt.toml --username alice
```

**Client Usage**:
```bash
# 1. Login to get JWT token
curl -X POST https://node.example.com:8332/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","password":"yourpass"}'

# Response: {"access_token":"eyJ...","token_type":"Bearer","expires_in":3600}

# 2. Use token for RPC calls
curl -X POST https://node.example.com:8332/rpc \
  -H "Authorization: Bearer eyJ..." \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"getblockcount","params":[],"id":1}'

# 3. Refresh token when it expires
curl -X POST https://node.example.com:8332/auth/refresh \
  -H "Content-Type: application/json" \
  -d '{"refresh_token":"eyJ..."}'
```

### Security Headers

All RPC responses include hardened security headers:

```
Strict-Transport-Security: max-age=63072000; includeSubDomains; preload
X-Content-Type-Options: nosniff
X-Frame-Options: DENY
Referrer-Policy: no-referrer
Content-Security-Policy: default-src 'none'
```

### Additional RPC Security Features

- **Rate Limiting**: Token bucket per IP (configurable via `--rpc-rl-burst` and `--rpc-rl-refill-per-sec`)
- **DNS Rebinding Protection**: Host header validation (`--rpc-allowed-hosts`)
- **CORS Protection**: Origin header validation (`--rpc-allowed-origins`)
- **Request Size Limits**: Body (1 MiB default) and header (8 KiB default) size limits
- **Connection Timeouts**: Header read, body read, and idle connection timeouts
- **Proxy Support**: X-Forwarded-For with trusted proxy CIDRs

### Testing

Comprehensive test coverage for security features:

```bash
# Run all RPC tests
cargo test -p bitquan-rpc

# JWT authentication tests
cargo test -p bitquan-rpc --test jwt_simple_test

# TLS enforcement tests
cargo test -p bitquan-rpc --test tls_enforcement_tests
```

### Security Metrics (Prometheus)

RPC server exposes security metrics at `/metrics` (localhost only):

```
rpc_requests_total                # Total requests
rpc_requests_status_total{code}   # HTTP status codes
rpc_auth_401_total                # Failed authentication attempts
rpc_rl_drops_total                # Rate limited requests
rpc_latency_ms_bucket{le}         # Request latency histogram
```

### Audit Trail

All security-relevant events are logged in JSON format:

```json
{
  "event": "rpc_guard",
  "reason": "auth_failure",
  "status": 401,
  "method": "POST",
  "route": "/rpc",
  "ip": "192.168.1.100",
  "bytes": 256,
  "latency_ms": 42
}
```

## Pre-Launch Security Validation

### TLS/JWT Preflight Checks

Before mainnet launch, TLS and JWT configurations must pass automated preflight validation:

**Validation Script**: `scripts/preflight/check_tls_jwt.sh`

**Checks Performed**:
1. **TLS Handshake**: Verify successful TLS 1.2+ connection
2. **JWT Token Validation**: Ensure invalid tokens return 401
3. **Metrics Tracking**: Confirm JWT validation events are counted
4. **Security Headers**: Validate presence of required headers:
   - `Strict-Transport-Security`
   - `Content-Security-Policy`
   - `X-Content-Type-Options`
   - `X-Frame-Options`

**Running Preflight**:
```bash
# Individual TLS/JWT check
scripts/preflight/check_tls_jwt.sh mainnet v1.0.0

# Full preflight validation
scripts/preflight/preflight.sh --network mainnet --release-tag v1.0.0
```

**Pass Criteria**:
- [DONE] TLS handshake completes successfully
- [DONE] Invalid JWT tokens rejected (401)
- [DONE] Metrics counter increments for validation events
- [DONE] All required security headers present
- [DONE] HSTS max-age ≥ 31536000 (1 year)
- [DONE] CSP header restricts content sources

**CI Integration**:
Preflight validation runs automatically in `.github/workflows/preflight.yml` on:
- Release tag pushes (`v*.*.*`)
- Pull requests affecting RPC or security code
- Manual workflow dispatch

**Mock Mode**:
For CI environments without actual TLS setup, preflight supports mock mode:
```bash
PREFLIGHT_MOCK=1 scripts/preflight/check_tls_jwt.sh mainnet v1.0.0
```

See [docs/PRELAUNCH_CHECKLIST.md](docs/PRELAUNCH_CHECKLIST.md) for complete validation requirements.

### Continuous Security

- Dependencies are checked on every PR via `cargo-deny`.
- Nightly `cargo audit` runs against the RustSec database.
- Production crates enable `-D clippy::unwrap_used` and CI fails on warnings.
- Consensus/crypto paths are covered by `nextest` and line-coverage gate (≥80%).
