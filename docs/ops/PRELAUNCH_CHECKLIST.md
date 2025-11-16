# BitQuan Pre-Launch Validation Checklist

## Overview

This checklist ensures BitQuan mainnet v1.0.0 is ready for production launch. All items must pass before the network goes live.

**Status Legend:**
- ✅ **PASS** - Requirement met, validation successful
- ❌ **FAIL** - Critical issue, blocks launch
- ⚠️ **WARN** - Non-critical issue, review required
- 🔄 **PENDING** - Not yet validated

## Quick Start

### Local Validation
```bash
# Full preflight check for mainnet
scripts/preflight/preflight.sh --network mainnet --release-tag v1.0.0-rc1

# Full preflight check for testnet
scripts/preflight/preflight.sh --network testnet --release-tag v1.0.0-rc1

# View generated report
cat preflight_report.md
```

### CI Validation
```bash
# Trigger GitHub Actions workflow
gh workflow run preflight.yml -f network=mainnet -f release_tag=v1.0.0-rc1

# Check workflow status
gh run list --workflow=preflight.yml --limit 5
```

## Validation Matrix

### 1. Genesis Configuration

| Check | Requirement | Status | Notes |
|-------|-------------|--------|-------|
| Genesis hash match | Hash matches `docs/GENESIS.md` | 🔄 | Critical |
| Genesis file format | Valid JSON, all required fields | 🔄 | Critical |
| Network ID | `mainnet` for production | 🔄 | Critical |
| Chain ID | `bitquan-mainnet-v1` | 🔄 | Critical |
| Genesis timestamp | Valid Unix timestamp | 🔄 | Required |
| PQC signature | Dilithium3 signature valid | 🔄 | Required |

**Validation Command:**
```bash
scripts/preflight/check_genesis_hash.sh mainnet v1.0.0
```

**Pass Criteria:** Exit code 0, hash matches documented value exactly

---

### 2. DNS Bootstrap & Network Seeds

| Check | Requirement | Status | Notes |
|-------|-------------|--------|-------|
| DNS resolution | All seeds resolve to IPs | 🔄 | Required |
| TCP connectivity | Seeds respond on P2P port | 🔄 | Required |
| Reachability threshold | ≥ 60% seeds reachable | 🔄 | Critical |
| Seed diversity | Geographic distribution | 🔄 | Recommended |
| IPv4/IPv6 support | Both address families work | 🔄 | Recommended |

**Validation Command:**
```bash
scripts/preflight/check_dns_seeds.sh mainnet v1.0.0
```

**Pass Criteria:**
- Minimum 60% of seeds reachable
- At least 3 seeds operational
- Response time < 2 seconds per seed

---

### 3. Build Reproducibility

| Check | Requirement | Status | Notes |
|-------|-------------|--------|-------|
| Deterministic build | Two builds produce identical binary | 🔄 | Critical |
| SHA256 match | Local hash matches release artifact | 🔄 | Critical |
| Toolchain version | Rust toolchain locked | 🔄 | Required |
| Dependencies locked | Cargo.lock committed | 🔄 | Required |
| Container build | Docker build reproducible | 🔄 | Recommended |

**Validation Command:**
```bash
scripts/preflight/check_build_repro.sh mainnet v1.0.0
```

**Pass Criteria:**
- SHA256 hashes match between builds
- Release artifacts available on GitHub
- Build completes without warnings

---

### 4. RPC Security Guards

| Check | Requirement | Status | Notes |
|-------|-------------|--------|-------|
| Authentication | Unauthenticated /rpc → 401 | 🔄 | Critical |
| Request timeout | Slow requests → 408 | 🔄 | Required |
| Rate limiting | Flood requests → 429 | 🔄 | Critical |
| Header size limit | Large headers → 431 | 🔄 | Required |
| Health endpoint | /health accessible (200) | 🔄 | Required |
| Retry-After header | 429 includes Retry-After | 🔄 | Required |

