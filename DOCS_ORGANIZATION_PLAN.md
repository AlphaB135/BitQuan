# BitQuan Documentation Organization Plan

**Date**: 2025-11-21
**Status**: Planning
**Goal**: Clean, organized, and maintainable documentation structure

---

## 📊 Current State Analysis

### Problems Identified:
1. ❌ **200+ markdown files** scattered across the project
2. ❌ **Duplicate files** in multiple locations (SECURITY.md, REPRODUCIBILITY.md, etc.)
3. ❌ **20+ root-level MD files** cluttering the repository
4. ❌ **Conflicting documentation** (multiple security reports, production readiness reports)
5. ❌ **Unclear hierarchy** - hard to find specific documentation
6. ❌ **Outdated files** mixed with current documentation

---

## 🎯 Proposed Structure

```
BitQuan/
├── README.md                           # Main project readme (keep)
├── CONTRIBUTING.md                     # How to contribute (keep)
├── CODE_OF_CONDUCT.md                  # Community guidelines (keep)
├── SECURITY.md                         # Security policy (keep)
├── CHANGELOG.md                        # Version history (keep)
├── LICENSE                             # Apache 2.0 license
│
├── docs/                               # ALL documentation goes here
│   ├── README.md                       # Documentation hub
│   ├── INDEX.md                        # Master index (improve)
│   │
│   ├── getting-started/                # 🚀 New users start here
│   │   ├── README.md                   # Getting started overview
│   │   ├── quick-start.md              # 5-minute quick start
│   │   ├── installation.md             # Detailed installation
│   │   ├── first-transaction.md        # Create first transaction
│   │   └── testnet-guide.md            # Join testnet
│   │
│   ├── guides/                         # 📖 How-to guides
│   │   ├── README.md                   # Guides index
│   │   ├── node-operator.md            # Run a node
│   │   ├── mining.md                   # Mining guide
│   │   ├── wallet-setup.md             # Wallet setup
│   │   ├── multisig.md                 # Multisig wallets
│   │   ├── mainnet-operations.md       # Mainnet operations
│   │   └── troubleshooting.md          # Common issues
│   │
│   ├── architecture/                   # 🏗️ System design
│   │   ├── README.md                   # Architecture overview
│   │   ├── overview.md                 # High-level design
│   │   ├── consensus.md                # Consensus mechanism
│   │   ├── cryptography.md             # Post-quantum crypto
│   │   ├── network.md                  # P2P networking
│   │   ├── storage.md                  # Database design
│   │   └── data-structures.md          # Core data structures
│   │
│   ├── specifications/                 # 📜 Technical specs
│   │   ├── README.md                   # Specs overview
│   │   ├── blocks.md                   # Block format
│   │   ├── transactions.md             # Transaction format
│   │   ├── addresses.md                # Address format
│   │   ├── consensus-rules.md          # Consensus rules
│   │   ├── economics.md                # Economic model
│   │   └── test-vectors.md             # Test vectors
│   │
│   ├── bqip/                           # 📋 BitQuan Improvement Proposals
│   │   ├── README.md                   # BQIP process
│   │   ├── BQIP-0001.md                # Genesis specification
│   │   ├── BQIP-0002.md                # Block weight system
│   │   ├── BQIP-0003.md                # Wallet standards
│   │   └── BQIP-0004.md                # Network protocol
│   │
│   ├── api/                            # 🔌 API documentation
│   │   ├── README.md                   # API overview
│   │   ├── rpc/                        # JSON-RPC API
│   │   │   ├── README.md               # RPC overview
│   │   │   ├── authentication.md       # JWT auth
│   │   │   ├── blockchain.md           # Blockchain methods
│   │   │   ├── wallet.md               # Wallet methods
│   │   │   └── mining.md               # Mining methods
│   │   └── sdk/                        # SDK documentation
│   │       ├── README.md               # SDK overview
│   │       ├── rust.md                 # Rust SDK
│   │       └── typescript.md           # TypeScript SDK
│   │
│   ├── security/                       # 🔒 Security documentation
│   │   ├── README.md                   # Security overview
│   │   ├── policy.md                   # Security policy
│   │   ├── bug-bounty.md               # Bug bounty program
│   │   ├── audit-reports/              # Security audits
│   │   │   ├── README.md               # Audit index
│   │   │   ├── 2025-11-21-security-assessment.md
│   │   │   ├── crypto-audit.md         # Cryptography audit
│   │   │   ├── consensus-audit.md      # Consensus audit
│   │   │   └── dependency-audit.md     # Dependency audit
│   │   ├── best-practices.md           # Security best practices
│   │   ├── emergency-procedures.md     # Emergency response
│   │   ├── reproducibility.md          # Reproducible builds
│   │   └── gpg-signing.md              # GPG signature verification
│   │
│   ├── operations/                     # ⚙️ Operations & deployment
│   │   ├── README.md                   # Ops overview
│   │   ├── deployment.md               # Deployment guide
│   │   ├── monitoring.md               # Monitoring & observability
│   │   ├── backup-recovery.md          # Backup & recovery
│   │   ├── performance-tuning.md       # Performance optimization
│   │   ├── runbook.md                  # Operational runbook
│   │   └── prelaunch-checklist.md      # Pre-launch checklist
│   │
│   ├── development/                    # 👩‍💻 Developer documentation
│   │   ├── README.md                   # Dev overview
│   │   ├── setup.md                    # Development setup
│   │   ├── building.md                 # Building from source
│   │   ├── testing.md                  # Testing guide
│   │   ├── contributing.md             # Contributing guide
│   │   ├── code-standards.md           # Code standards
│   │   ├── coverage.md                 # Code coverage
│   │   └── fuzzing.md                  # Fuzzing guide
│   │
│   ├── releases/                       # 📦 Release documentation
│   │   ├── README.md                   # Releases overview
│   │   ├── process.md                  # Release process
│   │   ├── v0.0.1-alpha.md             # Release notes v0.0.1
│   │   ├── v0.0.2-alpha.md             # Release notes v0.0.2
│   │   └── mainnet-v1.0.0.md           # Mainnet launch notes
│   │
│   ├── archive/                        # 📚 Archived documentation
│   │   ├── README.md                   # Archive index
│   │   ├── development-reports/        # Old dev reports
│   │   ├── deprecated-specs/           # Deprecated specifications
│   │   └── migration-guides/           # Migration guides
│   │
│   └── internal/                       # 🔐 Internal documentation (optional)
│       ├── README.md                   # Internal docs overview
│       ├── CLAUDE.md                   # AI assistant guidelines
│       └── maw-agents.md               # Multi-agent workflow
│
├── crates/                             # Crate-specific docs (keep)
│   ├── wallet/
│   │   ├── README.md                   # Wallet crate docs
│   │   └── SECURITY.md                 # Wallet security
│   ├── rpc/
│   │   └── README.md                   # RPC crate docs
│   └── ...
│
└── [other directories remain unchanged]
```

