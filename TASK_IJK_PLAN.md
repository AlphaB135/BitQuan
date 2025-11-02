# แผนงานรีสตาร์ท Tasks I/J/K (ความปลอดภัยเชิงลึก)

อัพเดต: 2025-11-02

## แนวทางการลงมือ (ทำทีละข้อ)
1. ปิด Task I (Checked Arithmetic) > รันเทส
2. ปิด Task J (Replay Protection) > รันเทส
3. ปิด Task K (Entropy Audit) > รันเทส
4. เก็บงานส่งท้าย + เอกสาร/CHANGELOG

---

## ✅ Task I – Checked Arithmetic Audit (เสร็จแล้ว)

ทำเสร็จ:
- [x] `crates/mempool/src/lib.rs` – error types + checked arithmetic ครบ
- [x] `crates/consensus/src/lib.rs` – block/tx weight `try_fold` + `checked_*`
- [x] `crates/consensus/src/utxo.rs` – นับ UTXO/fee ใช้ `checked_add/sub`
- [x] `crates/types/src/transaction.rs` – heuristic size ใช้ `checked_add` + `try_fold`
- [x] `crates/types/src/block.rs` – serialized size ปลอดภัย
- [x] `crates/node/src/main.rs` – สะสม balance/UTXO ด้วย `checked_add`
- [x] เพิ่ม unit/property tests สำหรับ overflow/underflow
- [x] `cargo test` สำหรับโมดูลที่เกี่ยวข้องผ่านทั้งหมด

---

## ✅ Task J – Replay Attack Prevention (เสร็จแล้ว)

ทำเสร็จ:
- [x] เพิ่ม `crates/types/src/context.rs` และเผยแพร่ใน `lib.rs`
- [x] อัปเดต `crates/consensus/src/sighash.rs` ให้ใช้ `TxContext` + domain separator
- [x] ปรับ `crates/node/src/tx_builder.rs` และ CLI ให้ส่ง context ขณะเซ็นธุรกรรม
- [x] ปรับ `crates/crypto/src/lib.rs` ให้ verify ผ่าน `TxContext`
- [x] เติม unit/integration tests (cross-network / genesis mismatch / builder)
- [x] `cargo test` ครอบคลุมทุก crate ที่เกี่ยวข้องผ่านทั้งหมด

---

## ⏳ Task K – Entropy Audit (กำลังดำเนินการ)

เสร็จแล้ว:
- [x] `crates/pqc-dilithium-seeded/src/randombytes.rs` เปลี่ยนเป็น `OsRng` และเพิ่มเทสทั้งหมด 8 รายการ

ค้างอยู่:
- [ ] ทบทวนจุดเรียกที่เกี่ยวข้อง (ensure deterministic build ฟีเจอร์ยังใช้ได้)
- [ ] เพิ่ม unit test เช็ค entropy พื้นฐาน (ต่าง seed ต่างค่า) ถ้ายังไม่ครอบคลุม
- [ ] อัปเดต `docs/ENTROPY_AUDIT.md`
- [ ] รัน `cargo test --all`

---

## 🔁 ขั้นตอนปิดงาน

1. `cargo fmt --all`
2. `cargo clippy --all-targets --all-features -- -D warnings`
3. `cargo test --all --locked`
4. อัปเดต `CHANGELOG.md` และรายการ TODO ให้ตรงสถานะ
5. เตรียม commit message + สรุปผลการเทส

---

> หมายเหตุ: หากพบ dependency ใหม่หรือ error type ที่ยังไม่มีใน crate ให้สร้างในโฟลเดอร์นั้น ๆ โดยยึดสไตล์ existing code. ทำทีละส่วน แล้วคอนเฟิร์มผลก่อนข้ามข้อถัดไป
