# 🚀 BitQuan Mainnet Launch Plan
**Last Updated:** 2025-11-08  
**Current Status:** 🟡 30-40% Ready (Pre-Alpha)

---

## ✅ FIXED TODAY

### RocksDB Build Error ✅
- **Problem:** Compilation failed - incompatible rocksdb crate version
- **Root Cause:** rocksdb 0.22 incompatible with RocksDB 10.7.5
- **Solution Applied:**
  1. Installed RocksDB 10.7.5 via Homebrew
  2. Upgraded rocksdb crate: `0.22 → 0.23`  
  3. Set env vars: `ROCKSDB_LIB_DIR`, `ROCKSDB_INCLUDE_DIR`
- **Status:** ✅ **RESOLVED** - Build completes successfully

---

## 📊 Current Status

| Metric | Value | Target | Gap |
|--------|-------|--------|-----|
| **Production unwraps** | 344 | < 50 | -294 |
| **Test coverage** | TBD | 80%+ | TBD |
| **Clippy warnings** | 0 | 0 | ✅ |
| **Dead code warnings** | ~10 funcs | 0 | -10 |
| **Benchmarks** | 0 | 3+ | -3 |
| **Metrics endpoint** | ❌ | ✅ | Missing |
| **Security score** | 65/100 | 85+ | -20 |
| **Overall readiness** | 35% | 95%+ | -60% |

---

## 🎯 6-Week Mainnet Roadmap

### **Week 1: Critical Security (Nov 11-15)**
**Goal:** Security score 65 → 75

#### Monday-Tuesday: Wallet Unwrap Elimination
- [ ] Fix `crates/wallet/src/multisig.rs` (37 unwraps → 0)
- [ ] Fix `crates/wallet/src/keystore.rs` (est. 15 unwraps → 0)
- [ ] Test: All wallet operations handle errors gracefully
- **Target:** -52 unwraps

#### Wednesday-Thursday: Node Unwrap Elimination  
- [ ] Fix `crates/node/src/mnemonic.rs` (32 unwraps → 0)
- [ ] Fix `crates/node/src/tx_builder.rs` (est. 10 unwraps → 0)
- [ ] Test: CLI commands don't panic
- **Target:** -42 unwraps

#### Friday: Consensus Critical Paths
- [ ] Fix `crates/consensus/src/fork.rs` (27 unwraps → 0)
- [ ] Fix `crates/consensus/src/sighash.rs` (20 unwraps → 0)
- **Target:** -47 unwraps

**Week 1 Total:** 344 → 203 unwraps (41% reduction)

---

### **Week 2: Security Deep Dive (Nov 18-22)**
**Goal:** Security score 75 → 85

#### Monday-Tuesday: Mempool & Network
- [ ] Fix `crates/mempool/src/lib.rs` (21 unwraps → 0)
- [ ] Fix `crates/network/src/peer.rs` (est. 15 unwraps → 0)
- **Target:** -36 unwraps

#### Wednesday: Constant-Time Crypto
- [ ] Audit all signature comparisons
- [ ] Replace `==` with `subtle::ConstantTimeEq` for:
  - Dilithium signatures
  - HMAC outputs
  - Password verifications
- [ ] Add tests for timing consistency

#### Thursday-Friday: Overflow Testing
- [ ] Add overflow tests for:
  - Fee calculations
  - Block weight calculations
  - Supply calculations
  - Timestamp arithmetic
- [ ] Fuzzing setup (optional)

**Week 2 Total:** 203 → 167 unwraps (51% total reduction)

---

### **Week 3: Observability (Nov 25-29)**
**Goal:** Performance 68 → 85, Metrics 68 → 90

#### Monday: Benchmarks Setup
- [ ] Create `benches/` directory
- [ ] Add `criterion` to dev-dependencies
- [ ] Benchmark: `validate_block` (consensus)
- [ ] Benchmark: `sign` and `verify` (crypto)
- [ ] Benchmark: `add_tx` (mempool)
- [ ] Document baseline results

#### Tuesday: Metrics Endpoint
- [ ] Add `prometheus` dependency
- [ ] Implement `/metrics` in RPC server:
  - `bitquan_blocks_total`
  - `bitquan_transactions_total`
  - `bitquan_mempool_size`
  - `bitquan_peers_connected`
  - `bitquan_sync_height`
  - `bitquan_block_validation_seconds` (histogram)
