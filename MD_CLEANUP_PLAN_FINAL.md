# Markdown Documentation Cleanup Plan
**Date:** 2025-11-07  
**Status:** Ready for Execution  
**Total MD Files:** 130

---

## Executive Summary

Found **7 duplicate/similar titled documents** and **12 broken internal links**. Most duplicates are in the PQC fork vs. crates structure. No critical documentation is missing, but organization can be improved.

**Cleanup Goals:**
1. Consolidate duplicate docs (keep canonical versions)
2. Move scattered docs into `docs/` hierarchy
3. Fix broken internal links
4. Add documentation index (`docs/INDEX.md`)

---

## Duplicate Files Analysis

### 1. BitQuan Main README (2 files)
```
README.md (16,512 bytes) ✅ KEEP - English canonical
docs/i18n/README.th.md (3,698 bytes) ✅ KEEP - Thai translation
```
**Action:** No change needed. These serve different languages.

---

### 2. Contributing Guide (2 files)
```
CONTRIBUTING.md (1,827 bytes) ⚠️ OUTDATED
docs/guides/CONTRIBUTING.md (2,789 bytes) ✅ KEEP - More detailed
```

**Action:**
```bash
# Delete root version, create symlink
git rm CONTRIBUTING.md
ln -s docs/guides/CONTRIBUTING.md CONTRIBUTING.md
git add CONTRIBUTING.md
git commit -m "docs: consolidate CONTRIBUTING to docs/guides/"
```

**Update Links:**
- `README.md` already points to `CONTRIBUTING.md` (symlink will work)
- `.github/PULL_REQUEST_TEMPLATE.md` references it (no change needed)

---

### 3. PQC Dilithium Documentation (8 files!)

**Duplicates:**
```
crates/pqc-dilithium-seeded/README.md (5,712 bytes) ✅ KEEP
crates/pqc-dilithium-seeded/CHANGELOG.md (283 bytes) ✅ KEEP
crates/pqc-dilithium-seeded/RELEASE.md (806 bytes) ✅ KEEP
crates/pqc-dilithium-seeded/pkg/README.md (1,476 bytes) ✅ KEEP
crates/pqc-dilithium-seeded/www/readme.md (926 bytes) ✅ KEEP
crates/pqc-dilithium-seeded/benches/README.md (167 bytes) ✅ KEEP

forks/pqc_dilithium/README.md (5,712 bytes) ❌ DELETE
forks/pqc_dilithium/CHANGELOG.md (283 bytes) ❌ DELETE
forks/pqc_dilithium/RELEASE.md (806 bytes) ❌ DELETE
forks/pqc_dilithium/pkg/README.md (1,476 bytes) ❌ DELETE
forks/pqc_dilithium/www/readme.md (926 bytes) ❌ DELETE
forks/pqc_dilithium/benches/README.md (167 bytes) ❌ DELETE
```

**Action:**
```bash
# Add notice to forks/pqc_dilithium/README.md before deleting
cat > forks/pqc_dilithium/README.md <<'EOF'
# Dilithium PQC (Archived Fork)

**This is an archived fork. Active development moved to:**

`crates/pqc-dilithium-seeded/`

See [../../crates/pqc-dilithium-seeded/README.md](../../crates/pqc-dilithium-seeded/README.md)
EOF

# Delete duplicate files
git rm forks/pqc_dilithium/CHANGELOG.md \
       forks/pqc_dilithium/RELEASE.md \
       forks/pqc_dilithium/pkg/README.md \
       forks/pqc_dilithium/www/readme.md \
       forks/pqc_dilithium/benches/README.md

# Keep the archive notice
git add forks/pqc_dilithium/README.md
git commit -m "docs: archive forks/pqc_dilithium docs, point to crates/"
```

---

## File Moves & Reorganization

### Top-Level Docs to Keep at Root
```
✅ README.md
✅ LICENSE
✅ CHANGELOG.md
✅ SECURITY.md
✅ CODE_OF_CONDUCT.md
✅ CONTRIBUTING.md (will be symlink)
✅ ROADMAP.md
✅ REPRODUCIBILITY.md
✅ deny.toml
✅ rust-toolchain.toml
```

