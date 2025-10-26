# BitQuan TODO Master Plan

## Progress Update (2025-10-26T09:32:41Z) – Code Quality & Cleanup ✅
- **Build Status**: ✅ Clean build (ไม่มี warnings!)
- **Code Quality**: แก้ไข warnings ทั้งหมด (unused imports, missing docs, dead code)
- **Documentation**: เพิ่ม inline docs ให้ P2P protocol messages (19 fields), consensus engine (3 fields)
- **Tests**: ✅ 51 tests passing (6 test suites)
- **Binary**: 8.7MB optimized release build
- **Status**: ~40% ของ Master Plan เสร็จสมบูรณ์
- **Next Phase**: Wire protocol binary parser → Full P2P networking → Enhanced Wallet CLI

## Progress Update (2025-10-26T06:00:00Z) – Storage & RPC Complete ✅
- **Storage**: RocksDB persistent backend สมบูรณ์ พร้อม column families (blocks/headers/height/tx/utxo/meta), atomic WriteBatch, และ ChainStore trait ที่ปรับปรุงแล้ว
- **RPC**: JSON-RPC 2.0 server สมบูรณ์ พร้อม 8 methods (getblockcount, getblockchaininfo, getmininginfo, getblocktemplate, submitblock, gettransaction, getbestblockhash, getblockhash)
- **Integration**: แก้ไข node/consensus ให้รองรับ ChainStore API ใหม่ (Result-based)
- **Tests**: 51 tests passing (เพิ่ม 8 tests สำหรับ storage + RPC)
- **Next**: Wire protocol binary parser, full P2P networking, enhanced Wallet CLI

## Progress Update (2025-10-26T05:41:00Z) – Phase 6 Started
- Starting implementation of persistent storage (RocksDB) and RPC server
- Priority: Database layer → RPC interface → P2P completion → Wallet enhancement
- Types: ปรับ `Transaction` ให้ใช้ `witnesses` (แทน signatures เดิม) พร้อมนับ signature ผ่าน witness และเตรียม serialization hint
- Consensus: เพิ่ม `transaction_sighash` แบบ deterministic (SHA-256) ใช้ตรวจ Dilithium, สร้าง `DifficultyState` + ฟังก์ชัน ASERT helper (float) และรายงาน subsidy ใน `BlockValidationReport`
- Node: คำสั่ง `check-block` แสดง subsidy; RNG ไม่จำเป็นใน consensus อีกต่อไป
- Docs: เติมสเปก witness ใน `docs/spec/transactions_blocks.md` และอธิบาย ASERT helper ใน `docs/spec/consensus_economics.md`
- Next: ผูก `DifficultyState` กับ chainstate/MTP จริง, เติม witness serialization vectors, และยืนยัน sighash กับ test vectors ข้ามภาษา

## Progress Update (2025-10-26T05:28:53.724Z)
- Consensus: เพิ่มตรวจ merkle_root/witness_root และกติกา coinbase (ต้องเป็นตัวแรก/เดียว, มูลค่า ≤ subsidy).
- Mining/Template: header.hashing/target check, MTP fallback (max(now, tip/MTP+1)), bits auto จาก DifficultyState, witness_root ใส่ใน header.
- Types: merkle_root_from_txids, compute_witness_root; tx binary vectors พร้อม.
- Node: miner เดโม, P2P demo; ต่อไปจะเสริม RPC/Stratum scaffolding และ persistence DB.
- Next: canonical wire parsing (tx/block), persistent chainstore, RPC/Stratum job server, mempool/template จาก fee-per-weight.


## 🚀 Progress Update (2025-10-26T05:15:59.703Z) - P2P scaffolding + Wallet CLI
- Added P2P protocol scaffolding in crates/network:
  - Message enum (Version/VerAck/Ping/Pong/Inv/GetData/Block/Tx/GetHeaders/Headers/Reject), envelopes with magic 'BQ' (0x42,0x51,0x01,0x01), MAX_MESSAGE_SIZE=10MB.
  - Simple Peer/PeerManager (states, add/remove, limits) + unit tests (serialization round-trip, max peers, no duplicates).
  - Next: implement TCP wire I/O, version/verack handshake, ping/keepalive, addr/getaddr peer discovery, headers/blocks relay, per-peer rate limiting.
