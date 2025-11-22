# Phase 6.5 Implementation Guide

## Quick Reference

### Files Created/Modified Summary

#### Created Files (14 files)

**Preflight Scripts (8 files):**
1. `/scripts/preflight/preflight.sh` - Master orchestration script
2. `/scripts/preflight/check_genesis_hash.sh` - Genesis hash verification
3. `/scripts/preflight/check_dns_seeds.sh` - DNS seed reachability
4. `/scripts/preflight/check_build_repro.sh` - Build reproducibility
5. `/scripts/preflight/check_rpc_security.sh` - RPC security guards
6. `/scripts/preflight/check_metrics.sh` - Metrics validation
7. `/scripts/preflight/check_pow_params.sh` - PoW parameter checks
8. `/scripts/preflight/check_tls_jwt.sh` - TLS/JWT configuration

**Rust Crate (2 files):**
9. `/crates/tools/preflight/Cargo.toml` - Preflight tool manifest
10. `/crates/tools/preflight/src/main.rs` - DNS/TCP probing binary

**Tests (1 file):**
11. `/tests/preflight_validation.rs` - 12 integration tests

**Documentation (3 files):**
12. `/docs/GENESIS.md` - Genesis block documentation
13. `/docs/PRELAUNCH_CHECKLIST.md` - Complete validation checklist
14. `/PHASE6.5_COMPLETE.md` - Implementation completion summary

**Modified Files (3 files):**
1. `/README.md` - Added Pre-Launch Validation section
2. `/SECURITY.md` - Added TLS/JWT preflight documentation
3. `/.github/workflows/preflight.yml` - Added CI workflow (if not existing)

---

## Tests Added Summary

### Unit Tests (2 tests in bq-preflight)

**Location:** `crates/tools/preflight/src/main.rs`

1. `test_tcp_probe_localhost` - Tests TCP connection logic
2. `test_find_project_root` - Tests project root detection

**Run:** `cargo test -p bq-preflight`

### Integration Tests (12 tests)

**Location:** `tests/preflight_validation.rs`

**Genesis Tests (3):**
1. `test_genesis_verify_mainnet_ok` - Mainnet genesis validation
2. `test_genesis_verify_testnet_ok` - Testnet genesis validation
3. `test_genesis_consensus_params` - Consensus parameter validation

**DNS Bootstrap Tests (3):**
4. `test_dns_seeds_file_exists` - DNS seeds file presence
5. `test_dns_seeds_format` - Format validation (domain:port)
6. `test_dns_bootstrap_min_threshold` - Threshold logic (60%)

**RPC Security Tests (2):**
7. `test_rpc_guard_matrix` - HTTP status codes (401/408/429/431)
8. `test_rpc_retry_after_header` - Retry-After header validation

**Metrics Tests (2):**
9. `test_metrics_key_presence` - Required metrics keys
10. `test_metrics_prometheus_format` - Prometheus format validation

**PoW Parameters Tests (2):**
11. `test_pow_param_matrix` - Mainnet PoW lock (SHA-256d only)
12. `test_pow_target_block_time` - Block time verification (600s)

**Run:** Tests are in workspace, run with `cargo test`

---

## How to Read preflight_report.md

### Overview

The `preflight_report.md` file is the primary output of the preflight validation system. It provides a single-page summary of all pre-launch checks.

### File Structure

```markdown
# BitQuan Pre-Launch Preflight Report

**Network:** [mainnet|testnet]
**Release Tag:** [version]
**Generated:** [UTC timestamp]
**Overall Status:** [✓ PASS | ✗ FAIL]

---

## Summary

[Status table with all checks]

---

## Pass Criteria

[List of requirements]

---

## Notes

[Command references]
```

### Reading the Summary Table

**Table Format:**
```
| Check                      | Status      | Details                    |
|----------------------------|-------------|----------------------------|
| Genesis Hash Verification  | ✅ PASS     | Hash verified: 00000...    |
| DNS Seeds Reachability     | ✅ PASS     | Reachable: 4/5 (80% ≥ 60%) |
| Build Reproducibility      | ❌ FAIL     | SHA256 mismatch            |
```

**Status Icons:**
- ✅ **PASS** - Check succeeded, safe to proceed
- ❌ **FAIL** - Check failed, blocks release

**Details Column:**
- Provides specific information about the check
- Success: Shows verification details (hash, percentage, etc.)
- Failure: Shows error reason

### Interpreting Results