- [ ] Test with `curl http://localhost:8332/metrics`

#### Wednesday-Friday: Dead Code Cleanup
- [ ] Remove OR document `MinerMetrics` unused functions
- [ ] Remove OR document `HybridMiner` unused functions
- [ ] Fix lifetime warnings in `peer.rs`
- [ ] Cargo clippy: 0 warnings target

**Week 3 Result:** Production monitoring ready

---

### **Week 4: Documentation Sprint (Dec 2-6)**
**Goal:** Documentation 72 → 85

#### Monday-Tuesday: Architecture Docs
- [ ] Create `docs/ARCHITECTURE.md`:
  - System overview diagram
  - Crate dependency graph
  - Data flow (TX → Block → Chain)
  - Design decisions (Why UTXO? Why Dilithium?)
- [ ] Create `docs/specs/consensus.md`
- [ ] Create `docs/specs/block_weight.md`

#### Wednesday: Getting Started Guide
- [ ] Create `docs/guides/GETTING_STARTED.md`:
  - Installation (all 3 OS)
  - First wallet creation
  - Receiving first coins
  - Sending transaction
  - Running a node
- [ ] Create `docs/guides/MINING_GUIDE.md`

#### Thursday-Friday: Code Examples
- [ ] Create `examples/create_transaction.rs`
- [ ] Create `examples/validate_block.rs`
- [ ] Create `examples/rpc_client.rs`
- [ ] Test: All examples compile and run
- [ ] Add doc comments: 50/150 → 120/150 functions

**Week 4 Result:** Developer-friendly documentation

---

### **Week 5: Configuration & Testing (Dec 9-13)**
**Goal:** Config 78 → 88, Testing 80 → 90

#### Monday: Config Examples
- [ ] Create `config/mainnet.toml.example`
- [ ] Create `config/testnet.toml.example`
- [ ] Create `config/devnet.toml.example`
- [ ] Create `config/regtest.toml.example`
- [ ] Add validation for config loading

#### Tuesday-Wednesday: Regtest Network
- [ ] Implement regtest network parameters:
  - Instant mining (difficulty = 1)
  - No P2P (localhost only)
  - 1-second blocks
- [ ] Test regtest end-to-end
- [ ] Document regtest usage

#### Thursday-Friday: Test Coverage
- [ ] Run `cargo tarpaulin --all-features`
- [ ] Add missing tests to reach 80% coverage
- [ ] Integration tests for critical paths

**Week 5 Result:** Complete network suite (mainnet/testnet/devnet/regtest)

---

### **Week 6: Final Polish & Testnet (Dec 16-20)**
**Goal:** Overall 85%+ ready, Testnet deployed

#### Monday: Final Security Pass
- [ ] Review remaining unwraps: should be < 50
- [ ] Security checklist review
- [ ] CHANGELOG.md update for v0.1.0-beta

#### Tuesday: Testnet Genesis
- [ ] Create testnet genesis block
- [ ] Deploy first testnet node
- [ ] Document testnet parameters:
  - Network ID: `testnet`
  - Ports: 18443 (RPC), 18444 (P2P)
  - Faster blocks (2 minutes)
  - Lower difficulty

#### Wednesday-Thursday: Testnet Hardening
- [ ] Run stress tests on testnet
- [ ] Fix any bugs found
- [ ] Monitor metrics for 48 hours

#### Friday: Release Preparation
- [ ] Git tag `v0.1.0-beta`
- [ ] Build binaries (Linux, macOS, Windows)
- [ ] Create checksums + GPG signatures
- [ ] Write release notes
- [ ] Draft announcement blog post

**Week 6 Result:** Testnet live, beta release ready

---

## 📅 Extended Timeline (Weeks 7-12)

### **Weeks 7-8: External Security Audit**
- Engage professional security auditor
- Provide codebase + documentation
- Wait for initial findings
- **Cost estimate:** $5,000-$15,000

### **Weeks 9-10: Audit Remediation**
- Fix critical findings
- Fix high-priority findings
- Retest and verify fixes
- Final audit report

### **Weeks 11-12: Mainnet Preparation**
- Testnet must run 6+ weeks without issues
- Community testing and feedback
- Mainnet genesis preparation
- Marketing materials
- Exchange outreach (optional)

