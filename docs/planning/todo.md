🔮 BitQuan Security & Delivery Tracker
อัปเดตล่าสุด: 2025-11-02 (หลัง v0.0.2-alpha hardening)

---
## ✅ สถานะรวม
- [x] Task H – Cryptographic Timing Audit (ตรวจแล้ว Low risk)
- [x] Task I – Checked Arithmetic Audit (overflow/underflow ปลอดภัย)
- [x] Task J – Replay Attack Prevention (TxContext + domain separator)
- [x] Task K – Entropy Audit (OsRng, docs/ENTROPY_AUDIT.md)
- [ ] External Security Audit (ต้องทำก่อน mainnet)

---
## 🔥 Critical Follow-ups (หลังรีวิวล่าสุด)
- [ ] Audit และแทนที่ `.unwrap()` ใน production paths (เริ่มจาก multisig, mnemonic, fork, mempool, consensus, network, storage, tx_builder)
- [ ] เพิ่ม Retry-After header + body read timeout ใน RPC rate limiting/req handling
- [ ] Cleanup ไฟล์สำรอง/ซ้ำ (`*.bak`, `*-e`, `.tmp`, `.DS_Store`, ENTROPY_AUDIT duplicates) และอัปเดต `.gitignore`
- [ ] กำหนด activation heights สำหรับ feature flags ใน consensus params + tests
- [ ] กำหนด mempool DoS caps (inputs/outputs/script size/sigops + ancestor limits) พร้อม validation tests
- [ ] ตรวจและเติม P2P safety knobs (handshake timeout, max peers, ban score, max msg/block size, announce rate limit)
- [ ] เพิ่ม fuzz + miri jobs ใน CI เพื่อตรวจ undefined behavior / parser crashes

---
## 📦 รายละเอียดงานที่เสร็จแล้ว
### H) Timing & Side-channel Audit
- Argon2/JWT, Dilithium, Script verify ใช้ constant-time ที่ library จัดให้
- บันทึกไว้สำหรับ future work: พิจารณา `subtle` crate หากต้องการควบคุมเองเพิ่มเติม

### I) Checked Arithmetic
- เพิ่ม/ใช้ `checked_add`, `checked_sub`, `try_fold` ใน mempool, consensus, types, node, wallet
- เพิ่ม error variants (`InvalidWeight`, `Overflow`, `WeightOverflow`, ฯลฯ)
- เพิ่ม test overflow/underflow 20+ เคส (u64::MAX, fee, balance, UTXO, coin selection)

### J) Replay Protection
- เพิ่ม `TxContext { network_id, genesis_hash }`
- `transaction_sighash(...) → Result<[u8;32], SighashError>` พร้อม domain separator `BitQuanSigHashV1`
- TransactionBuilder, CryptoRegistry, RPC wallet ปรับให้รับ context ใหม่
- เพิ่ม replay tests (cross-network, genesis mismatch, builder flow)

### K) Entropy Audit
- เปลี่ยน `thread_rng()` → `OsRng` ทุก production path
- deterministic helper อยู่ภายใต้ `#[cfg(test)]` เท่านั้น
- เพิ่ม tests เรื่อง entropy และ `docs/ENTROPY_AUDIT.md`
- README บันทึกสถานะ Security ล่าสุด

---
## 🛠️ Wallet Encryption Sprint (Week 2–5)
สถานะ: เตรียมเริ่ม (ยังไม่ได้ทำ)
- [ ] เพิ่ม dependencies (argon2, aes-gcm, zeroize) สำหรับ library ที่ยังไม่ครบ
- [ ] สร้าง `SecureString`
- [ ] สร้าง `SecurePrivateKey`
- [ ] Implement `KeyDerivation` (Argon2id)
- [ ] เขียน unit tests สำหรับ KDF
- [ ] Implement Encryptor (AES-256-GCM)
- [ ] เขียน tests สำหรับ encryption
- [ ] Update keystore struct
- [ ] Implement save/load with encryption
- [ ] Integration tests (keystore roundtrip)
- [ ] Update CLI commands
- [ ] Add password prompts (no-echo + backoff)
- [ ] File permissions (Unix) + Windows note
- [ ] Error handling / UX copy
- [ ] User documentation
- [ ] Security review
- [ ] Performance benchmarks
- [ ] Code coverage > 80%
- [ ] Update README / docs
- [ ] Merge to main (หลังผ่าน review)

---
## 🎯 Backlog / Optional Tasks
### Cryptography &安全
- [ ] เพิ่ม constant-time comparison ชัดเจน (พิจารณา `subtle` crate)
- [ ] H) Constant-time comparison audit (ละเอียดระดับ byte)

### Reliability / Hardening
- [ ] เพิ่ม fuzz targets สำหรับ parsers
- [ ] แทนที่ production `unwrap()` ที่เหลือ (ยืนยัน no panic)
- [ ] เพิ่ม panic metrics (Prometheus) + CI guard: fail on new unwrap()
- [ ] เอกสาร eclipse config สำหรับ operators
- [ ] Enable host/origin validation ใน `handle_connection` (ใช้ default ที่ enforce แล้ว? double-check)
- [ ] Add verification logic ใน node startup sequence (validate config)
- [ ] Add metrics/monitoring endpoints (เช่น Prometheus / health metrics เพิ่มเติม)

### Networking / RPC
- [ ] ตรวจว่าฟังก์ชัน RPC เก่า (BasicAuth helpers ฯลฯ) ยังจำเป็นหรือถอดออกได้
- [ ] ปรับปรุง `p2p_server` ให้แตกเป็นโมดูลย่อย / struct (ลด arg เพิ่มเติม)

---
## 📌 ภารกิจระดับโครงการต่อไป
- [ ] จัด External Security Audit (Trail of Bits, Cure53, ฯลฯ)
- [ ] เปิด public testnet อย่างน้อย 3–6 เดือน
- [ ] ตั้ง Bug Bounty Program ($5K–$50K)
- [ ] เขียนบล็อก “How I hardened a blockchain solo with AI” + โปรโมตใน community

---
## 🗒️ บันทึกอ้างอิง
- Release: [v0.0.2-alpha](../releases/RELEASE_NOTES_v0.0.2-alpha.md)
- Entropy Audit: [docs/ENTROPY_AUDIT.md](../security/ENTROPY_AUDIT.md)
- Security Status: README.md (ส่วน Security Status)
