# Phase 7 Quick Reference Card

**Version:** v1.0.0-rc1
**Status:** ✅ Production-ready
**Date:** 2025-11-07

---

## 🎯 Quick Launch Checklist

### Pre-Launch
- [ ] Run `cargo fmt --all --check`
- [ ] Run `cargo clippy -D warnings`
- [ ] Run `cargo test --all --locked`
- [ ] Verify `scripts/alerts/lint.sh` passes
- [ ] Run stress tests (see below)

### External Audit
- [ ] Send `docs/AUDIT_HANDOFF_CHECKLIST.md` to auditor
- [ ] Receive audit artifacts
- [ ] Run: `gh workflow run audit-report.yml -f status=pass -f tag=v1.0.0`

### Release
- [ ] Tag: `git tag -s v1.0.0 -m "Mainnet v1.0.0"`
- [ ] Push: `git push origin v1.0.0`
- [ ] Verify `release-mainnet.yml` workflow completes
- [ ] Download and verify SHA256SUMS

### Deploy
- [ ] Dry-run: `gh workflow run deploy-seeds.yml -f dry_run=true`
- [ ] Deploy: `gh workflow run deploy-seeds.yml -f environment=mainnet -f tag=v1.0.0 -f dry_run=false`
- [ ] Verify DNS seeds: `cargo run -p bq-preflight -- dns-check --dns-seed-threshold 60`

### Monitor
- [ ] Import `alerts/mainnet-rules.yml` to Prometheus
- [ ] Import Grafana dashboard from `deploy/grafana-mainnet-dashboard.json`
- [ ] Verify alerts firing correctly
- [ ] Check metrics at https://metrics.bitquan.org

### Announce
- [ ] Publish `docs/MAINNET_ANNOUNCEMENT.md`
- [ ] Update README with live URLs
- [ ] Post to social media

---

## 🔧 Essential Commands

### Stress Testing
```bash
# RPC latency test
cargo run -p bq-stress -- rpc-hammer --concurrency 64 --duration 120 --url http://localhost:8332

# Pool share test
cargo run -p bq-stress -- pool-shares --miners 200 --qps 50 --duration 300
```

### Preflight Validation
```bash
# DNS seeds check
cargo run -p bq-preflight -- dns-check --network mainnet --dns-seed-threshold 60

# Full preflight (CI mode)
PREFLIGHT_MOCK=1 scripts/preflight/preflight.sh --network mainnet --release-tag v1.0.0
```

### Alert Rules
```bash
# Validate rules
bash scripts/alerts/lint.sh alerts/mainnet-rules.yml

# Import to Prometheus
cp alerts/mainnet-rules.yml /etc/prometheus/rules/ && systemctl reload prometheus
```

---

## 📁 Key Files

| Component | Location |
|-----------|----------|
| Audit workflow | `.github/workflows/audit-report.yml` |
| Release workflow | `.github/workflows/release-mainnet.yml` |
| Deploy workflow | `.github/workflows/deploy-seeds.yml` |
| Stress tool | `crates/tools/stress/` |
| Preflight tool | `crates/tools/preflight/` |
| Alert rules | `alerts/mainnet-rules.yml` |
| DNS seeds | `genesis/dns_seeds.txt` |
| Announcement | `docs/MAINNET_ANNOUNCEMENT.md` |
| Complete guide | `PHASE7_COMPLETE.md` |

---

## 🎯 SLO Targets

| Metric | Target | Measurement |
|--------|--------|-------------|
| RPC p95 | ≤ 250ms | `bq-stress rpc-hammer --concurrency 64` |
| Share reject | < 1.5% | `bq-stress pool-shares` + metrics |
| Orphan rate | < 1% | Prometheus: `rate(bitquan_orphan_blocks_total[30m])` |
| DNS reachability | ≥ 60% | `bq-preflight dns-check` |

---

## 🔐 Security Checklist

- ✅ Mainnet = SHA-256d only (no RandomX)
- ✅ RPC: TLS + JWT enforced
- ✅ Rate limiting: 100 req/min
- ✅ No unsafe code in consensus/crypto
- ✅ Reproducible builds (`--locked`)
- ✅ All workflows validated

---

**For detailed instructions, see:** `PHASE7_COMPLETE.md`