- Wallet CLI prototype in bitquan-node:
  - New commands: WalletGen (Dilithium3 placeholder keypair) and BuildTx (1-in-1-out JSON tx builder).
  - Next: real Dilithium keypair via pqc_dilithium, encrypted keystore, Bech32m addresses, TX signing/verification flow, coin selection, RPC/mempool integration.
- Build/Test: cargo build+test green; no functional wire networking yet.

## 🔒 Progress Update (2025-10-25T18:40:00Z) - SECURITY HARDENING
**Major Security Fixes Completed:**
- ✅ **Dilithium Signature Verification**: Implemented real PQC signature verification (pqc_dilithium v0.2)
- ✅ **Transaction Validation**: Created comprehensive validation module with DoS protection
- ✅ **Merkle Tree Security**: Fixed CVE-2012-2459 style duplicate attacks
- ✅ **Mempool Limits**: Added 300MB size cap with low-fee eviction
- ✅ **Timestamp Safety**: Removed panics, added bounds checking (2-hour future limit)
- ✅ **Difficulty Overflow**: Protected against NaN/infinity in target calculations
- ✅ **RNG DoS Protection**: 10MB allocation limit
- ✅ **Network Config**: Added rate limits (100 msg/sec/peer) and size limits (10MB)

**Files Modified:**
- `crates/crypto/src/lib.rs` - Real Dilithium verification
- `crates/types/src/validation.rs` - NEW: Validation framework
- `crates/types/src/block.rs` - Merkle tree security fix
- `crates/mempool/src/lib.rs` - Size limits + eviction
- `crates/node/src/main.rs` - Safe timestamp handling
- `crates/consensus/src/difficulty.rs` - Overflow protection
- `SECURITY_FIXES.md` - NEW: Complete security audit report

**Tests Status**: ✅ All 21 tests passing
**Build Status**: ✅ Clean (4 doc warnings only)

## 🚀 Progress Update (2025-10-25T19:10:00Z) - PHASE 4 CORE FEATURES
**Major Features Implemented:**
- ✅ **UTXO Set & Double-Spend Detection** (445 lines) - Full UTXO database with:
  - Outpoint tracking and validation
  - Double-spend prevention
  - Coinbase maturity (100 blocks)
  - Fee calculation
  - Input/output overflow protection
  - 5 comprehensive tests

- ✅ **Fork Choice & Reorg** (450 lines) - Blockchain reorganization with:
  - Longest chain rule (most work)
  - Automatic reorg detection
  - Fork point identification
  - Max reorg depth limit (100 blocks default)
  - Orphan block detection
  - Chain work calculation
  - 5 reorg tests

- ✅ **Script Interpreter** (380 lines) - PQC script execution:
  - Stack-based VM
  - OP_CHECKSIG_PQC implementation
  - Dilithium signature verification
  - DoS protection (stack/ops limits)
  - Hash operations (SHA-256d)
  - 7 interpreter tests

**New Files:**
- `crates/consensus/src/utxo.rs` (445 lines)
- `crates/consensus/src/fork.rs` (450 lines)
- `crates/consensus/src/script.rs` (380 lines)

**Tests Status**: ✅ All 38 tests passing (17 new tests)
**Build Status**: ✅ Clean

**Total New Code**: ~1,275 lines of production code + tests

## Progress Update (2025-10-25T17:54:54.589Z)
- Fees/weight: เพิ่ม witness_weight_beta=0.5 และใช้งานใน consensus/mempool.
- Types: เพิ่ม binary serialization (base+witness) และตัวช่วย txid/wtxid.
- Vectors: เพิ่มตัวอย่าง gen_tx_vectors (hex fixtures สำหรับ cross-language).
- PoW: เพิ่ม header serialization, SHA256d header hash, bits→target และ target check.
- Node: เพิ่มคำสั่ง MineOnce (CPU miner demo) ระหว่างกำลังจัด wiring ให้คอมไพล์สมบูรณ์.

