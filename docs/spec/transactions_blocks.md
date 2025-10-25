<details open>
<summary>ภาษาไทย (Thai)</summary>

# สเปกข้อมูลธุรกรรมและบล็อก (Phase 3)

เอกสารนี้นิยามโครงสร้างข้อมูลหลักที่ใช้ใน BitQuan สำหรับธุรกรรม บล็อก และส่วนประกอบที่เกี่ยวข้อง เนื้อหาจะเป็นพื้นฐานของการพัฒนาระบบตรวจสอบ ความเข้ากันได้ และกระบวนการ serialization

## 1. ขนบการเข้ารหัสข้อมูล (Data Conventions)
- Endianness: ใช้ Little-endian สำหรับจำนวนเต็มภายใน, Big-endian สำหรับการแปลงแสดงผล (เช่น hash)
- ความยาวข้อมูล: ใช้ CompactSize Uint (similar to Bitcoin) สำหรับ field ที่มีความยาวตัวแปร
- เวอร์ชัน: ฟิลด์ `version` ของ transaction/block ใช้ signed 32-bit; เพิ่มค่าเมื่อมีการเปลี่ยนแปลงฟอร์แมตสำคัญ
- Signature Encoding: ลายเซ็น PQC (Dilithium/Falcon/SPHINCS+) เก็บในรูปแบบไบต์ดิบ ไม่บีบอัด

## 2. โครงสร้าง Transaction
```text
Transaction {
  version: i32
  lock_time: u32
  inputs_count: CompactUint
  inputs: Vec<TxIn>
  outputs_count: CompactUint
  outputs: Vec<TxOut>
  sig_algo: SigAlgorithm
  witnesses_count: CompactUint
  witnesses: Vec<Witness>
}
```

### 2.1 โครงสร้าง TxIn
```text
TxIn {
  prev_txid: [u8; 32]
  prev_vout: u32
  sequence: u32
  script_sig: VarBytes
}
```
- `prev_txid`: แฮชของธุรกรรมก่อนหน้า (SHA-256d), เขียนแสดงผลแบบ big-endian
- `script_sig`: สำหรับเวอร์ชันแรกใช้สคริปต์เรียบง่ายหรือ placeholder จัดเก็บ pointer สำหรับโครงสร้าง PQC ในอนาคต

### 2.2 โครงสร้าง TxOut
```text
TxOut {
  value: u64 // หน่วย satoshi-equivalent (1 BQ = 10^8 unit)
  script_pubkey: VarBytes
}
```
- `script_pubkey`: ใช้ `OP_CHECKSIG_PQC` เป็นพื้นฐาน โดยมีรูปแบบ `OP_DUP OP_HASH160 <pubkey_hash> OP_EQUALVERIFY OP_CHECKSIG_PQC`

### 2.3 นิยาม SigAlgorithm
```text
enum SigAlgorithm : u8 {
  Dilithium3 = 0x01,
  Falcon512 = 0x02,
  SPHINCSPlus = 0x03,
  Reserved(u8),
}
```
- ค่า `Reserved` กันไว้สำหรับอัลกอริทึมใหม่โดยระบุผ่าน BQIP

### 2.4 โครงสร้าง Witness
```text
Witness {
  signatures_count: CompactUint
  signatures: Vec<SignaturePayload>
}
```
- `signatures`: รวมลายเซ็นทั้งหมดที่เกี่ยวข้องกับอินพุตเดียวกันหรือเงื่อนไขเดียวกัน
- รองรับ metadata เพิ่มเติมในอนาคต (เช่น proof, commitment) โดยใช้ `SignaturePayload::aux`

### 2.5 โครงสร้าง SignaturePayload
```text
SignaturePayload {
  signer_index: u16
  signature: VarBytes
  public_key: VarBytes
  aux: Option<AuxiliarySignatureData>
}
```
- `signer_index`: อ้างถึงลำดับอินพุตที่ลายเซ็นนี้ผูกไว้
- `aux`: ใช้เก็บเมตาดาต้า เช่น commitment, randomness proof, หรือข้อมูลสำหรับ hybrid scheme

## 3. โครงสร้าง BlockHeader
```text
BlockHeader {
  version: i32
  prev_block: [u8; 32]
  merkle_root: [u8; 32]
  pqc_agg_hint: [u8; 32]
  time: u32
  bits: u32
  nonce: u64
}
```
- `pqc_agg_hint`: ค่า commitment สำหรับ aggregate signature หรือ reserved extension ในอนาคต (32 ไบต์)
- `bits`: บีบรัดเป้าหมายความยากเหมือน Bitcoin (compact difficulty target)
- `nonce`: ขยายเป็น 64-bit เพื่อรองรับพื้นที่ค้นหาเพิ่มเติมในยุค ASIC/FPGA

## 4. โครงสร้าง Block
```text
Block {
  header: BlockHeader
  tx_count: CompactUint
  transactions: Vec<Transaction>
}
```
- การ validate ต้องรวมตรวจสอบกฎ block weight: `weight = raw_bytes + α * (#pqc_signatures)` โดยเริ่มที่ `α = 384`

