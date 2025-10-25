<details open>
<summary>ภาษาไทย (Thai)</summary>

# ภาพรวมสถาปัตยกรรม BitQuan (Phase 1)

## 1. เป้าหมายหลักของระบบ
- **ความมั่นคงยาวนาน 50+ ปี**: ออกแบบให้ทนทานต่อการคำนวณยุคควอนตัมและการเปลี่ยนแปลงของฮาร์ดแวร์ระยะยาว
- **Latency การยืนยันบล็อกต่ำ**: ตั้งเป้าการกระจายบล็อกทั่วเครือข่าย < 5 วินาที (P50) และการคอนเฟิร์ม 1 บล็อกภายใน ~10 นาที
- **ความโปร่งใสและพิสูจน์ย้อนกลับได้**: ทุกส่วนของโค้ด เอกสาร และกระบวนการ build ต้องตรวจสอบย้อนกลับและทำซ้ำได้
- **ใช้งานได้บนฮาร์ดแวร์ทั่วไป**: โหนดเต็มทำงานได้บนเครื่อง 8 คอร์ / RAM 16 GB / SSD 1 TB และแบนด์วิธ 100 Mbps
- **โหมดออฟไลน์/ซิงค์ช้า**: รองรับโหนดในพื้นที่แบนด์วิธต่ำด้วยการซิงค์แบบ header-first และค่อยเติมข้อมูลภายหลัง

## 2. ข้อกำหนดจากชุมชนผู้ตรวจสอบ
- **ความง่ายในการรันโหนด**: ต้องสามารถรันบน Linux/macOS/Windows โดยใช้สคริปต์ตั้งค่าเดียวกัน และรองรับ container image สำหรับบริการคลาวด์
- **ขนาด Storage เป้าหมาย**: 4 TB หลังจาก 10 ปี (รวม chainstate และดัชนี) ด้วยนโยบาย block weight และการตัด pruning ที่กำหนด
- **ระบบนิยามมาตรฐานการตรวจสอบ**: จัดทำคู่มือ `docs/validation-guide.md` เพื่อช่วยผู้ตรวจสอบตั้งค่าซอฟต์แวร์และเครื่องมือวิเคราะห์บล็อก
- **Telemetry แบบเลือกเปิด**: ไม่มีการส่งข้อมูลกลับโดยตั้งค่าเริ่มต้น แต่มีตัวเลือก opt-in สำหรับเก็บสถิติการทดสอบ

## 3. วิเคราะห์ฉันทามติ
### ตัวเลือกที่พิจารณา
1. **Proof-of-Work Minimalist**
   - ข้อดี: โมเดลความปลอดภัยพิสูจน์แล้ว, เข้าใจง่าย, ลดความเสี่ยง Nothing-at-Stake
   - ข้อเสีย: ใช้พลังงานสูง, ตลาด ASIC มีผลต่อความแฟร์, ต้องวางมาตรการควบคุมศูนย์กลาง pool
2. **Proof-of-Stake Simple-BFT**
   - ข้อดี: พึ่งพาทรัพยากรพลังงานต่ำกว่า, ปรับเปลี่ยนพารามิเตอร์ง่าย, สามารถผสาน slashing/anti-equivocation
   - ข้อเสีย: ความซับซ้อนสูงในเชิงโค้ด, ความเสี่ยง long-range attack, ต้องบริหารคีย์ PQC สำหรับ validator จำนวนมาก

### ข้อเสนอเบื้องต้น
- เริ่มต้นด้วย **PoW Minimalist + ASERT per-block** เพื่อยึดโยงกับโมเดลความปลอดภัยที่เข้าใจง่ายในระยะเริ่มต้น
- จัดทำ roadmap PoS ไฮบริดเป็นทางเลือกอนาคตเมื่อชุมชนพร้อม และสร้าง BQIP เฉพาะสำหรับการเปลี่ยนผ่าน