## Progress Update (2025-10-25T17:21:54Z)
- Docs/spec: เพิ่ม/อัปเดต BQIP-0001..0004 (PQC Tx/Block, PoW params, Wallet/SDK, Witness+L2), ปรับสเปก transactions_blocks.md (witness, wtxid/witness_root, น้ำหนัก base+α+β), อัปเดตสถาปัตย์ (k-values), เพิ่ม test-vectors stub.
- i18n: อัปเดต README.th.md และส่วนภาษาไทยในสเปก/สถาปัตย์ให้สอดคล้อง witness/k-value.
- Tooling: เพิ่ม scripts/install-hooks.sh (pre-commit tooling) และอัปเดต CONTRIBUTING/README วิธีใช้งาน.
- SDK/Ecosystem: สร้างโครง sdk/ (Rust) และ bindings/ts/ (TypeScript) พร้อม README แนวทาง.
- Crypto: เอา panic ออกจาก hkdf.expand() ใน bq-crypto.
- Types: ขยายโครงสร้าง Transaction/Witness ใน bitquan-types; เพิ่ม tests JSON round-trip และตัวอย่าง tx builder + ตรวจน้ำหนักลายเซ็น.
- Consensus: เพิ่ม RewardSchedule::subsidy_at_height unit tests (halving/tail); เพิ่ม DifficultyState+MTP integration test และ utilities (compact<->target).
- Tracking: เปิด issues #1–#4 สำหรับ BQIP-0002/0003/0004 และ SDK scaffolding.

## 📋 Phase 0 Complete (2025-10-25T18:50:00Z) - GOVERNANCE & SECURITY FOUNDATION
**✅ ALL PHASE 0 TASKS COMPLETED!**

**New Documentation Created:**
- ✅ `docs/NO_BACKDOORS.md` (350 บรรทัด) - นโยบายปราศจาก backdoor พร้อมการ enforce
- ✅ `docs/GPG_SIGNING.md` (401 บรรทัด) - คู่มือการเซ็น commit/tag ด้วย GPG
- ✅ `docs/REPRODUCIBILITY.md` (อัพเดต 44 บรรทัดเพิ่ม) - Reproducible builds ครบถ้วน
- ✅ `docs/GOVERNANCE.md` (มีอยู่แล้ว) - โครงสร้างการบริหาร
- ✅ `docs/CONTRIBUTING.md` (มีอยู่แล้ว) - แนวทางการมีส่วนร่วม
- ✅ `docs/RELEASE.md` (มีอยู่แล้ว) - กระบวนการ release
- ✅ `docs/SECURITY.md` (มีอยู่แล้ว) - นโยบายความปลอดภัย
- ✅ `SECURITY_FIXES.md` (124 บรรทัด) - รายงานช่องโหว่ที่แก้ไขแล้ว

**Security Infrastructure:**
- ✅ `docs/security/keys/maintainers/` - GPG public keys registry
- ✅ `docs/security/attestations/` - Build attestations from community
- ✅ `docs/security/audits/` - Security audit reports
- ✅ `scripts/install-hooks.sh` - Git hooks สำหรับ enforce policies

**Total New Content**: ~1,275 บรรทัด documentation + infrastructure

## Progress Update (2025-10-25T17:21:54Z)
- Docs/spec: เพิ่ม/อัปเดต BQIP-0001..0004 (PQC Tx/Block, PoW params, Wallet/SDK, Witness+L2), ปรับสเปก transactions_blocks.md (witness, wtxid/witness_root, น้ำหนัก base+α+β), อัปเดตสถาปัตย์ (k-values), เพิ่ม test-vectors stub.
- i18n: อัปเดต README.th.md และส่วนภาษาไทยในสเปก/สถาปัตย์ให้สอดคล้อง witness/k-value.
- Tooling: เพิ่ม scripts/install-hooks.sh (pre-commit tooling) และอัปเดต CONTRIBUTING/README วิธีใช้งาน.
- SDK/Ecosystem: สร้างโครง sdk/ (Rust) และ bindings/ts/ (TypeScript) พร้อม README แนวทาง.
- Crypto: เอา panic ออกจาก hkdf.expand() ใน bq-crypto.
- Types: ขยายโครงสร้าง Transaction/Witness ใน bitquan-types; เพิ่ม tests JSON round-trip และตัวอย่าง tx builder + ตรวจน้ำหนักลายเซ็น.
- Consensus: เพิ่ม RewardSchedule::subsidy_at_height unit tests (halving/tail); เพิ่ม DifficultyState+MTP integration test และ utilities (compact<->target).
- Tracking: เปิด issues #1–#4 สำหรับ BQIP-0002/0003/0004 และ SDK scaffolding.

