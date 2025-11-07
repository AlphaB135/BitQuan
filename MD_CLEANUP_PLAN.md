# BitQuan Markdown Documentation Cleanup Plan

**Date:** 2025-11-07  
**Status:** Proposed  
**Goal:** Consolidate 120+ MD files into canonical `docs/` structure with clear hierarchy

---

## Current State Analysis

**Total MD Files:** 120+  
**Issues Identified:**
1. Top-level clutter (PHASE*.md, P2_*.md, SECURITY_*.md)
2. Duplicate BQIPs (`docs/spec/BQIP-*.md` vs `docs/bqip/BQIP-*.md`)
3. Multiple README files in nested directories
4. Scattered status reports not in `docs/status/`
5. Release notes mixed in top-level vs `docs/releases/`

---

## Proposed Canonical Structure

```
BitQuan/
├── README.md                              # Main entry point (keep)
├── CHANGELOG.md                           # Version history (keep)
├── LICENSE                                # Keep
├── CODE_OF_CONDUCT.md                    # Keep
├── CONTRIBUTING.md                       # Keep (or → docs/guides/)
├── SECURITY.md                            # Keep (GitHub scans this location)
│
└── docs/
    ├── README.md                          # Index/navigation for all docs
    │
    ├── guides/                            # User-facing tutorials
    │   ├── QUICKSTART.md
    │   ├── INSTALL.md
    │   ├── CONTRIBUTING.md
    │   ├── RELEASE.md
    │   └── JWT_QUICK_START.md
    │
    ├── spec/                              # Protocol specifications
    │   ├── BQIP-0001.md
    │   ├── BQIP-0002.md
    │   ├── BQIP-0003.md
    │   ├── BQIP-0004.md
    │   ├── block.md
    │   ├── transaction.md
    │   ├── consensus_economics.md
    │   ├── block-weight.md
    │   └── test-vectors.md
    │
    ├── architecture/                      # System design docs
    │   ├── overview.md
    │   ├── code-structure.md
    │   ├── data-structures.md
    │   └── system-overview.md
    │
    ├── security/                          # Security artifacts
    │   ├── README.md
    │   ├── ENTROPY_AUDIT.md
    │   ├── NO_BACKDOORS.md
    │   ├── REPRODUCIBILITY.md
    │   ├── GPG_SIGNING.md
    │   ├── oncall.md
    │   ├── audits/
    │   │   └── README.md
    │   ├── attestations/
    │   │   └── README.md
    │   └── keys/
    │       └── maintainers/
    │           └── README.md
    │
    ├── releases/                          # Version-specific notes
    │   ├── RELEASE_NOTES_v0.0.1-alpha.md
    │   ├── RELEASE_NOTES_v0.0.2-alpha.md
    │   ├── PHASE6_COMPLETE.md
    │   ├── PHASE6.5_COMPLETE.md
    │   ├── PHASE7_COMPLETE.md
    │   └── P2_COMMIT2_SUMMARY.md
    │
    ├── status/                            # Implementation status
    │   ├── BIP39_DETERMINISTIC_STATUS.md
    │   ├── CLEANUP_SUMMARY.md
    │   ├── DOCUMENTATION_COMPLETE.md
    │   ├── JWT_COMPLETE_SUMMARY.md
    │   ├── JWT_MANUAL_TEST.md
    │   ├── JWT_MVP_COMPLETE.md
    │   ├── JWT_STATUS.md
    │   ├── MULTISIG_COMPLETE.md
    │   ├── PHASE2_COMPLETE.md
    │   ├── SECURITY_WEEK1_SUMMARY.md
    │   ├── TASK_HIJ_COMPLETE.md
    │   ├── TASK_IJK_PLAN.md
    │   ├── TLS_IMPLEMENTATION_SUMMARY.md
    │   ├── TODAY_ACCOMPLISHMENTS.md
    │   └── WARNINGS_CLEAN.md
    │
    ├── operations/                        # Ops runbooks
    │   ├── RUNBOOK.md
    │   ├── POOL_OPERATIONS.md
    │   ├── DASHBOARD.md
    │   ├── METRICS.md
    │   ├── OBSERVABILITY.md
    │   ├── LOAD_TESTING.md
    │   └── LOGGING_POLICY.md
    │
    ├── wallet/                            # Wallet-specific docs
    │   ├── README.md
    │   ├── backup.md
    │   └── MULTISIG_GUIDE.md
    │
    ├── storage/                           # Database docs
    │   └── DATABASE_RECOVERY.md
    │
    ├── development/                       # Dev guides
    │   ├── MINING_IMPROVEMENTS.md
    │   └── SECURITY_FIXES.md
    │
    ├── planning/                          # Task tracking
    │   ├── todo.md
    │   ├── TODO_SECURITY_CRITICAL.md
    │   └── TODO_UPDATE.md
    │
    ├── fuzzing/                           # Fuzz testing
    │   └── FUZZING_STATUS.md
    │
    ├── governance/                        # Governance model
    │   └── GOVERNANCE.md
    │
    ├── i18n/                              # Translations
    │   ├── README.en.md
    │   └── README.th.md
    │
    └── rpc/                               # RPC testing
        └── testing.md
```