## 4. พารามิเตอร์ฉันทามติ (เบื้องต้น)
- **เวลาเป้าบล็อก**: 10 นาที
- **อัลกอริทึมแฮช**: `SHA-256d` ในเวอร์ชันเริ่มต้น พร้อมแผนสำรอง `BLAKE3` หากมีความจำเป็นด้านประสิทธิภาพ
- **กลไกปรับความยาก**: ASERT ต่อบล็อก (half-life ≈ 1 วัน) เพื่อควบคุม hash rate shock
- **Block Weight Cap**: 4,000,000 weight units (wu)
- **ค่าสัมประสิทธิ์น้ำหนักลายเซ็น (`α`)**: เริ่มต้นที่ 384 wu ต่อ PQC signature

## 5. นิยาม Crypto Primitives
- **ลายเซ็นหลัก**: CRYSTALS-Dilithium (ระดับความปลอดภัย 3) สำหรับธุรกรรมและบล็อก
- **ทางเลือกในอนาคต**: Falcon สำหรับกรณีต้องการลายเซ็นขนาดเล็ก, SPHINCS+ สำหรับ fallback ที่อาศัย hash-based
- **Address Checksum**: รูปแบบ Bech32m prefix `bq1`
- **Randomness**: OS CSPRNG ผสาน HMAC-DRBG หรือ ChaCha20-DRBG สำหรับ deterministic wallet seed
- **ไฮบริด TLS/KEM**: หากต้องการการเข้ารหัสการสื่อสาร ให้ใช้ Kyber (ระดับ 768) ควบคู่กับ TLS 1.3

## 6. โครงสร้างองค์ประกอบระบบ
- **โมดูล Crypto**: ครอบคลุม Dilithium verify/sign, ฮาร์ดแวร์เร่งความเร็ว, และ binding กับ liboqs
- **โมดูล Consensus**: กฎบล็อก, difficulty retarget, ตรวจสอบบล็อก/ธุรกรรม, batch verification
- **โมดูล P2P**: โปรโตคอล gossip, compact block, header-first, anti-DoS (peer scoring, rate limiting)
- **โมดูล Storage**: ใช้ RocksDB/LMDB สำหรับ chainstate + UTXO set, ออกแบบ snapshot และ pruning
- **โมดูล Wallet**: CLI สำหรับ key management, address derivation, transaction builder รองรับ PQC
- **โมดูล RPC/CLI**: API สำหรับโหนดเต็มและสคริปต์การทำงานของ miner/reference wallet

## 7. มาตรวัดและความเสี่ยง
- **Latency propagation**: เฝ้าดู P50/P95 และ orphan rate < 1.5% บน testnet
- **Throughput ลายเซ็น PQC**: เป้าหมาย verify ≥ 5,000 ลายเซ็น/วินาทีบน CPU 8 คอร์
- **ความเสี่ยง ASIC**: จัดทำแผนประเมิน RandomX / KAWPOW / Equihash / Argon2id และกำหนดจุดตัดสินใจชัดเจนใน roadmap
- **ความเสี่ยง Supply Chain**: ยึดนโยบาย reproducible builds, dependency pinning, และ signed commits
- **เอกสารรายละเอียด**: ดู `docs/spec/consensus_economics.md` สำหรับ weight, tail emission และโมเดลค่าธรรมเนียม

## 8. งานต่อเนื่อง
- เขียน BQIP ชุดแรก (0001–0004) ให้สอดคล้องกับข้อเสนอข้างต้น
- สร้างเอกสาร threat model สำหรับ PoW + Dilithium เพื่อลดช่องโหว่ในช่วง Testnet
- เตรียมชุด benchmark สำหรับ Dilithium (sign/verify, batch) และ P2P propagation simulator

</details>

<details>
<summary>English</summary>

# BitQuan Architecture Overview (Phase 1)

## 1. System Objectives
- **50+ Year Security Horizon**: Engineer the protocol to withstand post-quantum computation and long-term hardware shifts.
- **Low Confirmation Latency**: Target block propagation across the network in < 5 seconds (P50) and single-block confirmation near 10 minutes.
- **Transparency and Auditability**: Every component of code, documentation, and build process must be reproducible and verifiable.
- **Commodity Hardware Friendly**: Full nodes run on 8-core CPUs, 16 GB RAM, 1 TB SSD, and 100 Mbps bandwidth.
- **Offline / Slow Sync Mode**: Support bandwidth-constrained operators via header-first sync followed by deferred block fetches.