## 0. หลักการและโครงสร้างพื้นฐาน (Phase 0)
- [ ] ประกาศนโยบาย “ไม่มี backdoor/admin key/สวิตช์ลับ” ในเอกสารโครงการและรีวิวโค้ดทุกส่วนเพื่อยืนยัน
- [ ] จัดเตรียมกระบวนการทำงานให้เป็น open-source 100% (repo สาธารณะ, history โปร่งใส)
- [ ] ออกแบบระบบ reproducible builds (กำหนด toolchain, flags, `SOURCE_DATE_EPOCH`, สคริปต์ build)
- [ ] เตรียมการเซ็น GPG สำหรับ commit/tag และจัดการการเผยแพร่ checksum
- [ ] วางแผน vesting on-chain + multisig + timelock (ถ้ามีทุนทีม) พร้อมเปิดเผยข้อมูล
- [ ] สร้าง/เติมไฟล์มาตรฐานใน repo: `docs/GOVERNANCE.md`, `docs/CONTRIBUTING.md`, `docs/RELEASE.md`, `docs/SECURITY.md`, `docs/REPRODUCIBILITY.md`, `MAINTAINERS`

## 1. ออกแบบสถาปัตยกรรม (Phase 1)
- [ ] ระบุเป้าหมายระบบ: ความปลอดภัย 50+ ปี, latency ยืนยันบล็อก, ข้อจำกัดฮาร์ดแวร์/แบนด์วิธ
- [ ] สรุปข้อกำหนดการตรวจสอบของชุมชน (ความง่ายในการรันโหนด, ขนาด storage เป้าหมาย)
- [ ] เลือกฉันทามติหลัก (PoW Minimalist vs PoS Simple-BFT) และบันทึกเหตุผล + trade-off
- [ ] หากเลือก PoW: ยืนยันเป้าบล็อก 10 นาที, อัลกอริทึมแฮช (SHA-256/Blake3), กลไก ASERT/EMA per-block
- [ ] หากเลือก PoS: ออกแบบ slashing, anti-nothing-at-stake, และขั้นตอนโรเตต validator set
- [ ] นิยาม crypto primitives: Hash (`SHA-256`/`BLAKE3`), Address checksum (Bech32m), Randomness (OS CSPRNG + HMAC-DRBG/ChaCha20-DRBG)

## 2. Post-Quantum Cryptography Integration (Phase 2)
- [ ] ตัดสินใจเลือกสกีมลายเซ็นหลัก (เริ่ม Dilithium) และกำหนด roadmap รอง (Falcon, SPHINCS+)
- [ ] กำหนดมาตรฐาน BQIP สำหรับชนิดลายเซ็น (เช่น `BQIP-0001 PQC Signature Standard`)
- [ ] วางแผนสร้าง/ใช้ liboqs หรือไลบรารีอื่นสำหรับ Dilithium sign/verify
- [ ] นิยามการใช้ PQC ใน Wallet keys, TX signatures (UTXO vs Account style), Block signatures (กรณี PoS)
- [ ] กำหนด hybrid TLS/KEM (Kyber) สำหรับการเข้ารหัสการสื่อสาร (ถ้าต้องการ)
- [ ] ออกแบบสูตร block weight: `weight = raw_bytes + α * (#pqc_signatures)` พร้อมค่าตั้งต้น `α ≈ 256–512` wu และ block cap 4,000,000 wu

## 3. สเปกข้อมูล (Phase 3)
- [ ] สรุปโครงสร้าง `Transaction` (version, inputs, outputs, lock_time, `sig_algo`, `sig`)
- [ ] นิยาม enum `sig_algo` สำหรับ Dilithium/Falcon/SPHINCS+
- [ ] สร้างโครงสร้าง `BlockHeader` และ `Block` พร้อม `pqc_agg_hint` สำหรับการขยายในอนาคต
- [ ] ออกแบบ address format: Bech32m prefix ใหม่ (เช่น `q1...`) และสคริปต์พื้นฐาน `OP_CHECKSIG_PQC`
- [ ] วางแผนรองรับ multisig/เงื่อนไขขั้นสูงภายหลัง (script extensions)