**Validation Command:**
```bash
scripts/preflight/check_rpc_security.sh mainnet v1.0.0
```

**Pass Criteria:**
- All HTTP status codes correct
- No unauthenticated access to protected endpoints
- Rate limits enforced

---

### 5. Metrics & Observability

| Check | Requirement | Status | Notes |
|-------|-------------|--------|-------|
| Metrics endpoint | /metrics returns Prometheus format | 🔄 | Required |
| Network metrics | `network_peers_*` present | 🔄 | Required |
| Chain metrics | `chain_finalized_height` present | 🔄 | Required |
| RPC metrics | `rpc_requests_total` present | 🔄 | Required |
| Mining metrics | `stratum_*` present (if enabled) | 🔄 | Conditional |
| Format validation | Valid Prometheus exposition format | 🔄 | Required |

**Validation Command:**
```bash
scripts/preflight/check_metrics.sh mainnet v1.0.0
```

**Pass Criteria:**
- All required metric keys present
- No parsing errors
- Metrics update dynamically

**Required Metrics:**
```
network_peers_mainnet_total
chain_finalized_height
chain_tip_height
rpc_requests_total
rpc_errors_total
mempool_size
block_processing_duration_seconds
```

---

### 6. PoW Parameters

| Check | Requirement | Status | Notes |
|-------|-------------|--------|-------|
| Mainnet algo lock | SHA-256d from genesis, hybrid at block 10,000 | 🔄 | Critical |
| Testnet hybrid | Hybrid allowed if configured | 🔄 | Required |
| ASERT params | Half-life and targets correct | 🔄 | Critical |
| Difficulty bits | Min/max values within spec | 🔄 | Required |
| Target block time | 600s for mainnet | 🔄 | Critical |
| Adjustment interval | 2016 blocks for mainnet | 🔄 | Critical |

**Validation Command:**
```bash
scripts/preflight/check_pow_params.sh mainnet v1.0.0
```

**Pass Criteria:**
- Mainnet: `pow_algo = "sha256d"` only
- Mainnet: Hybrid disabled in code
- Parameters match specification

**Mainnet Parameters:**
```
pow_algo: sha256d
target_block_time: 600
difficulty_adjustment_interval: 2016
min_difficulty_bits: 486604799
max_difficulty_bits: 503382015
```

---

### 7. TLS & JWT Configuration

| Check | Requirement | Status | Notes |
|-------|-------------|--------|-------|
| TLS handshake | Successful connection | 🔄 | Required |
| JWT validation | Invalid token → 401 | 🔄 | Critical |
| JWT metrics | Token validation counted | 🔄 | Required |
| HSTS header | Strict-Transport-Security present | 🔄 | Required |
| CSP header | Content-Security-Policy present | 🔄 | Recommended |
| Certificate validity | Valid cert (not self-signed) | 🔄 | Production |

**Validation Command:**
```bash
scripts/preflight/check_tls_jwt.sh mainnet v1.0.0
```

**Pass Criteria:**
- TLS 1.2+ supported
- JWT tokens validated correctly
- Security headers present

**Required Headers:**
```
Strict-Transport-Security: max-age=31536000; includeSubDomains
Content-Security-Policy: default-src 'self'
X-Content-Type-Options: nosniff
X-Frame-Options: DENY
```

---

## Overall Pass Criteria Summary

### Critical Requirements (Must Pass)
1. ✅ Genesis hash matches documented value
2. ✅ DNS seeds reachable ≥ 60%
3. ✅ Reproducible build hash verified
4. ✅ RPC authentication enforced (401 for unauth)
5. ✅ Rate limiting active (429 with Retry-After)
6. ✅ Mainnet PoW locked to SHA-256d
7. ✅ JWT validation working

### Required (Should Pass)
1. ✅ Metrics endpoints expose required keys
2. ✅ Request timeouts configured (408)
3. ✅ Header size limits enforced (431)
4. ✅ TLS handshake successful
5. ✅ Security headers present

