# Documentation Refactoring Plan

**Status**: Phase A Complete ✅  
**Last Updated**: 2025-01-07

This document tracks the documentation reorganization effort to improve discoverability, reduce duplication, and fix broken links.

## Overview

BitQuan had **136 markdown files** scattered across the repository with:
- 43 normalized duplicate file names
- 39 broken relative links
- 113 orphaned files (not linked from README)

**Solution**: Reorganize into clear domain-based structure with centralized index.

## Phase A: Structure & Index ✅ COMPLETE

**Goal**: Create new `docs/` structure and move key files

### New Structure Created

```
docs/
├── INDEX.md                    # Central documentation hub
├── getting-started/            # Installation, quick start
├── concepts/                   # Consensus, UTXO, PQC, ASERT
├── cli/                        # CLI documentation
│   ├── README.md
│   ├── bitquan-node.md
│   ├── bitquan-wallet.md
│   ├── bq-stress.md
│   └── bq-preflight.md
├── dev/                        # Development guides
│   ├── README.md
│   ├── COVERAGE.md
│   ├── LOGGING_POLICY.md
│   └── PANIC_SAFETY.md
├── ops/                        # Operations & deployment
│   ├── README.md
│   ├── RUNBOOK.md
│   ├── OBSERVABILITY.md
│   ├── PRELAUNCH_CHECKLIST.md
│   ├── PHASE7_LAUNCH_READY.md
│   ├── PHASE7_COMPLETE.md
│   └── PHASE7_QUICKREF.md
├── testnet/                    # Testnet setup
│   ├── README.md
│   └── LOAD_TESTING.md
├── security/                   # Security docs
│   ├── README.md
│   ├── AUDIT_HANDOFF.md
│   ├── AUDIT_HANDOFF_CHECKLIST.md
│   ├── AUDIT_SUMMARY.md
│   └── BUG_BOUNTY.md
├── releases/                   # Release notes
│   └── MAINNET_ANNOUNCEMENT.md
├── roadmap/                    # Planning docs
├── assets/                     # Images, dashboards
└── _archive/                   # Deprecated files
```

### Files Moved

**Operations** (→ `docs/ops/`):
- PHASE7_LAUNCH_READY.md
- PHASE7_COMPLETE.md
- PHASE7_QUICKREF.md
- RUNBOOK.md
- PRELAUNCH_CHECKLIST.md
- OBSERVABILITY.md

**Security** (→ `docs/security/`):
- AUDIT_HANDOFF.md
- AUDIT_HANDOFF_CHECKLIST.md
- AUDIT_SUMMARY.md
- BUG_BOUNTY.md

**Testnet** (→ `docs/testnet/`):
- TESTNET_README.md → README.md
- LOAD_TESTING.md

**Development** (→ `docs/dev/`):
- COVERAGE.md
- LOGGING_POLICY.md
- PANIC_SAFETY.md

**CLI** (→ `docs/cli/`):
- command.md

**Concepts** (→ `docs/concepts/`):
- CONSENSUS_ECON.md
- GENESIS.md

**Releases** (→ `docs/releases/`):
- MAINNET_ANNOUNCEMENT.md

### New Files Created

- `docs/INDEX.md` - Central documentation hub
- `docs/cli/README.md` - CLI overview
- `docs/cli/bitquan-node.md` - Node command reference
- `docs/cli/bitquan-wallet.md` - Wallet command reference
- `docs/cli/bq-stress.md` - Stress testing tool
- `docs/cli/bq-preflight.md` - Preflight validation tool
- `docs/ops/README.md` - Operations overview
- `docs/security/README.md` - Security overview (updated)
- `docs/dev/README.md` - Development guide

### Links Updated

- `README.md` → Points to `docs/INDEX.md`
- `README.md` → Fixed references to moved files

## Phase B: Deduplicate & Fix Links 🔄 TODO

**Goal**: Merge duplicate files and fix all broken links

### Duplicate Resolution Strategy

For each duplicate pair:
1. Compare file sizes, H1 headings, last modified date
2. Keep version with most content and clearest H1
3. Merge unique content from duplicate into keeper
4. Delete duplicate file
5. Update all references to point to keeper
6. Create git commit with redirect comment

### Broken Links to Fix (39 total)

**Script Approach**:
```bash
# Find all markdown files
find docs -name "*.md" -type f > /tmp/md_files.txt

# For each file, extract links
grep -oP '\[.*?\]\(\K[^)]+' file.md

# Check if target exists
# If not, search for similar filename
# Suggest correction
```

### Link Policy

**Going Forward**:
- All relative links from `docs/` root
- Example: `../ops/preflight.md` (not `../../docs/ops/preflight.md`)
- Use directory READMEs for section landing pages
- No absolute GitHub URLs for internal files

### Orphan File Resolution

**113 orphaned files** need one of:
1. **Link from INDEX/README** - If still relevant
2. **Merge into parent** - If content belongs elsewhere
3. **Move to `_archive/`** - If historical/deprecated
4. **Delete** - If truly obsolete (after review)