---

## 🔄 Migration Plan

### Phase 1: Root Directory Cleanup ✅

**Move to docs/security/audit-reports/**:
- ❌ `SECURITY_AUDIT_REPORT.md` → `docs/security/audit-reports/2025-11-security-assessment.md`
- ❌ `SECURITY_PROGRESS_REPORT.md` → `docs/security/audit-reports/progress-report.md`
- ❌ `CODE_AUDIT_REPORT.md` → `docs/security/audit-reports/code-audit.md`
- ❌ `security_check_prompt.md` → `docs/internal/security-check-prompt.md`

**Move to docs/operations/**:
- ❌ `PRODUCTION_READINESS_REPORT.md` → `docs/operations/production-readiness-2025-01.md`
- ❌ `PRODUCTION_READINESS_AUDIT.md` → `docs/operations/production-readiness-audit.md`
- ❌ `POST_QUANTUM_SECURITY_DOCUMENTATION.md` → `docs/security/post-quantum-security.md`

**Move to docs/development/**:
- ❌ `INTEGRATION_SUCCESS_REPORT.md` → `docs/archive/integration-success-report.md`
- ❌ `TRANSACTION_TESTING_GUIDE.md` → `docs/development/transaction-testing.md`

**Move to docs/releases/**:
- ❌ `TESTNET_ANNOUNCEMENT.md` → `docs/releases/testnet-announcement.md`
- ❌ `TESTNET_LAUNCH_POST.md` → `docs/releases/testnet-launch.md`
- ❌ `BETA_TESTER_RECRUITMENT.md` → `docs/archive/beta-tester-recruitment.md`

**Move to docs/operations/**:
- ❌ `VPS_DEPLOYMENT_GUIDE.md` → `docs/operations/vps-deployment.md`

**Move to docs/internal/**:
- ❌ `CLAUDE.md` → `docs/internal/CLAUDE.md` (keep reference in root with link)
- ❌ `MAW-AGENTS.md` → `docs/internal/maw-agents.md`

**Keep in root** (essential files only):
- ✅ `README.md` - Main project readme
- ✅ `CONTRIBUTING.md` - Contributing guidelines
- ✅ `CODE_OF_CONDUCT.md` - Code of conduct
- ✅ `SECURITY.md` - Security policy (link to detailed docs)
- ✅ `CHANGELOG.md` - Version history
- ✅ `FUNDING.md` - Funding information
- ✅ `ROADMAP.md` - Project roadmap

### Phase 2: Consolidate Duplicates ✅

**SECURITY.md** (3 copies):
- ✅ Root `/SECURITY.md` - Main policy (link to detailed docs)
- ❌ `docs/security/SECURITY.md` - Delete (consolidate into README)
- ✅ `crates/wallet/SECURITY.md` - Keep (wallet-specific)

**REPRODUCIBILITY.md** (2 copies):
- ✅ Root `/REPRODUCIBILITY.md` - Link to detailed docs
- ❌ `docs/REPRODUCIBILITY.md` - Delete
- ✅ `docs/security/REPRODUCIBILITY.md` - Main document

**RELEASE.md** (2 copies):
- ❌ `docs/RELEASE.md` - Delete
- ❌ `docs/guides/RELEASE.md` - Delete
- ✅ `docs/releases/process.md` - Consolidated release process

**Installation guides** (multiple):
- ❌ `docs/INSTALL_GUIDE.md` - Delete
- ❌ `docs/guides/INSTALL.md` - Delete
- ✅ `docs/getting-started/installation.md` - Main installation guide
- ✅ `docs/guides/MAINNET_INSTALLATION.md` → `docs/operations/mainnet-deployment.md`

**Multisig guides**:
- ❌ `docs/MULTISIG_GUIDE.md` - Delete
- ✅ `docs/guides/multisig.md` - Keep and improve
- ✅ `docs/wallet/multisig.md` - Technical details

### Phase 3: Reorganize docs/ Directory ✅

**Create new directories**:
```bash
mkdir -p docs/getting-started
mkdir -p docs/specifications
mkdir -p docs/api/rpc
mkdir -p docs/api/sdk
mkdir -p docs/security/audit-reports
mkdir -p docs/internal
```

**Move and rename files**:
- `docs/guides/QUICKSTART.md` → `docs/getting-started/quick-start.md`
- `docs/spec/*` → `docs/specifications/*`
- `docs/rpc/*` → `docs/api/rpc/*`
- `docs/cli/*` → `docs/api/cli/*`

### Phase 4: Update Master Index ✅

Create comprehensive `docs/README.md` with:
- Quick navigation to all major sections
- Link to getting started guide
- Link to API reference
- Link to security documentation

### Phase 5: Create Redirects/Links ✅

For moved files, leave a redirect file in the old location:
```markdown
# [Moved]

This document has been moved to: [new/location.md](new/location.md)
```

---

## ✅ Success Criteria

- [ ] Root directory has ≤ 10 MD files (only essential)
- [ ] All documentation organized into logical categories
- [ ] No duplicate files
- [ ] Clear navigation structure
- [ ] Updated links in all files
- [ ] Master index document created
- [ ] Old locations have redirect files

---

## 📝 File Inventory

### Root Level MD Files (Current: 20+)
1. ✅ Keep: README.md, CONTRIBUTING.md, CODE_OF_CONDUCT.md, SECURITY.md, CHANGELOG.md, FUNDING.md, ROADMAP.md
2. ❌ Move: All others (see Phase 1)

### docs/ Directory (Current: 180+ files)
- Needs complete reorganization into new structure

### Crate-specific (Current: ~20 files)
- ✅ Keep as is (crate-specific documentation is fine)

---

## 🎯 Timeline

- **Phase 1**: 1-2 hours (root cleanup)
- **Phase 2**: 1 hour (consolidate duplicates)
- **Phase 3**: 2-3 hours (reorganize docs/)
- **Phase 4**: 1 hour (master index)
- **Phase 5**: 1 hour (redirects)

**Total estimated time**: 6-8 hours

---

## 🚀 Next Steps

1. Get approval for this structure
2. Create backup branch before changes
3. Execute phases 1-5 systematically
4. Update all internal links
5. Test documentation site
6. Commit and push changes

---

*Generated: 2025-11-21*
*Status: Awaiting approval*