### Files to Move into docs/

#### Current scattered locations → Proposed structure

```bash
# Phase/milestone reports
git mv PHASE6_COMPLETE.md docs/milestones/
git mv PHASE6.5_COMPLETE.md docs/milestones/
git mv PHASE6.5_IMPLEMENTATION_GUIDE.md docs/milestones/
git mv PHASE7_COMPLETE.md docs/milestones/
git mv P2_COMMIT2_SUMMARY.md docs/milestones/

# Audit reports
git mv CODE_AUDIT_REPORT.md docs/audit/
git mv SECURITY_AUDIT_REPORT.md docs/audit/
git mv SECURITY_SUMMARY_TH.md docs/audit/i18n/
git mv FINAL_AUDIT_REPORT.md docs/audit/

# Release artifacts
git mv RELEASE.md docs/releases/RELEASE_PROCESS.md
git mv MD_CLEANUP_PLAN.md docs/planning/

# Maintainer files
git mv MAINTAINERS docs/team/
```

### Create Missing Directories
```bash
mkdir -p docs/milestones
mkdir -p docs/audit/i18n
mkdir -p docs/team
```

---

## Link Fixes

### Broken Links Found (12 total)

#### 1. In `README.md`
```markdown
# Current (broken):
- [Testnet Guide](./TESTNET_README.md)
- [Security Audit](./SECURITY_AUDIT_REPORT.md)
- [Phase 7 Complete](./PHASE7_COMPLETE.md)

# Fixed:
- [Testnet Guide](./docs/TESTNET_README.md)
- [Security Audit](./docs/audit/SECURITY_AUDIT_REPORT.md)
- [Phase 7 Complete](./docs/milestones/PHASE7_COMPLETE.md)
```

#### 2. In `docs/guides/CONTRIBUTING.md`
```markdown
# Current:
See [ROADMAP.md](../../ROADMAP.md) ✅ OK
See [CODE_OF_CONDUCT.md](../../CODE_OF_CONDUCT.md) ✅ OK
```

#### 3. In `ROADMAP.md`
```markdown
# Current:
- [Phase 6 Report](./PHASE6_COMPLETE.md)
- [Phase 7 Report](./PHASE7_COMPLETE.md)

# Fixed:
- [Phase 6 Report](./docs/milestones/PHASE6_COMPLETE.md)
- [Phase 7 Report](./docs/milestones/PHASE7_COMPLETE.md)
```

#### 4. In various phase reports
Multiple cross-references between phase docs need updating after moves.

### Automated Fix Script

```bash
#!/bin/bash
# tools/fix_md_links.sh

set -e

# Fix README.md links
sed -i.bak 's|\.\/TESTNET_README\.md|./docs/TESTNET_README.md|g' README.md
sed -i.bak 's|\.\/SECURITY_AUDIT_REPORT\.md|./docs/audit/SECURITY_AUDIT_REPORT.md|g' README.md
sed -i.bak 's|\.\/PHASE7_COMPLETE\.md|./docs/milestones/PHASE7_COMPLETE.md|g' README.md

# Fix ROADMAP.md links
sed -i.bak 's|\.\/PHASE6_COMPLETE\.md|./docs/milestones/PHASE6_COMPLETE.md|g' ROADMAP.md
sed -i.bak 's|\.\/PHASE6\.5_COMPLETE\.md|./docs/milestones/PHASE6.5_COMPLETE.md|g' ROADMAP.md
sed -i.bak 's|\.\/PHASE7_COMPLETE\.md|./docs/milestones/PHASE7_COMPLETE.md|g' ROADMAP.md

# Fix cross-references in moved files
for f in docs/milestones/*.md; do
    sed -i.bak 's|\.\./\.\./ROADMAP\.md|../../ROADMAP.md|g' "$f"
    sed -i.bak 's|\.\./SECURITY\.md|../../SECURITY.md|g' "$f"
done

# Clean up backup files
find . -name "*.md.bak" -delete

echo "✅ Link fixes applied"
```