#### All Checks Pass (✅)

**What it means:**
- All critical requirements met
- Safe to proceed with release
- CI will succeed with exit code 0

**Next steps:**
1. Review the details for any warnings
2. Verify artifacts are uploaded
3. Proceed with release process

#### Any Check Fails (❌)

**What it means:**
- Critical requirement not met
- Release is blocked
- CI will fail with exit code 1

**Next steps:**
1. Identify which check failed (table shows ❌)
2. Read the Details column for error reason
3. Check `preflight_raw_logs.txt` for full output
4. Fix the issue
5. Re-run preflight validation

### Example Scenarios

#### Scenario 1: Genesis Hash Mismatch

```
| Genesis Hash Verification | ❌ FAIL | Hash mismatch: expected 00000..., got 11111... |
```

**Action:**
- Update `docs/GENESIS.md` with correct hash
- OR regenerate genesis file if incorrect

#### Scenario 2: DNS Seeds Unreachable

```
| DNS Seeds Reachability | ❌ FAIL | Reachable: 2/5 (40% < 60%) |
```

**Action:**
- Check DNS seed infrastructure
- Verify seeds are operational
- Ensure network connectivity
- May need to add more seeds

#### Scenario 3: RPC Guards Not Working

```
| RPC Security Guards | ❌ FAIL | Guards validated: 2/6 (minimum 4 required) |
```

**Action:**
- Verify RPC server is running
- Check security configuration
- Test endpoints manually with curl
- Review RPC guard implementation

### Using Raw Logs

**File:** `preflight_raw_logs.txt`

**Structure:**
```
BitQuan Preflight Raw Logs
Network: mainnet | Tag: v1.0.0
Started: 2024-11-06 10:00:00 UTC
=========================================

---[ Genesis Hash Verification ]---
[Full command output]

---[ DNS Seeds Reachability ]---
[Full command output]

[... more checks ...]
```

**When to use:**
- Debugging failures
- Understanding why a check failed
- Viewing full command output
- Troubleshooting mock vs production differences

**How to use:**
1. Find the failing check section (marked with `---[ Check Name ]---`)
2. Read the full command output
3. Look for error messages or unexpected behavior
4. Use information to fix the issue

### Mock Mode vs Production Mode

#### Mock Mode Indicators

Look for "Mock mode:" in the Details column:

```
| DNS Seeds Reachability | ✅ PASS | Mock mode: Reachable: 4/5 (80% >= 60%) |
```

**Characteristics:**
- No actual network calls
- Simulated success/failure
- Fast execution (~5 seconds)
- Used in CI

**When to use:**
- CI/CD pipelines
- Offline development
- Quick validation of structure

#### Production Mode Indicators

No "Mock mode:" prefix in Details:

```
| DNS Seeds Reachability | ✅ PASS | Reachable: 4/5 (80% >= 60%) |
```

**Characteristics:**
- Real network calls to DNS seeds
- Actual RPC endpoint tests
- Slower execution (~30-60 seconds)
- Reflects true production readiness

**When to use:**
- Final pre-launch validation
- After infrastructure is deployed
- Before tagging release
- When testing actual deployment

### Pass Criteria Section

The report includes a checklist of all requirements:

```markdown
## Pass Criteria

- ✓ Genesis hash matches documented value
- ✓ DNS seeds reachable ≥ 60%
- ✓ RPC guards active (401/408/429/431)
- ✓ Metrics endpoints expose required keys
- ✓ PoW parameters locked for mainnet
- ✓ Reproducible build hash verified
- ✓ TLS/JWT security headers present
```

**All items must have ✓** for mainnet launch approval.

### Notes Section

The report includes helpful commands:

```markdown
## Notes

- Raw logs available at: `preflight_raw_logs.txt`
- For local validation: `scripts/preflight/preflight.sh --network mainnet --release-tag v1.0.0`
- For CI validation: `gh workflow run preflight.yml -f network=mainnet -f release_tag=v1.0.0`
```

**Use these commands to:**
- Re-run validation locally
- Trigger CI validation
- Access detailed logs

### Report Location

**Local:**
- File: `./preflight_report.md` (project root)
- Generated after running `scripts/preflight/preflight.sh`

**CI:**
- Artifact: `preflight-report-mainnet` or `preflight-report-testnet`
- Download: `gh run download <run-id>`
- Retention: 30 days

### Quick Decision Matrix