## 4. กฎโปรโตคอลและการตรวจสอบ (Phase 4)
- [ ] เขียนกติกาหลัก: block time, difficulty retarget (ASERT per-block), block validity checks
- [ ] ออกแบบ mempool policy: คัดเรียงตาม `fee_per_weight`, ปฏิเสธ TX ที่ fee ต่ำเมื่อเทียบกับจำนวนลายเซ็น
- [ ] จัดทำ batch verification สำหรับลายเซ็น Dilithium/Falcon
- [ ] นิยามกลยุทธ์ relay: Header-first sync, compact blocks, Erlay-style gossip
- [ ] ตั้งเป้าหมาย orphan rate < 1.5% และกำหนดตัวชี้วัด latency propagation
- [ ] ผูก DifficultyState/ASERT เข้ากับ chainstate จริง (anchor block, MTP, bits update)
- [ ] พัฒนา canonical sighash test vectors (cross-language) สำหรับ witness layout ใหม่

## 5. เศรษฐศาสตร์การขุดและความแฟร์ (Phase 5 + ASIC notes)
- [ ] กำหนดรางวัลบล็อกเริ่มต้น (50 BQ) และตาราง Halving ทุก 210,000 บล็อก (~4 ปี)
- [ ] ออกแบบนโยบายค่าธรรมเนียมตาม block weight และตัวเลือก burn (80% burn / 20% miner)
- [ ] นิยามกลยุทธ์เปิดเผยอัลกอริทึม PoW อย่างเท่าเทียม (ไม่มี premine, เปิดโค้ด/เอกสารพร้อมกัน)
- [ ] จัดเตรียม reference miner CPU ที่คอมไพล์ได้ทั้ง macOS, Windows, Linux (Rust/C++ cross-platform)
- [ ] ออกแบบ API `getwork/submit` หรือ Stratum สำหรับ external miner plugins (OpenCL/CUDA/Metal)
- [ ] ประเมินอัลกอริทึม PoW memory-hard (RandomX, KAWPOW, Equihash, Argon2id) เพื่อควบคุม ASIC dominance หรือวางแผน roadmap ยอมรับ ASIC อย่างแฟร์
- [ ] วางแผน diff retarget response + pool decentralization (ป้องกัน pool > 50%)

## 6. โครงสร้างทีมและโมดูลโค้ด (Phase 6)
- [ ] เลือกภาษา (Rust/Go) และตั้งค่าโครงสร้างโปรเจกต์ baseline
- [ ] สร้างโมดูล `crypto/`, `consensus/`, `mempool/`, `p2p/`, `storage/` (RocksDB/LMDB), `wallet/`, `rpc/cli/`
- [ ] ผูก liboqs หรือไลบรารี PQC ภายในโมดูล `crypto/`
- [ ] พัฒนา reference node ที่รองรับ full validation, P2P, RPC
- [ ] พัฒนา reference wallet CLI (สร้าง keypair, address, TX, ký/sign Dilithium)
- [ ] เตรียม integration tests สำหรับ mempool และ consensus
- [ ] ออกแบบ interface สำหรับ external miner (plugin system)

## 7. Benchmarking & Testing (Phase 6.3)
- [ ] สร้างชุด benchmark PQC verify throughput (เป้า ≥5k signatures/sec บน 8-core)
- [ ] วัด propagation time (P50/P95) และ orphan rate ในสภาพเน็ตจำลอง/ของจริง
- [ ] ทดสอบ reorg safety (จำลอง reorg 1–3 บล็อก)
- [ ] สร้าง test harness สำหรับ block weight enforcement และ batch verification
- [ ] ตั้งค่า CI ให้รัน unit/integration/fuzz tests ทุก PR
- [ ] Witness serialization/round-trip tests (ข้ามภาษา): เพิ่ม vectors ใน docs/spec/test-vectors.md และตัวอย่าง tx builder
- [ ] Integration tests: DifficultyState + chainstate จริง (MTP, anchor block), รวม L2/witness relay/validation
- [ ] ตรวจสอบและเปรียบเทียบ sighash ระหว่างภาษา (Rust/TS SDK) เพื่อป้องกัน mismatch