---

## Move Actions (Automated)

### Phase 1: Top-Level to docs/releases/
```bash
git mv PHASE6_COMPLETE.md docs/releases/
git mv PHASE6.5_COMPLETE.md docs/releases/
git mv PHASE6.5_IMPLEMENTATION_GUIDE.md docs/releases/
git mv PHASE7_COMPLETE.md docs/releases/
git mv P2_COMMIT2_SUMMARY.md docs/releases/
git mv RELEASE.md docs/guides/RELEASE.md
```

### Phase 2: Top-Level to docs/security/
```bash
git mv REPRODUCIBILITY.md docs/security/
git mv SECURITY_AUDIT_REPORT.md docs/security/audits/AUDIT_REPORT_2025.md
git mv SECURITY_SUMMARY_TH.md docs/security/SUMMARY_TH.md
```

### Phase 3: Top-Level to docs/
```bash
git mv ROADMAP.md docs/ROADMAP.md
```

### Phase 4: Consolidate BQIPs (Remove Duplicates)
```bash
# Keep docs/spec/BQIP-*.md as canonical
# Remove docs/bqip/ after verifying content matches
diff docs/spec/BQIP-0001.md docs/bqip/BQIP-0001.md
diff docs/spec/BQIP-0002.md docs/bqip/BQIP-0002.md
diff docs/spec/BQIP-0003.md docs/bqip/BQIP-0003.md
diff docs/spec/BQIP-0004.md docs/bqip/BQIP-0004.md

# If identical:
git rm -r docs/bqip/
```

### Phase 5: Reorganize docs/ Internal Structure
```bash
# Create missing directories
mkdir -p docs/operations

# Move to operations/
git mv docs/RUNBOOK.md docs/operations/
git mv docs/POOL_OPERATIONS.md docs/operations/
git mv docs/DASHBOARD.md docs/operations/
git mv docs/METRICS.md docs/operations/
git mv docs/OBSERVABILITY.md docs/operations/
git mv docs/LOAD_TESTING.md docs/operations/
git mv docs/LOGGING_POLICY.md docs/operations/

# Consolidate guides/
# (Most already in docs/guides/, verify)

# Move spec files to spec/
git mv docs/overview.md docs/spec/overview.md
git mv docs/address-and-script.md docs/spec/address-and-script.md
git mv docs/command.md docs/guides/COMMANDS.md
```

### Phase 6: Crate-Specific Docs
```bash
# Keep crate docs in their directories:
# - crates/wallet/*.md (keep as-is)
# - crates/rpc/*.md (keep as-is)
# - crates/pqc-dilithium-seeded/*.md (keep as-is)

# Remove duplicates if any:
# e.g., if docs/wallet/README.md duplicates crates/wallet/README.md
diff docs/wallet/README.md crates/wallet/README.md
# If duplicate → keep crate version, link from docs/
```

### Phase 7: Archive Old Status Files
```bash
# Move old status reports to archive if not actively referenced
mkdir -p docs/archive/status-2024
git mv docs/status/TODAY_ACCOMPLISHMENTS.md docs/archive/status-2024/
# (Or just delete if obsolete)
```

---

## Link Rewrites

After moving files, update all internal `[text](./path.md)` references.

### Automated Search-Replace Map