| Overall Status | All Checks | Action                          |
|----------------|------------|---------------------------------|
| ✓ PASS         | ✅ PASS    | Proceed with release            |
| ✗ FAIL         | ≥1 ❌ FAIL | Fix issues, re-run validation   |
| N/A            | Mock mode  | Run production mode for final   |

### Troubleshooting Tips

**Problem:** Report shows all PASS but you suspect issues

**Solution:**
- Check if running in mock mode
- Run in production mode without `PREFLIGHT_MOCK=1`
- Verify actual infrastructure is deployed

**Problem:** Can't find preflight_report.md

**Solution:**
- Check current directory (project root)
- Ensure script ran successfully (exit code 0)
- Look for errors in terminal output

**Problem:** Details column says "No details"

**Solution:**
- Check `preflight_raw_logs.txt` for full output
- Script may have encountered error before details
- Run individual check script manually for debugging

### Integration with Release Process

1. **Before tagging release:**
   ```bash
   scripts/preflight/preflight.sh --network mainnet --release-tag v1.0.0
   ```

2. **Review report:**
   - All checks must show ✅ PASS
   - No warnings in details
   - Production mode (not mock)

3. **Tag release:**
   ```bash
   git tag -s v1.0.0 -m "Release v1.0.0"
   git push origin v1.0.0
   ```

4. **CI runs automatically:**
   - Workflow triggered on tag push
   - Generates report artifact
   - Blocks if any check fails

5. **Download CI report:**
   ```bash
   gh run list --workflow=preflight.yml
   gh run download <run-id> -n preflight-report-mainnet
   cat preflight_report.md
   ```

### Best Practices

**✅ DO:**
- Run preflight before every release
- Review report carefully, even if PASS
- Keep reports for audit trail
- Run production mode for final validation
- Address warnings even if checks pass

**❌ DON'T:**
- Ignore failures and proceed with release
- Rely only on mock mode for production
- Skip preflight for "minor" releases
- Delete reports before archiving
- Override CI failures without fixing root cause

---

## Command Quick Reference

### Local Validation

```bash
# Full preflight validation (mock mode)
PREFLIGHT_MOCK=1 scripts/preflight/preflight.sh --network mainnet --release-tag v1.0.0-rc1

# Full preflight validation (production mode)
scripts/preflight/preflight.sh --network mainnet --release-tag v1.0.0

# View report
cat preflight_report.md

# View raw logs
cat preflight_raw_logs.txt

# Run individual checks
scripts/preflight/check_genesis_hash.sh mainnet v1.0.0
scripts/preflight/check_dns_seeds.sh mainnet v1.0.0
scripts/preflight/check_pow_params.sh mainnet v1.0.0

# Build preflight tool
cargo build --release -p bq-preflight

# Run preflight tool
./target/release/bq-preflight dns-check --network mainnet
./target/release/bq-preflight tcp-probe --host seed1.bitquan.network --port 8333
```

### CI Validation

```bash
# Trigger workflow manually
gh workflow run preflight.yml -f network=mainnet -f release_tag=v1.0.0-rc1

# List recent runs
gh run list --workflow=preflight.yml --limit 10

# View run details
gh run view <run-id>

# Download artifacts
gh run download <run-id> -n preflight-report-mainnet
gh run download <run-id> -n preflight-report-testnet

# Watch run in progress
gh run watch <run-id>
```

### Testing

```bash
# Run all tests
cargo test --all --locked

# Run preflight unit tests
cargo test -p bq-preflight

# Run preflight integration tests (if in workspace)
cargo test --test preflight_validation

# Run with verbose output
cargo test -p bq-preflight -- --nocapture
```

---

## Summary

The `preflight_report.md` file is your **single source of truth** for pre-launch validation.

**Key takeaways:**
1. ✅ **All PASS** = Safe to release
2. ❌ **Any FAIL** = Block release, fix issues
3. **Details column** explains what happened
4. **Raw logs** provide debugging information
5. **Mock mode** for CI, **production mode** for final validation
6. **30-day retention** in CI artifacts

**When in doubt:**
- Review `preflight_raw_logs.txt`
- Consult `docs/PRELAUNCH_CHECKLIST.md`
- Run individual check scripts manually
- Check CI workflow logs

**For mainnet launch:**
- Production mode validation required
- All checks must PASS
- Manual review checklist completion
- Team approval before release

---

*Document Version: 1.0.0*
*Last Updated: November 6, 2024*
*Phase: 6.5 - Pre-Launch Validation*
