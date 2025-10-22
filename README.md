<details open>
<summary>ภาษาไทย (Thai)</summary>

# BitQuan Project Overview

BitQuan คือเครือข่ายบล็อกเชนที่มุ่งเน้นความปลอดภัยระยะยาว (50+ ปี) และการบูรณาการ Post-Quantum Cryptography เต็มรูปแบบ เอกสารนี้สรุปภาพรวม โปรเจกต์สเตตัส และจุดเริ่มต้นสำหรับผู้ร่วมพัฒนา

## ไฮไลต์
- ไม่มี backdoor / admin key ตามนโยบายเฟส 0
- สถาปัตยกรรม PoW Minimalist + Dilithium พร้อม roadmap พัฒนาต่อใน `docs/architecture/overview.md`
- ชุดเอกสารมาตรฐาน (Governance, Contributing, Release, Security, Reproducibility) อยู่ภายใต้ `docs/`

## โครงสร้างโฟลเดอร์หลัก
- `docs/` – เอกสารอ้างอิงทั้งหมด (ธรรมาภิบาล ความปลอดภัย สถาปัตยกรรม ฯลฯ)
- `docs/architecture/overview.md` – ภาพรวมสถาปัตยกรรมสองภาษา
- `docs/security/` – คีย์ GPG, ตาราง on-call, และโพสต์มอร์เท็ม
- `todo.md` – Master plan รายเฟส

## ขั้นถัดไปที่เปิดอยู่
1. ร่างสเปกธุรกรรม/บล็อก (Phase 3)
2. เตรียม BQIP 0001–0004 ให้ตรงกับสถาปัตยกรรม
3. สร้าง baseline โค้ด (Rust) สำหรับโมดูลหลัก

## วิธีร่วมพัฒนา
- อ่าน `docs/CONTRIBUTING.md` เพื่อทำความเข้าใจกระบวนการรีวิวและมาตรฐานโค้ด
- ตั้งค่าการ build ตาม `docs/REPRODUCIBILITY.md`
- ส่ง Pull Request พร้อมลายเซ็น commit (`git commit -S`)

---

</details>

<details>
<summary>English</summary>

# BitQuan Project Overview

BitQuan targets a 50+ year security horizon with full Post-Quantum Cryptography integration. This README summarizes the current status and onboarding path for contributors.

## Highlights
- Phase 0 policy: absolutely no backdoors or hidden admin switches
- Minimalist PoW consensus with Dilithium signatures; see `docs/architecture/overview.md`
- Standard documentation set (Governance, Contributing, Release, Security, Reproducibility) under `docs/`

## Key Directories
- `docs/` – Canonical documentation (governance, security, reproducibility, etc.)
- `docs/architecture/overview.md` – Bilingual architecture overview
- `docs/security/` – GPG keys, on-call roster, post-mortems
- `todo.md` – Phase-by-phase master plan

## Open Next Steps
1. Draft transaction/block data specs (Phase 3)
2. Author BQIP drafts 0001–0004 aligned with the architecture
3. Bootstrap the Rust baseline for core modules

## Contributing
- Review `docs/CONTRIBUTING.md` for workflow and coding standards
- Configure builds per `docs/REPRODUCIBILITY.md`
- Submit signed commits (`git commit -S`) in pull requests

</details>