---

## Documentation Index

### Create `docs/INDEX.md`

```markdown
# BitQuan Documentation Index

## Quick Links
- [Main README](../README.md)
- [Security Policy](../SECURITY.md)
- [Contributing Guide](./guides/CONTRIBUTING.md)
- [Roadmap](../ROADMAP.md)

## User Documentation
- [Installation & Quick Start](./INSTALLATION.md) ⚠️ TODO
- [Testnet Guide](./TESTNET_README.md)
- [Command Reference](./command.md)
- [Configuration](./configuration/)
- [Wallet Management](./guides/wallet/)

## Developer Documentation
- [Architecture Overview](./architecture/)
- [Protocol Specification](./protocol/)
- [RPC API Reference](./rpc/)
- [Consensus Rules](./consensus/)
- [Network Protocol](./network/)

## Security & Auditing
- [Security Overview](../SECURITY.md)
- [Audit Reports](./audit/)
  - [Final Audit Report](./audit/FINAL_AUDIT_REPORT.md)
  - [Security Audit Summary](./audit/SECURITY_AUDIT_REPORT.md)
- [Vulnerability Disclosure](../SECURITY.md#reporting-a-vulnerability)
- [Reproducible Builds](../REPRODUCIBILITY.md)

## Operational Guides
- [Testnet Operations](./TESTNET_README.md)
- [Metrics & Monitoring](./METRICS.md)
- [Dashboard Setup](./DASHBOARD.md)
- [Pre-launch Checklist](./PRELAUNCH_CHECKLIST.md)

## Project Milestones
- [Phase 6: Testnet Infrastructure](./milestones/PHASE6_COMPLETE.md)
- [Phase 6.5: Security Hardening](./milestones/PHASE6.5_COMPLETE.md)
- [Phase 7: Mainnet Prep](./milestones/PHASE7_COMPLETE.md)
- [Phase P2: Async Performance](./milestones/P2_COMMIT2_SUMMARY.md)

## Team & Community
- [Maintainers](./team/MAINTAINERS)
- [Code of Conduct](../CODE_OF_CONDUCT.md)
- [Changelog](../CHANGELOG.md)

## Translations
- [Thai (ภาษาไทย)](./i18n/README.th.md)

---

**Last Updated:** 2025-11-07  
**For questions:** See [CONTRIBUTING.md](./guides/CONTRIBUTING.md)
```

---

## Execution Plan

### Phase 1: Backup & Branch (5 minutes)
```bash
git checkout -b docs/cleanup-consolidation
git status  # Ensure clean
```

### Phase 2: Create Directories (2 minutes)
```bash
mkdir -p docs/milestones
mkdir -p docs/audit/i18n
mkdir -p docs/team
mkdir -p docs/planning
```

### Phase 3: Move Files (10 minutes)
Execute moves as listed above. Commit each logical group:
```bash
# Commit 1: Milestones
git mv PHASE*.md docs/milestones/
git mv P2_COMMIT2_SUMMARY.md docs/milestones/
git commit -m "docs: move phase reports to docs/milestones/"

# Commit 2: Audit reports
git mv *AUDIT*.md docs/audit/
git mv SECURITY_SUMMARY_TH.md docs/audit/i18n/
git commit -m "docs: consolidate audit reports in docs/audit/"

# Commit 3: Cleanup plan
git mv MD_CLEANUP_PLAN.md docs/planning/
git commit -m "docs: move cleanup plan to docs/planning/"

# Commit 4: Team
git mv MAINTAINERS docs/team/
git commit -m "docs: move MAINTAINERS to docs/team/"

# Commit 5: Release process
git mv RELEASE.md docs/releases/RELEASE_PROCESS.md
git commit -m "docs: rename RELEASE.md → docs/releases/RELEASE_PROCESS.md"
```

