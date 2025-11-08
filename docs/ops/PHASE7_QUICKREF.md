# Phase 7 Quick Reference Card

**Fast lookup for Phase 7 mainnet workflows, commands, and thresholds.**

---

## 🔧 Workflows

### Audit Report Ingestion
```bash
# Manual dispatch with audit results
gh workflow run audit-report.yml \
  -f status=pass \
  -f tag=v1.0.0-rc1 \
  -f sha=$(git rev-parse HEAD)

# Uploads auditor_report.json → updates badges/audit.svg → commits to repo
```

**Inputs**:
- `status`: `pass` or `fail`
- `tag`: release tag (e.g., `v1.0.0-rc1`)
- `sha`: optional commit SHA

**Artifacts**: `auditor_report.json`, `auditor_diff.md` (30-day retention)

---

### Release Build (Mainnet)
```bash
# Tag triggers automatic multi-platform build
git tag -a v1.0.0 -m "BitQuan Mainnet v1.0.0"
git push origin v1.0.0

# Workflow: .github/workflows/release-mainnet.yml
# Builds: linux-x86_64, linux-aarch64
# Outputs: binaries + SHA256SUMS + cosign attestations
```

**Artifacts**:
- `bitquan-node-v1.0.0-linux-x86_64.tar.gz`
- `bitquan-node-v1.0.0-linux-aarch64.tar.gz`
- `SHA256SUMS`
- `attestation.sig` (OIDC keyless cosign)

---

### Seed Node Deployment
```bash
# Manual dispatch
gh workflow run deploy-seeds.yml \
  -f environment=mainnet \
  -f tag=v1.0.0 \
  -f dry_run=false

# Workflow: .github/workflows/deploy-seeds.yml
# Uses: deploy-cluster.sh + verify-signature.sh
# Outputs: deployment.json (host statuses)
```

**Inputs**:
- `environment`: `testnet` or `mainnet`
- `tag`: release tag
- `dry_run`: `true` (logs only) or `false` (execute)

---

### Preflight Validation
```bash
# CI-friendly (mock mode)
PREFLIGHT_MOCK=1 scripts/preflight/preflight.sh \
  --network mainnet \
  --release-tag v1.0.0-rc1

# Production run (real DNS/RPC checks)
scripts/preflight/preflight.sh \
  --network mainnet \
  --release-tag v1.0.0
```

**Checks**:
- Genesis hash verification
- DNS seed reachability (≥60% threshold)
- Build reproducibility
- RPC security guards (TLS/JWT)
- Metrics availability
- PoW parameters (mainnet = SHA-256d only)

**Output**: `preflight_report.md`, `preflight_raw_logs.txt`

---

## 🧪 Stress Testing

### RPC Load Test
```bash
cargo run -p bq-stress -- rpc-hammer \
  --concurrency 64 \
  --duration 60s \
  --url http://127.0.0.1:28332/rpc

# Outputs: tools/stress/rpc_YYYYMMDD_HHMMSS.json
```

**Metrics**:
- p50/p95/p99 latency
- Request rate (QPS)
- Error rate (4xx/5xx)

**SLO Target**: p95 < 250ms @ 64 concurrency

---

### Pool Share Stress Test
```bash
cargo run -p bq-stress -- pool-shares \
  --miners 200 \
  --qps 50 \
  --duration 300

# Outputs: tools/stress/pool_YYYYMMDD_HHMMSS.json
```

**Metrics**:
- Share submission rate
- Reject rate
- Backpressure counters
- CPU/memory usage

**SLO Targets**:
- Reject rate: < 1.5%
- Backpressure events: logged but non-blocking

---

## 📊 Monitoring & Alerts

### Alert Rules
```bash
# Validate syntax
promtool check rules alerts/mainnet-rules.yml

# Import to Prometheus
# Copy alerts/mainnet-rules.yml to /etc/prometheus/rules/
# Reload: curl -X POST http://localhost:9090/-/reload
```

