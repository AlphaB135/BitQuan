# 🔴 Critical Security Tasks (H–K Priority P0)

อัปเดตล่าสุด: 2025-11-03

---

## สถานะโดยรวม
- [x] Basic Auth Removal – **COMPLETE** ✅
- [x] H) Cryptographic Timing Attacks – **COMPLETE** ✅ (2025-11-02)
- [x] I) Integer Overflow Protection – **COMPLETE** ✅ (2025-11-03)
- [x] J) Replay Attack Prevention – **COMPLETE** ✅ (2025-11-03)
- [x] K) Entropy Quality – **VERIFIED OK** ✅

---

## H) Cryptographic Timing Attacks ✅ COMPLETE
- ใช้ `subtle::ConstantTimeEq` ในทุกจุดที่เปรียบเทียบข้อมูลสำคัญ (`crates/wallet/src/backup.rs` ฯลฯ)
- เพิ่มการทดสอบ tamper detection ให้ผ่านครบ 33 รายการ
- ไม่มีการเปลี่ยนแปลงเพิ่มเติม จำเป็นเพียงดูแลไม่ให้ regression ในอนาคต

---

## I) Integer Overflow Protection ✅ COMPLETE

### สิ่งที่ทำแล้ว
- สลับโค้ด critical arithmetic ในโมดูลสำคัญ (`crates/mempool`, `crates/consensus`, `crates/types`, `crates/node`) มาใช้ `checked_*` + `try_fold`.
- เปิดใช้ shared error stack (`bitquan_types::error`, `ResultExt`, `checked!`) เพื่อบับเบิล `Error::Overflow` / `Error::Invalid`.
- เพิ่ม unit/property tests ครอบคลุมการรวม weight, fee underflow, buffer growth และ balance accumulation.
- ยืนยัน `cargo fmt`, `cargo clippy -- -D warnings`, `cargo test --all` ผ่าน ณ วันที่ 2025-11-03.

### สิ่งที่ต้องระวังต่อ
- เมื่อเพิ่ม accumulator ใหม่ให้ยึด `checked_*` เป็น default และเพิ่ม regression test เสมอ.
- ติดตาม performance ใน mempool/consensus หากมีการ optimize เพิ่มเติม.

---

## J) Replay Attack Prevention ✅ COMPLETE

### สิ่งที่ทำแล้ว
- เพิ่ม `TxContext` และ `NetworkId::magic_bytes` ใน `crates/types/src/context.rs` และเผยแพร่ผ่าน `lib.rs`.
- บังคับทุกเส้นทางเซ็น/ตรวจ (`crates/consensus/src/sighash.rs`, wallet signer, node builder, RPC) ให้รับ `TxContext` พร้อม domain separator.
- ปรับ `crates/node/src/tx_builder.rs` และ mnemonic path ให้ใช้ shared error แทน panic และยิงทดสอบ cross-network/genesis mismatch.
- ทดสอบ `cargo test` สำหรับ consensus, node, wallet ผ่านทั้งหมด.

### สิ่งที่ต้องระวังต่อ
- ตรวจสอบ integration ภายนอก (SDK / bindings) ว่าพร้อมส่ง `TxContext`.
- เพิ่มเอกสาร client-facing ใน `docs/rpc/` อธิบาย requirement ของ context.

---

## K) Entropy Quality ✅ VERIFIED
- `OsRng` ใช้ในทุกจุดสำคัญ (Dilithium keypair, salt/nonce, wallet keystore, RPC JWT).
- เพิ่ม unit tests สำหรับ `randombytes` (random vs deterministic helper) และ entropy smoke tests.
- อัปเดต `docs/ENTROPY_AUDIT.md` สรุปผลการตรวจสอบ.
- TODO ต่อไป: เพิ่ม test เชิงสถิติ/ collision ใน sprint ถัดไปถ้า performance เอื้อ.

---

## ภาพรวมไทม์ไลน์ (อ้างอิงเดิม)
| Task | Days | Priority |
|------|------|----------|
| H) Timing Attacks | 4 | P0 🔴 |
| I) Integer Overflow | 6 | P0 🔴 |
| J) Replay Prevention | 6 | P0 🔴 |
| K) Entropy Quality | 2 | P1 🟡 |

> หมายเหตุ: งานหลักปิดครบแล้ว ใช้ตารางเพื่อติดตาม regression/maintenance

---

## 🔜 โฟกัสงานถัดไป (ภายในทีมความปลอดภัย)
- [ ] **RPC DoS Regression Tests** – เพิ่มเคส slow-body timeout (คาดหวัง 408), header flood (413/431), ตรวจ `Retry-After` บน 429 และเชื่อม metrics กับ alert.
- [ ] **Shared Error Adoption** – ไล่ปรับ call-site ที่ยังใช้ `anyhow::Result` (เช่น coin selection, network peer manager) ให้อยู่บน `bitquan_types::error::Result`.
- [ ] **Policy & Docs** – สรุป arithmetic guard/mempool cap ใหม่ใน `docs/spec/` และสร้าง `docs/mempool-policy.md`.
- [ ] **Nightly CI Hardening** – เพิ่ม workflow สำหรับ fuzz (20s smoke), miri, docs lint/link check.
- [ ] **External SDK Alignment** – ยืนยัน binding/CLI ใช้งาน `TxContext` และ RNG policy เดียวกัน.

---

> หากพบงานใหม่จาก security review ให้เติมใน "โฟกัสงานถัดไป" พร้อมวัน/ผู้รับผิดชอบ แล้วแตกไฟล์แผนย่อยตามความเหมาะสม
