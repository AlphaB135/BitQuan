# BitQuan TODO Master Plan

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

