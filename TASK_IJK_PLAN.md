# แผนงานรีสตาร์ท Tasks I/J/K (ความปลอดภัยเชิงลึก)

อัพเดต: 2025-11-02

## แนวทางการลงมือ (ทำทีละข้อ)
1. ปิด Task I (Checked Arithmetic) > รันเทส
2. ปิด Task J (Replay Protection) > รันเทส
3. ปิด Task K (Entropy Audit) > รันเทส
4. เก็บงานส่งท้าย + เอกสาร/CHANGELOG

---

## ✅ Task I – Checked Arithmetic Audit

ทำตามลำดับนี้
1. `crates/mempool/src/lib.rs`
   - เพิ่ม error type ที่จำเป็น
   - เปลี่ยน `fee / weight`, `.sum()`, `+=` ให้ใช้ `checked_*` / `try_fold`
2. `crates/consensus/src/lib.rs`
   - คำนวณ block weight ด้วย `try_fold` + `checked_mul`
3. `crates/consensus/src/utxo.rs`
   - ใช้ `checked_add/sub` กับตัวนับ UTXO และค่าธรรมเนียม
4. `crates/types/src/transaction.rs`
   - แก้ heuristic size ให้ใช้ `checked_add` + `try_fold`
5. `crates/types/src/block.rs`
   - เหมือนข้อ 4 สำหรับ block
6. `crates/node/src/main.rs`
   - นับ balance/UTXO ด้วย `checked_add`
7. เพิ่ม unit tests/prop tests สำหรับ overflow conditions
8. รัน `cargo test --all` (เฉพาะส่วน Task I เพื่อยืนยันก่อนข้ามขั้น)

---

## ✅ Task J – Replay Attack Prevention

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

## ✅ Task K – Entropy Audit

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