## 5. Serialization (Wire Format)
- ใช้ลำดับฟิลด์ตามโครงสร้างที่ระบุ โดยไม่มี padding
- ข้อมูลที่มีความยาวตัวแปร (เช่น script, signatures) นำหน้าด้วย CompactUint เลือกความยาว
- ลำดับธุรกรรมภายในบล็อกคงที่ตามที่รับจาก miner หรือตามกฎ mempool policy ในอนาคต

## 6. ข้อพิจารณาเพิ่มเติม
- ข้อมูล witness อยู่ใน `Transaction::witnesses` แยกจาก payload หลักของธุรกรรม
- เตรียมช่อง `pqc_agg_hint` เพื่อให้สามารถใช้ aggregate signature หรือ commitment สำหรับ batch verification
- อนุญาตให้ใช้ multisig ผ่านสคริปต์เชิงพันธุ์ (script extensions) ใน Phase ถัดไป โดยกำหนด opcode เพิ่มเติมผ่าน BQIP

## 7. งานที่ต้องทำต่อ
1. เขียน Test vectors สำหรับ transaction serialization/deserialization
2. นิยามกฎ block weight enforcement ในโมดูล consensus พร้อม unit test
3. ออกแบบ Witness format และเงื่อนไขสำหรับ multisig เพื่อเตรียม Phase 4–5

</details>

<details>
<summary>English</summary>

# Transaction and Block Data Specification (Phase 3)

This document defines the core data structures used by BitQuan for transactions, blocks, and related components. These definitions underpin validation logic, interoperability guarantees, and the wire serialization format.

## 1. Data Conventions
- **Endianness**: Little-endian for internal integers; Big-endian for display (e.g., hashes)
- **Length fields**: CompactSize unsigned integers (Bitcoin-style) for variable-length values
- **Versioning**: `version` fields are signed 32-bit integers and increment on major format changes
- **Signature encoding**: PQC signatures (Dilithium/Falcon/SPHINCS+) stored as raw uncompressed bytes

## 2. Transaction Structure
```text
Transaction {
  version: i32
  lock_time: u32
  inputs_count: CompactUint
  inputs: Vec<TxIn>
  outputs_count: CompactUint
  outputs: Vec<TxOut>
  sig_algo: SigAlgorithm
  witnesses_count: CompactUint
  witnesses: Vec<Witness>
}
```

### 2.1 TxIn Structure
```text
TxIn {
  prev_txid: [u8; 32]
  prev_vout: u32
  sequence: u32
  script_sig: VarBytes
}
```
- `prev_txid`: Hash of the previous transaction (SHA-256d), displayed Big-endian
- `script_sig`: Placeholder for PQC script data or custom unlocking scripts in future phases

### 2.2 TxOut Structure
```text
TxOut {
  value: u64 // satoshi-equivalent (1 BQ = 10^8 units)
  script_pubkey: VarBytes
}
```
- `script_pubkey`: Defaults to `OP_DUP OP_HASH160 <pubkey_hash> OP_EQUALVERIFY OP_CHECKSIG_PQC`

### 2.3 SigAlgorithm Enumeration
```text
enum SigAlgorithm : u8 {
  Dilithium3 = 0x01,
  Falcon512 = 0x02,
  SPHINCSPlus = 0x03,
  Reserved(u8),
}
```
- Additional algorithms require a dedicated BQIP assignment

### 2.4 Witness Structure
```text
Witness {
  signatures_count: CompactUint
  signatures: Vec<SignaturePayload>
}
```
- `signatures`: Grouped PQ signatures tied to a particular input or script path.
- Extensible via `SignaturePayload::aux` for future commitments or aggregation metadata.

### 2.5 SignaturePayload Structure
```text
SignaturePayload {
  signer_index: u16
  signature: VarBytes
  public_key: VarBytes
  aux: Option<AuxiliarySignatureData>
}
```
- `signer_index`: References the input index associated with this signature
- `aux`: Metadata for commitments, randomness proofs, or hybrid schemes

## 3. BlockHeader Structure
```text
BlockHeader {
  version: i32
  prev_block: [u8; 32]
  merkle_root: [u8; 32]
  pqc_agg_hint: [u8; 32]
  time: u32
  bits: u32
  nonce: u64
}
```
- `pqc_agg_hint`: Reserved 32-byte commitment for future aggregate signature extensions
- `bits`: Compact representation of the difficulty target (Bitcoin-compatible)
- `nonce`: 64-bit to accommodate extended search space for ASIC/FPGA miners

## 4. Block Structure
```text
Block {
  header: BlockHeader
  tx_count: CompactUint
  transactions: Vec<Transaction>
}
```
- Validation includes enforcing the block weight rule: `weight = raw_bytes + α * (#pqc_signatures)` with `α = 384` initially

## 5. Serialization Rules
- Fields are serialized in the order listed without padding
- Variable-length data (scripts, signatures) are prefixed with CompactUint length indicators
- Transaction ordering inside a block remains as produced by the miner or future mempool policy

## 6. Additional Considerations
- Witness data resides in `Transaction::witnesses`, enabling segregation from the base transaction body
- `pqc_agg_hint` prepares for aggregate signatures or commitments used during batch verification
- Multisig support will evolve via script extensions defined in subsequent BQIPs

## 7. Follow-up Work
1. Produce serialization/deserialization test vectors for transactions
2. Define block weight enforcement rules in the consensus module with unit tests
3. Design witness formats and multisig constraints for Phase 4–5 planning

</details>