### **Week 13: Mainnet Launch 🎉**
- Mainnet genesis: Block 0
- Announce publicly
- Monitor 24/7 for first week
- Bug bounty program active

---

## 🚨 Go/No-Go Criteria for Mainnet

### ✅ Must Have (100% required)
1. **Security**
   - [ ] Production unwraps < 50
   - [ ] Zero clippy warnings
   - [ ] Constant-time crypto ops
   - [ ] External audit: no critical findings

2. **Stability**
   - [ ] Testnet: 6+ weeks uptime
   - [ ] Testnet: 1,000+ blocks mined
   - [ ] Testnet: 10+ independent nodes
   - [ ] Zero consensus failures

3. **Observability**
   - [ ] /metrics endpoint working
   - [ ] Benchmarks documented
   - [ ] Monitoring dashboard ready

4. **Documentation**
   - [ ] ARCHITECTURE.md complete
   - [ ] GETTING_STARTED.md tested
   - [ ] API docs 80%+ coverage

5. **Testing**
   - [ ] Test coverage 80%+
   - [ ] Integration tests passing
   - [ ] Stress tests passing

### ⚠️ Should Have (80% required)
- [ ] Blog post published
- [ ] Video tutorial created
- [ ] Community Discord/Telegram active (100+ members)
- [ ] Code review by independent Rust expert
- [ ] Bug bounty program configured

### 🟢 Nice to Have (optional)
- [ ] Exchange listing secured
- [ ] Academic paper submitted
- [ ] Conference presentation scheduled
- [ ] Wallet GUI (can be post-mainnet)

---

## 🎯 Success Metrics (3 Months Post-Mainnet)

### Technical Success
- [ ] 100,000+ blocks mined
- [ ] 50+ independent nodes worldwide
- [ ] 0 protocol-level exploits
- [ ] 0 loss of funds incidents
- [ ] 99%+ uptime

### Community Success
- [ ] 1,000+ GitHub stars
- [ ] 25+ contributors
- [ ] 500+ active addresses
- [ ] 10+ projects building on BitQuan

### Adoption Success
- [ ] 1+ exchange listing (optional)
- [ ] Academic citations (2+)
- [ ] Media coverage (5+ articles)
- [ ] Developer adoption (10+ apps)

---

## 📞 Emergency Contacts

**Security Issues:**  
- Email: security@bitquan.org (if exists)
- GitHub: Security Advisories (private)

**Build Issues:**  
- GitHub Issues: Public tracker
- CI/CD: GitHub Actions logs

**Community:**  
- Discord/Telegram: TBD
- Twitter: TBD

---

## 📝 Daily Checklist (During Intensive Period)

### Every Morning
- [ ] `git pull origin main`
- [ ] `cargo build --release` (verify still compiles)
- [ ] Check GitHub issues/PRs
- [ ] Review security alerts (Dependabot)

### Every Evening
- [ ] `cargo test --all`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `git commit -S` (GPG signed)
- [ ] Update progress tracker
- [ ] Push to GitHub

### Every Week
- [ ] Run benchmarks, compare to baseline
- [ ] Review test coverage
- [ ] Security checklist review
- [ ] Documentation updates
- [ ] Community engagement

---

## 🎬 Next Immediate Actions (This Weekend)

1. **Set RocksDB env vars permanently** ✅
   ```bash
   export ROCKSDB_LIB_DIR=/opt/homebrew/lib
   export ROCKSDB_INCLUDE_DIR=/opt/homebrew/include
   ```

2. **Start Week 1 prep**
   - [ ] Read through wallet/multisig.rs
   - [ ] Create branch: `fix/week1-wallet-unwraps`
   - [ ] Map out all 37 unwraps and fix strategy

3. **Set up tracking**
   - [ ] Create GitHub Project board
   - [ ] Create issues for each week's tasks
   - [ ] Set up daily standup notes (markdown)

---

**Estimated Total Time to Mainnet:**  
- **Optimistic:** 6 weeks (if full-time)
- **Realistic:** 10 weeks (if part-time + waiting for audit)  
- **Conservative:** 16 weeks (with delays and iterations)

**Next Milestone:** Week 1 complete (Nov 15) - Security 65→75

---

*"Better to delay mainnet 3 months than launch with security holes."*