### Tasks

- [ ] Generate duplicate pairs report with details
- [ ] Merge top 20 most obvious duplicates
- [ ] Fix broken links (automated script)
- [ ] Categorize all orphan files
- [ ] Link or archive orphans
- [ ] Run link checker to verify all fixed

## Phase C: Documentation Website 🔄 TODO

**Goal**: Deploy static documentation site

### Technology Choice

**Option 1: MkDocs + Material Theme**
- ✅ Beautiful, searchable UI
- ✅ Easy navigation
- ✅ GitHub Pages deployment
- ✅ Code syntax highlighting
- ✅ Auto-generate nav from structure
- ❌ Requires Python, build step

**Option 2: Docsify**
- ✅ No build step (pure client-side)
- ✅ Single `index.html` file
- ✅ Markdown rendering in browser
- ✅ Search plugin
- ❌ Less polished than MkDocs

**Recommendation**: Start with **Docsify** for speed, migrate to MkDocs if needed.

### Implementation

```bash
# Create docs site
cat > docs/index.html <<'EOF'
<!DOCTYPE html>
<html>
<head>
  <meta charset="UTF-8">
  <title>BitQuan Documentation</title>
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <link rel="stylesheet" href="//cdn.jsdelivr.net/npm/docsify@4/themes/vue.css">
</head>
<body>
  <div id="app"></div>
  <script>
    window.$docsify = {
      name: 'BitQuan',
      repo: 'your-org/BitQuan',
      loadSidebar: true,
      subMaxLevel: 3,
      search: 'auto',
      homepage: 'INDEX.md'
    }
  </script>
  <script src="//cdn.jsdelivr.net/npm/docsify@4"></script>
  <script src="//cdn.jsdelivr.net/npm/docsify/lib/plugins/search.js"></script>
</body>
</html>
EOF

# Create sidebar
cat > docs/_sidebar.md <<'EOF'
- [Home](/)
- [Getting Started](getting-started/)
- [Concepts](concepts/)
- [CLI Reference](cli/)
- [Development](dev/)
- [Operations](ops/)
- [Security](security/)
- [Testnet](testnet/)
- [Releases](releases/)
EOF

# Test locally
cd docs && python3 -m http.server 3000
# Visit http://localhost:3000
```

### GitHub Pages Deployment

```bash
# Enable GitHub Pages
# Settings → Pages → Source: Deploy from branch → Branch: main → Folder: /docs

# Site will be available at:
# https://your-org.github.io/BitQuan/
```

### Tasks

- [ ] Choose documentation framework (Docsify vs MkDocs)
- [ ] Create `docs/index.html` and `_sidebar.md`
- [ ] Test locally
- [ ] Configure GitHub Pages
- [ ] Add link to README
- [ ] Set up custom domain (optional)

## Quality Standards

All documentation must follow:

### File Standards
- ✅ One H1 per file matching filename
- ✅ H2 for major sections, H3 for subsections
- ✅ "Last Updated: YYYY-MM-DD" at bottom
- ✅ Relative links only (from `docs/` root)
- ✅ Code blocks with language tags

### Runbook Standards (ops/)
- ✅ Checklist for procedures
- ✅ Rollback steps
- ✅ Prerequisites section
- ✅ Expected output/results
- ✅ Troubleshooting section

### Media Standards
- ✅ Images in `docs/assets/images/`
- ✅ Dashboards in `docs/assets/dashboards/`
- ✅ Alt text for all images
- ✅ Max 1MB per image (optimize PNGs)

## Metrics

**Before**:
- Markdown files: 136
- Normalized duplicates: 43
- Broken links: 39
- Orphan files: 113
- Documentation sections: Unclear

**After Phase A**:
- ✅ New structure: 9 clear sections
- ✅ Central INDEX.md created
- ✅ README.md updated to point to INDEX
- ✅ CLI docs split into 4 files
- ✅ Section READMEs created (ops, security, dev)
- 🔄 Duplicates: Still 43 (Phase B)
- 🔄 Broken links: Partially fixed (Phase B)
- 🔄 Orphans: Still 113 (Phase B)

**Target After Phase B**:
- Duplicates: 0
- Broken links: 0
- Orphans: 0 (all linked or archived)

**Target After Phase C**:
- Live documentation site at https://your-org.github.io/BitQuan/
- Full-text search working
- Mobile-friendly navigation

## Timeline

- **Phase A**: ✅ Complete (2025-01-07)
- **Phase B**: 1-2 days (deduplication + link fixing)
- **Phase C**: Half day (Docsify setup + deploy)

**Total Estimated Time**: 2-3 days for full documentation overhaul

## Next Steps

1. Review Phase A changes (this commit)
2. Generate detailed duplicate pairs report
3. Begin Phase B: Merge top duplicates
4. Script link fixing automation
5. Categorize and process orphan files
6. Deploy Phase C: Documentation site

---

*This is a living document. Update after each phase completion.*