**Critical Alerts**:
- `HighRPCErrorRate`: 5xx > 1% for 5m
- `BlockProductionStall`: no new block in 3× target interval
- `HeightLag`: local height lags > 2 blocks for 10m
- `HighRejectRate`: stratum reject > 3% for 10m

---

### Grafana Dashboard
```bash
# Import docs/DASHBOARD_MAINNET.json
# URL: https://metrics.bitquan.org
```

**Panels**:
- Chain height gap
- Orphan rate
- RPC p95 latency
- Pool reject rate
- Stratum active miners
- Network peer count

---

## 🔐 Security Gates

### Mainnet Invariants
✅ **PoW Algorithm**: SHA-256d only (RandomX disabled)  
✅ **RPC Auth**: TLS + JWT (no Basic Auth)  
✅ **Rate Limiting**: Enabled (CORS, CSRF, request quotas)  
✅ **Mock PoW**: Forbidden on mainnet (returns error)

### Build Verification
```bash
# Reproducible build check
cargo build --release --locked
sha256sum target/release/bitquan-node

# Compare against SHA256SUMS from release artifacts
```

### DNS Seeds Policy
- Minimum reachable: **60%** of seeds
- Probe timeout: **5 seconds** (TCP handshake)
- Seed list: `genesis/dns_seeds.txt`

---

## 📦 Key Files

| File | Purpose |
|------|---------|
| `docs/AUDIT_HANDOFF_CHECKLIST.md` | Auditor handoff materials |
| `docs/LOAD_TESTING.md` | Stress testing guide |
| `docs/OBSERVABILITY.md` | Monitoring setup |
| `docs/MAINNET_ANNOUNCEMENT.md` | Public launch announcement |
| `alerts/mainnet-rules.yml` | Prometheus alert rules |
| `genesis/dns_seeds.txt` | Mainnet DNS seeds |
| `.github/workflows/audit-report.yml` | Audit ingestion workflow |
| `.github/workflows/release-mainnet.yml` | Release build workflow |
| `.github/workflows/deploy-seeds.yml` | Deployment workflow |
| `.github/workflows/preflight.yml` | Pre-launch validation |

---

## 🚀 Launch Sequence

1. **External Audit**:
   ```bash
   # Send audit checklist
   # Wait for auditor_report.json
   gh workflow run audit-report.yml -f status=pass -f tag=v1.0.0
   ```

2. **Final Preflight**:
   ```bash
   scripts/preflight/preflight.sh --network mainnet --release-tag v1.0.0
   # Must show: Overall Status: ✓ PASS
   ```

3. **Tag & Release**:
   ```bash
   git tag -a v1.0.0 -m "BitQuan Mainnet v1.0.0"
   git push origin v1.0.0
   # → triggers release-mainnet.yml
   ```

4. **Deploy Seeds**:
   ```bash
   gh workflow run deploy-seeds.yml \
     -f environment=mainnet \
     -f tag=v1.0.0 \
     -f dry_run=false
   ```

5. **Enable Alerts**:
   ```bash
   # Import alerts/mainnet-rules.yml to Prometheus
   # Import docs/DASHBOARD_MAINNET.json to Grafana
   ```

6. **Announce**:
   - Publish `docs/MAINNET_ANNOUNCEMENT.md`
   - Update README with mainnet endpoints
   - Notify community channels

---

## 📞 Emergency Contacts

- **Security Issues**: `security@bitquan.org` or [GitHub Security Advisories](https://github.com/AlphaB135/BitQuan/security/advisories)
- **Incident Response**: See `docs/OBSERVABILITY.md` for runbooks
- **Audit Coordination**: See `docs/AUDIT_HANDOFF_CHECKLIST.md`

---

**Phase 7 Quick Reference** — Updated 2025-11-06  
**Full Details**: [PHASE7_COMPLETE.md](./PHASE7_COMPLETE.md)