| Old Path | New Path |
|----------|----------|
| `./PHASE6_COMPLETE.md` | `./docs/releases/PHASE6_COMPLETE.md` |
| `./PHASE6.5_COMPLETE.md` | `./docs/releases/PHASE6.5_COMPLETE.md` |
| `./PHASE7_COMPLETE.md` | `./docs/releases/PHASE7_COMPLETE.md` |
| `./P2_COMMIT2_SUMMARY.md` | `./docs/releases/P2_COMMIT2_SUMMARY.md` |
| `./REPRODUCIBILITY.md` | `./docs/security/REPRODUCIBILITY.md` |
| `./SECURITY_AUDIT_REPORT.md` | `./docs/security/audits/AUDIT_REPORT_2025.md` |
| `./ROADMAP.md` | `./docs/ROADMAP.md` |
| `./RELEASE.md` | `./docs/guides/RELEASE.md` |
| `./docs/RUNBOOK.md` | `./docs/operations/RUNBOOK.md` |
| `./docs/METRICS.md` | `./docs/operations/METRICS.md` |
| `./docs/DASHBOARD.md` | `./docs/operations/DASHBOARD.md` |
| `./docs/OBSERVABILITY.md` | `./docs/operations/OBSERVABILITY.md` |
| `./docs/LOAD_TESTING.md` | `./docs/operations/LOAD_TESTING.md` |
| `./docs/bqip/BQIP-*.md` | `./docs/spec/BQIP-*.md` |

### Execution Script
```bash
# Run after moves
find . -name "*.md" -not -path "./target/*" -not -path "./.git/*" \
  -exec sed -i.bak 's|(\./PHASE6_COMPLETE\.md)|(./docs/releases/PHASE6_COMPLETE.md)|g' {} \;

find . -name "*.md" -not -path "./target/*" -not -path "./.git/*" \
  -exec sed -i.bak 's|(\./REPRODUCIBILITY\.md)|(./docs/security/REPRODUCIBILITY.md)|g' {} \;

# ... repeat for each mapping

# Clean up backups
find . -name "*.md.bak" -delete
```

---

## Duplicates Found (Resolve Manually)

### 1. BQIP Files
- `docs/spec/BQIP-0001.md` vs `docs/bqip/BQIP-0001.md`
- **Action:** Verify identical content, keep `docs/spec/`, remove `docs/bqip/`

### 2. README Files
- Root `README.md` (main entry) - **KEEP**
- `docs/README.md` (doc index) - **KEEP**
- `bindings/ts/README.md` (TS bindings) - **KEEP**
- `crates/*/README.md` (crate-specific) - **KEEP**
- `forks/*/README.md` (fork-specific) - **KEEP**
- `sdk/README.md` - **KEEP**

**No duplicates found** - all serve different purposes.

### 3. Security Docs
- `SECURITY.md` (root - GitHub scans this) - **KEEP**
- `crates/wallet/SECURITY.md` (wallet-specific) - **KEEP**
- `docs/security/README.md` (security index) - **KEEP**

**No duplicates** - hierarchical structure is intentional.

### 4. CHANGELOG Files
- Root `CHANGELOG.md` (project-wide) - **KEEP**
- `crates/wallet/CHANGELOG.md` (wallet-specific) - **KEEP**
- `crates/pqc-dilithium-seeded/CHANGELOG.md` (PQC-specific) - **KEEP**

**No duplicates** - scoped appropriately.

---

## Acceptance Criteria

✅ **Phase 1-7 Moves Complete**
- All top-level docs moved to `docs/`
- Canonical structure established
- No duplicate BQIPs

✅ **Link Rewrites Applied**
- No broken internal links
- Verify with: `markdown-link-check docs/**/*.md`

✅ **README Updated**
- Root README links to `docs/README.md` as doc index
- Doc index lists all major sections

✅ **CI Validation**
- `cargo doc --no-deps` still works
- No warnings about missing files

---

## Rollback Plan

If issues arise:
```bash
git checkout HEAD -- .  # Revert working tree
git reset --hard origin/main  # Full reset
```

All moves are tracked in Git history, easy to revert individually.

---

## Timeline

- **Phase 1-3:** 30 minutes (top-level moves)
- **Phase 4:** 15 minutes (BQIP dedup)
- **Phase 5:** 30 minutes (internal reorg)
- **Phase 6:** 15 minutes (crate doc audit)
- **Phase 7:** 10 minutes (archiving)
- **Link Rewrites:** 30 minutes (automated + manual verification)

**Total:** ~2.5 hours

---

## Next Actions

1. Review this plan with maintainers
2. Execute moves in a dedicated commit: `docs: reorganize MD structure per MD_CLEANUP_PLAN.md`
3. Apply link rewrites: `docs: fix internal links after reorganization`
4. Update root README with new structure
5. Archive plan as `docs/archive/MD_CLEANUP_PLAN_2025-11.md`

---

**Plan Approved By:** [Pending]  
**Execution Date:** [Pending]  
**Status:** PROPOSED
