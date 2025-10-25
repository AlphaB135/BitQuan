# BitQuan Rust Code Structure (Phase 3 Scaffold)

เอกสารนี้สรุปโครงสร้างโค้ด Rust ที่ตั้งต้นในรีโพฯ เพื่อรองรับงาน Phase 3–5 ตามที่ระบุใน `todo.md` และเอกสารสถาปัตยกรรม ก่อนลงรายละเอียดในแต่ละโมดูลเพิ่มเติม

## ภาพรวม Workspace

- `Cargo.toml` ระดับรากกำหนด workspace และ metadata ร่วม (edition 2021, license, repository)
- โค้ดอยู่ภายใต้ `crates/` โดยแบ่งเป็น crate ตามขอบเขตฟังก์ชันชัดเจน พร้อมเตรียม dependency chain ดังนี้:

| Crate | ชนิด | พึ่งพา | หน้าที่หลัก |
|-------|------|--------|--------------|
| `bitquan-types` | lib | `serde` | โครงสร้างข้อมูลธุรกรรม/บล็อก, CompactUint, ฟังก์ชัน utility |
| `bitquan-crypto` | lib | `bitquan-types`, `thiserror` | Trait ของลายเซ็น PQC, registry สำหรับ verify, RNG service (OsRng→ChaCha20 + HKDF substreams), placeholder Dilithium |
| `bitquan-consensus` | lib | `bitquan-crypto`, `bitquan-types`, `thiserror` | คำนวณ block weight, ตรวจบล็อกขั้นต้น, เก็บค่าพารามิเตอร์ Phase 3 |
| `bitquan-mempool` | lib | `bitquan-consensus`, `bitquan-types`, `thiserror` | โครงร่าง mempool, การจัดลำดับตาม fee/weight |
| `bitquan-network` | lib | `bitquan-types`, `thiserror` | โครงสร้าง config และ service เบื้องต้นสำหรับ P2P |
| `bitquan-storage` | lib | `bitquan-types`, `thiserror` | Trait เก็บบล็อก + in-memory prototype |
| `bitquan-node` | bin | libs ทั้งหมดด้านบน, `anyhow`, `clap` | CLI จุดเริ่มรันโหนด, สาธิตการร้อย subsystem |

## แนวคิดการแยกโมดูล

1. **Data-first (`bitquan-types`)**: ลงรายละเอียด struct และ helper (เช่น CompactUint, SigAlgorithm) เพื่อเป็นสัญญาให้ทุก crate ใช้ร่วมกัน
2. **Crypto abstraction**: ใช้ trait `SignatureScheme` + `CryptoRegistry` สำหรับ plug-in ลายเซ็น (เริ่มด้วย Dilithium placeholder)
3. **Consensus**: ฟังก์ชัน `calculate_block_weight` ใช้ซ้ำใน mempool & validation, พร้อม `ConsensusParams::phase3_defaults()` ให้ค่าเริ่มต้นตรงกับเอกสาร
4. **Mempool / Network / Storage**: เก็บข้อมูลใน struct placeholder พร้อม TODO ชัดเจนสำหรับ Phase ต่อไป
5. **Node CLI**: ใช้ `clap` เพื่อเตรียม subcommand (`run`, `check-block`) และแสดงวิธีเชื่อม subsystems

## ใบงานถัดไป (Next Steps)

1. เติม logic verify Dilithium/Falcon/SPHINCS+ ผ่าน liboqs หรือ implementation ที่เลือก แล้วต่อกับ `CryptoRegistry`
2. นิยาม serialization ที่ตรงสเปกใน `bitquan-types` (ปัจจุบันเป็น heuristic) พร้อม test vectors
3. กำหนด `BlockId / TxId` ให้ชัดเพื่อเลี่ยง placeholder (เช่นใช้ prev_block เป็น key)
4. ต่อ mempool กับ consensus checks เพิ่มเติม (double-spend detection, ancestor limit)
5. เปิด event loop จริงใน `bitquan-node` (Tokio) และผูกกับ P2P transport

อัปเดตนี้ช่วยให้ทุกฟีเจอร์ใน Phase 3–4 เริ่มต้นได้จากโครงสร้างเดียวกัน และเอื้อต่อการ review/iterate แบบแยกโมดูลได้อย่างชัดเจน