### Recommended (Best Effort)
1. ⚠️ Seed geographic diversity
2. ⚠️ IPv6 support
3. ⚠️ Container build reproducibility
4. ⚠️ CSP headers

## Automation

### GitHub Actions
The preflight validation runs automatically on:
- All release tags (`v*.*.*`, `v*.*.*-rc*`)
- Pull requests affecting preflight scripts
- Manual workflow dispatch

### Artifacts
CI workflow uploads:
- `preflight_report.md` - Summary table
- `preflight_raw_logs.txt` - Detailed logs
- Retention: 30 days

### Failure Handling
If any **Critical** check fails:
- CI job fails (red X)
- Release is blocked
- Team is notified
- Issue must be resolved before retry

## Manual Review Checklist

Before final mainnet launch, manually verify:

- [ ] All automated checks passed
- [ ] Security audit complete (see `AUDIT_SUMMARY.md`)
- [ ] Bug bounty program active
- [ ] Documentation complete and reviewed
- [ ] Reproducible build verified by independent party
- [ ] Testnet running stably for ≥ 30 days
- [ ] Community notification sent (7 days advance)
- [ ] Exchange partnerships confirmed
- [ ] Mining pools ready
- [ ] Block explorer operational
- [ ] Wallet releases available (desktop, mobile, CLI)
- [ ] Backup infrastructure tested (failover)
- [ ] Incident response team on standby
- [ ] Post-launch monitoring configured

## Command Reference

### Local Validation
```bash
# Run individual checks
scripts/preflight/check_genesis_hash.sh mainnet v1.0.0
scripts/preflight/check_dns_seeds.sh mainnet v1.0.0
scripts/preflight/check_build_repro.sh mainnet v1.0.0
scripts/preflight/check_rpc_security.sh mainnet v1.0.0
scripts/preflight/check_metrics.sh mainnet v1.0.0
scripts/preflight/check_pow_params.sh mainnet v1.0.0
scripts/preflight/check_tls_jwt.sh mainnet v1.0.0

# Run all checks
scripts/preflight/preflight.sh --network mainnet --release-tag v1.0.0
```

### CI Validation
```bash
# Manual trigger
gh workflow run preflight.yml -f network=mainnet -f release_tag=v1.0.0-rc1

# View latest run
gh run view --workflow=preflight.yml

# Download artifacts
gh run download <run-id> -n preflight-report-mainnet
```

## Troubleshooting

### Genesis Hash Mismatch
```bash
# Regenerate genesis file
scripts/generate_genesis.sh mainnet

# Update docs/GENESIS.md with new hash
```

### DNS Seeds Unreachable
```bash
# Test individual seed
dig seed1.bitquan.network +short
nc -zv seed1.bitquan.network 8333 -w 2

# Use bq-preflight tool
./target/release/bq-preflight dns-check --network mainnet --timeout 2
```

### Build Not Reproducible
```bash
# Clean build
cargo clean

# Rebuild with locked dependencies
cargo build --release --locked

# Check for non-determinism
find target/release -name "bitquan-node" -exec sha256sum {} \;
```

### RPC Security Checks Failing
```bash
# Start node with correct config
bitquan-node --config config/mainnet.toml

# Test endpoints manually
curl -v http://localhost:8545/health
curl -v http://localhost:8545/rpc  # Should get 401
```

## References

- [Genesis Documentation](../concepts/GENESIS.md)
- [Security Policy](../SECURITY.md)
- [Metrics Guide](../METRICS.md)
- [Observability](OBSERVABILITY.md)
- [Runbook](RUNBOOK.md)
- [Audit Summary](../security/AUDIT_SUMMARY.md)

## Contact

For preflight validation issues:
- **GitHub Issues:** Label with `preflight` and `phase-6.5`
- **CI/CD Support:** Check `.github/workflows/preflight.yml`
- **Security Concerns:** security@bitquan.org

---

*Last Updated: November 2024*  
*Phase: 6.5 - Pre-Launch Validation*  
*Document Version: 1.0.0*
