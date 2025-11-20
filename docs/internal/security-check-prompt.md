# BitQuan Project Security & Risk Assessment Prompt

## คำสั่งให้ Claude ตรวจสอบโปรเจค

### วัตถุประสงค์
- ตรวจสอบความปลอดภัยของโปรเจค BitQuan blockchain
- หาช่องโหว่และความเสี่ยงที่อาจเกิดขึ้น
- ประเมินคุณภาพโค้ดและการออกแบบระบบ

## คำสั่ง Prompt (คัดลอกส่วนนี้ไปให้ Claude)

```
ทำการตรวจสอบโปรเจค BitQuan blockchain โดยใช้วิธีการต่อไปนี้เพื่อประหยัดโทเคน:

### 1. การวิเคราะห์ระดับสูง (High-Level Analysis)
- อ่านไฟล์ README.md, SECURITY.md, SECURITY_AUDIT_REPORT.md ก่อน
- ดูโครงสร้างโปรเจคจาก Cargo.toml และ directory listing
- ตรวจสอบ architecture จาก docs/architecture/

### 2. การค้นหาความเสี่ยงแบบมีทิศทาง (Targeted Risk Search)
ใช้ grep/search หา:
- "unsafe", "panic!", "unwrap()", "expect()" ในไฟล์ .rs
- "TODO", "FIXME", "HACK", "XXX" ทั่วโปรเจค
- การเชื่อมต่อ network และ RPC endpoints
- การจัดการ private keys และ secrets
- crypto operations และ randomness

### 3. การตรวจสอบส่วนสำคัญ (Critical Components)
โฟกัสที่:
- crates/crypto/ - การเข้ารหัสลับ
- crates/consensus/ - กลไก consensus
- crates/wallet/ - การจัดการ wallet
- crates/network/ - การเชื่อมต่อเครือข่าย
- crates/rpc/ - API endpoints

### 4. การประเมินความเสี่ยง (Risk Assessment)
จัดลำดับความเสี่ยง:
- 🔴 ความเสี่ยงสูง (Critical) - ช่องโหว่ที่สามารถถูก exploit ได้
- 🟡 ความเสี่ยงกลาง (Medium) - ปัญหาด้าน performance หรือ logic errors
- 🟢 ความเสี่ยงต่ำ (Low) - code quality issues

### 5. การตรวจสอบการออกแบบ (Design Review)
- ตรวจสอบ post-quantum cryptography implementation
- ดูการออกแบบ consensus mechanism
- ตรวจสอบการจัดการ memory และ resources

## สิ่งที่ต้องการให้รายงาน:

### ความเสี่ยงด้านความปลอดภัย:
1. ช่องโหว่ที่พบ (file:line)
2. ปัญหาการใช้ unsafe code
3. การจัดการ secrets ที่ไม่ปลอดภัย
4. ช่องโหว่ด้าน network/cryptography

### ความเสี่ยงด้านสถาปัตยกรรม:
1. ปัญหาการออกแบบ consensus
2. จุดอ่อนในการจัดการ wallet
3. ปัญหา scalability หรือ performance

### คำแนะนำการแก้ไข:
1. การแก้ไขแบบ immediate (ฉุกเฉิน)
2. การปรับปรุงระยะยาว
3. การเพิ่ม security measures

### สรุปความเสี่ยง:
- จำนวนช่องโหว่ตามระดับความรุนแรง
- ความพร้อมสำหรับ production
- ประเด็นที่ต้องตรวจสอบเพิ่มเติม

## ข้อควรระวัง:
- อย่าอ่านไฟล์ขนาดใหญ่ทั้งไฟล์ ใช้ grep แทน
- โฟกัสที่ไฟล์สำคัญก่อน
- ใช้ search patterns ที่มีประสิทธิภาพ
- สรุปเฉพาะประเด็นสำคัญ
```

## วิธีการใช้งาน:

1. **คัดลอก prompt ข้างบน** แล้วส่งให้ Claude
2. **Claude จะทำการวิเคราะห์แบบ step-by-step** ตามที่กำหนด
3. **จะได้รายงานความเสี่ยง** พร้อมคำแนะนำการแก้ไข

## ประโยชน์ของวิธีนี้:
- ประหยัดโทเคน (ไม่อ่านไฟล์ใหญ่ๆ โดยตรง)
- มีทิศทางชัดเจน (targeted search)
- ครอบคลุมด้านความปลอดภัยโดยเฉพาะ
- จัดลำดับความสำคัญของความเสี่ยง

## สิ่งที่ควรตรวจสอบเพิ่มเติม:
- Post-quantum cryptography implementation
- Memory safety ใน Rust
- Network protocol security
- Consensus algorithm robustness
- Private key management