## 2. Community Validation Requirements
- **Ease of Node Operation**: Must run on Linux/macOS/Windows with a shared setup script and offer container images for cloud deployments.
- **Target Storage Footprint**: 4 TB after 10 years (chainstate + indices) enabled by block weight policy and controlled pruning.
- **Standardized Validation Procedures**: Provide `docs/validation-guide.md` to help auditors configure software and analysis tooling.
- **Opt-in Telemetry**: Default to no data egress; optional telemetry can be enabled for testing metrics.

## 3. Consensus Analysis
### Candidates Considered
1. **Proof-of-Work Minimalist**
   - Pros: Battle-tested security model, straightforward implementation, mitigates Nothing-at-Stake concerns.
   - Cons: High energy footprint, ASIC market centralization risks, requires pool decentralization safeguards.
2. **Proof-of-Stake Simple-BFT**
   - Pros: Lower energy reliance, flexible parameter tuning, supports slashing and anti-equivocation mechanisms.
   - Cons: Higher implementation complexity, susceptible to long-range attacks, demands large-scale PQC key management for validators.

### Initial Recommendation
- Launch with **Minimalist PoW + per-block ASERT** to anchor security in a well-understood model.
- Draft a hybrid PoS roadmap as a future option once community readiness is established, backed by dedicated transition BQIPs.

## 4. Preliminary Consensus Parameters
- **Block Target Time**: 10 minutes.
- **Hash Algorithm**: `SHA-256d` initially, with `BLAKE3` contingency if throughput or efficiency requires.
- **Difficulty Retarget**: Per-block ASERT (half-life ≈ 1 day) to manage hash rate shocks.
- **Block Weight Cap**: 4,000,000 weight units (wu).
- **Signature Weight Coefficient (`α`)**: Start at 384 wu per PQC signature.

## 5. Crypto Primitives Definition
- **Primary Signature Scheme**: CRYSTALS-Dilithium Level 3 for transactions and block validation.
- **Future Options**: Falcon for smaller signatures, SPHINCS+ for hash-based fallback.
- **Address Checksum**: Bech32m prefix `bq1`.
- **Randomness**: OS CSPRNG combined with HMAC-DRBG or ChaCha20-DRBG for deterministic wallet seeds.
- **Hybrid TLS/KEM**: Employ Kyber (security level 768) with TLS 1.3 when encrypted communication is required.

## 6. System Component Layout
- **Crypto Module**: Dilithium sign/verify, hardware acceleration hooks, liboqs bindings.
- **Consensus Module**: Block rules, difficulty retarget, block/transaction validation, batch verification.
- **P2P Module**: Gossip protocol, compact blocks, header-first sync, anti-DoS (peer scoring, rate limiting).
- **Storage Module**: RocksDB/LMDB for chainstate and UTXO sets, plus snapshot and pruning strategies.
- **Wallet Module**: CLI tooling for key management, address derivation, PQC-aware transaction builder.
- **RPC/CLI Module**: Full node APIs and reference miner/wallet automation scripts.

## 7. Metrics and Risks
- **Propagation Latency**: Track P50/P95 and ensure orphan rate < 1.5% on testnet.
- **PQC Signature Throughput**: Target ≥ 5,000 verifications per second on 8-core CPUs.
- **ASIC Risk**: Evaluate RandomX / KAWPOW / Equihash / Argon2id and define decision checkpoints in the roadmap.
- **Supply Chain Risk**: Enforce reproducible builds, dependency pinning, and signed commits.
- **Further Reading**: See `docs/spec/consensus_economics.md` for weight policy, tail emission, and fee modeling.

## 8. Follow-up Tasks
- Author initial BQIPs (0001–0004) aligned with these recommendations.
- Produce a threat model covering PoW + Dilithium to harden the testnet phase.
- Prepare benchmarking suites for Dilithium (sign/verify, batch) and a P2P propagation simulator.

</details>