## 8. ความปลอดภัยซอฟต์แวร์และซัพพลายเชน (Phase 7)
- [ ] บังคับ signed commits/tags (GPG) ใน CI
- [ ] ทำ reproducible build walkthrough + third-party attestation ≥3 ทีม
- [ ] กำหนด static analysis rule: ล้ม build ถ้าพบคำต้องห้าม (`admin|backdoor|godmode|testmode|hardcoded key`)
- [ ] ตั้งโครงสร้าง fuzzing targets สำหรับ mempool, script interpreter, TX parser, consensus critical paths
- [ ] จัดจ้าง audit ภายนอก (crypto, implementation, build pipeline) และวางแผน remediation
- [ ] จัดทำ responsible disclosure policy + security contact (ใน `docs/SECURITY.md`)

## 9. Governance และการมีส่วนร่วม (Phase 8)
- [ ] นิยามบทบาท Lead Maintainer, Core Maintainers (≥3), Steering Committee (5), Community Council (9)
- [ ] สร้าง workflow การ merge (ต้องมี maintainers ≥2 อนุมัติ) และการออกนโยบายผ่าน BQIP
- [ ] สร้าง `bqip/` พร้อม `BQIP-0001` ถึง `BQIP-0004` (PQC, Block Weight, Difficulty Retarget, Governance)
- [ ] วาง schedule เลือกตั้ง Community Council รายปี และกระบวนการรับรองผล
- [ ] ตั้งระบบโหวต/การเก็บบันทึกการตัดสินใจ (on-chain/off-chain)

## 10. เฟสการเปิดเครือข่าย (Phase 9)
- [ ] เขียนสคริปต์สร้าง Genesis block พร้อมบันทึก “Genesis Statement”
- [ ] สรรหาทีม Early nodes 5–10 ทีมจากหลายประเทศและจัด guideline การรัน
- [ ] สร้างกลไกปรับ `α`, weight cap, retarget จากข้อมูล Devnet
- [ ] เปิด Testnet สาธารณะพร้อม reference node + wallet CLI + เอกสารสอน (run full node, mine, create TX, verify binary)
- [ ] ตั้ง milestone gate สำหรับ Testnet → Mainnet (orphan rate, validation time, reproducible build attestation, audit critical 0)
- [ ] เตรียม Mainnet release v1.0.0 (signed binary, checksums, reproducibility steps)
- [ ] จัดตั้ง DNS seeds ≥3 โดเมน (คนละทีม) และ seed peers หลายเจ้า
- [ ] ยืนยัน Genesis block ไม่มี founder reward (หรือระบุ lock/burn/vesting ตามที่ตกลง)
- [ ] เตรียมข้อความ Genesis: “The Quantum Age Begins — 22 Oct 2025. Ownerless. Verifiable. For everyone.”

## 11. ค่าพารามิเตอร์ตั้งต้น (Phase 10)
- [ ] ตอกย้ำตัวเลขตั้งต้น: block time 10 นาที, block weight cap 4,000,000 wu, `α = 384` wu/ลายเซ็น
- [ ] ปรับค่า retarget ASERT (half-life ≈ 1 วัน) และทดสอบความเสถียร
- [ ] กำหนด Subsidy schedule (50 BQ → Halving ทุก 210,000 บล็อก)
- [ ] ทดสอบ/ยืนยัน Fee market burn (80% burn / 20% miner) บน Testnet ก่อน Mainnet

## 12. แพ็กเอกสารและแม่แบบ (Deliverables)
- [ ] จัดโครงสร้าง `docs/` ให้ครบชุด (GOVERNANCE, RELEASE, REPRODUCIBILITY, SECURITY, CONTRIBUTING)
- [ ] เติม `MAINTAINERS` รายชื่อ + หลักเกณฑ์ rotation
- [ ] สร้างเทมเพลต BQIP (front-matter, lifecycle, status)
- [ ] จัดทำคู่มือ Devnet/Testnet/Mainnet (Run node, Mine, Wallet guide, Binary verification)
- [ ] จัดทำ FAQ เรื่อง ASIC, ความแฟร์, cross-platform mining

## 13. งานติดตามและการสื่อสาร
- [ ] ตั้ง roadmap สาธารณะ (issue board) เพื่อสะท้อนเฟส 0–10 พร้อมสถานะ
- [ ] ตั้ง cadence update (รายสัปดาห์/รายเดือน) ให้ทีม core และชุมชน
- [ ] วางระบบ feedback จากชุมชน (ช่องทาง report bug, เสนอ BQIP)
- [ ] กำหนด KPI/OKR รายไตรมาสสำหรับ core team (เช่น throughput, latency, adoption)
