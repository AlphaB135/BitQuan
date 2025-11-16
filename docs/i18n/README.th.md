<p align="right">
  <a href="./README.md"><img alt="English" src="https://img.shields.io/badge/English-blue?style=for-the-badge"></a>
</p>

# BitQuan

[![CI](https://github.com/alphab/BitQuan/actions/workflows/ci.yml/badge.svg)](https://github.com/alphab/BitQuan/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

BitQuan: บล็อกเชนเน้น PQC (PoW + Dilithium) มุ่งความทนทานความปลอดภัย 50+ ปี

เอกสารหน้านี้เป็นภาษาไทย; คลิกปุ่ม English ด้านบนเพื่อไปยัง README ภาษาอังกฤษ (`README.md`).

## Quickstart

```bash
cargo test -p bq-crypto
```

## เอกสาร
- [ภาพรวมสถาปัตย์](../architecture/overview.md)
- [การกำกับดูแลโครงการ](../governance/GOVERNANCE.md)
- [นโยบายความปลอดภัย](SECURITY.md)
- [กระบวนการออกเวอร์ชัน](../guides/RELEASE.md)

---

# ภาพรวมโครงการ BitQuan

BitQuan คือเครือข่ายบล็อกเชนที่มุ่งสู่ความปลอดภัยระยะยาว 50+ ปี พร้อมการผสาน Post-Quantum Cryptography (PQC) เต็มรูปแบบ เอกสารนี้สรุปภาพรวมโครงการ สถานะปัจจุบัน และแนวทางเริ่มต้นสำหรับผู้ร่วมพัฒนา

## ไฮไลต์
- นโยบาย Phase 0: ไม่มี backdoor, admin key หรือสวิตช์ลับใด ๆ
- สถาปัตยกรรมเบื้องต้น: Proof-of-Work Minimalist + ลายเซ็น Dilithium (ดู `docs/architecture/overview.md`)
- ชุดเอกสารมาตรฐาน (Governance, Contributing, Release, Security, Reproducibility) อยู่ภายใต้ `docs/`

## โครงสร้างโฟลเดอร์หลัก
- `docs/` – เอกสารอ้างอิงทั้งหมด (ธรรมาภิบาล ความปลอดภัย การ build ซ้ำ ฯลฯ)
- `docs/architecture/overview.md` – ภาพรวมสถาปัตยกรรมสองภาษา พร้อมเลือกภาษาได้ในไฟล์เดียว
- `docs/security/` – คีย์ GPG, ตาราง on-call, และโพสต์มอร์เท็มเหตุการณ์
- `todo.md` – Master plan รายเฟส (Phase 0–13)

## งานต่อเนื่อง
1. ร่างสเปก Transaction / Block (Phase 3)
2. จัดทำ BQIP 0001–0004 ให้สอดคล้องกับสถาปัตยกรรม
3. ตั้งต้นโค้ดฐาน (Rust) สำหรับโมดูลหลัก: crypto, consensus, mempool, p2p, storage

## วิธีร่วมพัฒนา
- อ่าน `docs/CONTRIBUTING.md` เพื่อทำความเข้าใจขั้นตอนรีวิวและมาตรฐานโค้ด
- ตั้งค่าการ build ตาม `docs/REPRODUCIBILITY.md`
- ส่ง Pull Request พร้อมลายเซ็น commit (`git commit -S`)

## ช่องทางติดต่อความปลอดภัย
- อีเมล: `security@bitquan.org`
- รายละเอียดเพิ่มเติมดูใน `docs/SECURITY.md`