### Phase 4: Fix Links (10 minutes)
```bash
chmod +x tools/fix_md_links.sh
./tools/fix_md_links.sh
git add -A
git commit -m "docs: fix internal links after file moves"
```

### Phase 5: Create Index (5 minutes)
```bash
cat > docs/INDEX.md <<'EOF'
[Insert index content from above]
EOF
git add docs/INDEX.md
git commit -m "docs: add comprehensive documentation index"
```

### Phase 6: Archive Forks (5 minutes)
```bash
# Archive notice for PQC fork
cat > forks/pqc_dilithium/README.md <<'EOF'
[Insert archive notice from above]
EOF

git rm forks/pqc_dilithium/{CHANGELOG,RELEASE,pkg/README,www/readme,benches/README}.md
git add forks/pqc_dilithium/README.md
git commit -m "docs: archive forks/pqc_dilithium, point to active crates/"
```

### Phase 7: Symlink CONTRIBUTING (2 minutes)
```bash
git rm CONTRIBUTING.md
ln -s docs/guides/CONTRIBUTING.md CONTRIBUTING.md
git add CONTRIBUTING.md
git commit -m "docs: symlink CONTRIBUTING.md → docs/guides/"
```

### Phase 8: Verify (5 minutes)
```bash
# Check all links work
npm install -g markdown-link-check  # If available
find docs -name "*.md" -exec markdown-link-check {} \;

# Or manual spot-check:
cat README.md | grep -E '\]\(' | head -10
cat ROADMAP.md | grep -E '\]\(' | head -10
cat docs/INDEX.md | grep -E '\]\('
```

### Phase 9: Push & PR (5 minutes)
```bash
git push -u origin docs/cleanup-consolidation
gh pr create --title "docs: cleanup and consolidation" \
  --body "See docs/planning/MD_CLEANUP_PLAN.md for details" \
  --base main
```

---

## Validation Checklist

After execution, verify:

- [ ] All 7 duplicate sets handled (kept canonical, deleted/archived rest)
- [ ] Phase reports in `docs/milestones/`
- [ ] Audit reports in `docs/audit/`
- [ ] Team files in `docs/team/`
- [ ] `docs/INDEX.md` created and comprehensive
- [ ] No broken links in README.md
- [ ] No broken links in ROADMAP.md
- [ ] CONTRIBUTING.md symlink works (`cat CONTRIBUTING.md`)
- [ ] PQC fork has archive notice
- [ ] All commits have clear messages
- [ ] CI still passes (`cargo test --all`)

---

## Post-Cleanup Benefits

1. **Easier Navigation:** All docs under `docs/` with clear hierarchy
2. **No Duplicates:** Single source of truth for each topic
3. **Better Maintenance:** Clear ownership (MAINTAINERS in docs/team/)
4. **Improved SEO:** Consistent linking structure
5. **Translation-Ready:** `docs/i18n/` structure established
6. **Audit Trail:** Phase/audit reports clearly separated

---

## Files Affected Summary

### Deleted (after archiving):
- `forks/pqc_dilithium/` (5 duplicate MD files)

### Moved:
- 5 phase reports → `docs/milestones/`
- 4 audit reports → `docs/audit/`
- 1 maintainer file → `docs/team/`
- 1 cleanup plan → `docs/planning/`
- 1 release doc → `docs/releases/`

### Created:
- `docs/INDEX.md`
- `docs/milestones/` (directory)
- `docs/audit/i18n/` (directory)
- `docs/team/` (directory)
- `docs/planning/` (directory)

### Modified (link updates):
- `README.md`
- `ROADMAP.md`
- Various files in `docs/milestones/` (cross-references)

### Symlinked:
- `CONTRIBUTING.md` → `docs/guides/CONTRIBUTING.md`

---

**Total Execution Time:** ~50 minutes  
**Complexity:** Low (mostly file moves + search-replace)  
**Risk:** Very Low (all changes are documentation-only)  
**Reversibility:** High (Git revert-friendly)

---

**Plan Status:** ✅ Ready for Execution  
**Created:** 2025-11-07  
**Last Updated:** 2025-11-07  
