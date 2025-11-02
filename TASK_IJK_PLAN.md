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

## ⏳ Task J – Replay Attack Prevention (กำลังดำเนินการ)

ลำดับแนะนำ
1. เพิ่มไฟล์ `crates/types/src/context.rs` (นิยาม `NetworkId`, `TxContext`)
2. เปิดเผยใน `crates/types/src/lib.rs`
3. อัปเดต `crates/consensus/src/sighash.rs`
   - ให้ `transaction_sighash` รับ `&TxContext`
   - ใส่ domain separator + context bytes
4. ปรับทุก caller:
   - `crates/node/src/tx_builder.rs`
   - `crates/crypto/src/lib.rs`
   - จุดอื่นที่เซ็น/ตรวจลายเซ็น
5. เขียน unit/integration tests:
   - cross-network replay ต้อง fail
   - genesis ต่างกันต้อง fail
6. รัน `cargo test --all`

---

## ⏳ Task K – Entropy Audit (รอทำ)

1. `crates/pqc-dilithium-seeded/src/randombytes.rs`
   - เปลี่ยนไปใช้ `OsRng`
   - ถ้าต้องการ deterministic สำหรับเทสเพิ่ม helper ภายใต้ `#[cfg(test)]`
2. ทบทวนจุดเรียกที่เกี่ยวข้อง (ensure deterministic build ฟีเจอร์ยังใช้ได้)
3. เพิ่ม unit test เช็ค entropy พื้นฐาน (ต่าง seed ต่างค่า)
4. อัปเดต `docs/ENTROPY_AUDIT.md`
5. รัน `cargo test --all`

---

## 🔁 ขั้นตอนปิดงาน

1. `cargo fmt --all`
2. `cargo clippy --all-targets --all-features -- -D warnings`
3. `cargo test --all --locked`
4. อัปเดต `CHANGELOG.md` และรายการ TODO ให้ตรงสถานะ
5. เตรียม commit message + สรุปผลการเทส

---

> หมายเหตุ: หากพบ dependency ใหม่หรือ error type ที่ยังไม่มีใน crate ให้สร้างในโฟลเดอร์นั้น ๆ โดยยึดสไตล์ existing code. ทำทีละส่วน แล้วคอนเฟิร์มผลก่อนข้ามข้อถัดไป
