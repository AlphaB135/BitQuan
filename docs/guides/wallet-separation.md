# BitQuan แนะนำการแยกกระเป๋า (เหมือน Bitcoin)

## 📁 โครงสร้างกระเป๋าที่แนะนำ

```
bitquan-wallets/
├── mainnet-wallet.keystore   # เงินจริง (ระวัง!)
├── testnet-wallet.keystore   # ทดสอบ
└── devnet-wallet.keystore    # พัฒนา
```

## 🛡️ เหตุผลที่ควรแยกกระเป๋า

### 1. **ปลอดภัย - รั่วไหลทีละ network**
- หาก mainnet รั่ว → เสียเงินแค่ mainnet
- testnet/devnet รั่ว → ไม่มีค่ามูล
- ลดความเสียหายสูงสุด

### 2. **ชัดเจน - ไม่สับสน**
- เห็นได้ทันทีว่าอยู่บน network ไหน
- ไม่ส่งเงินจริงไป testnet โดยผิดพลาด
- ไม่ส่ง test token ไป mainnet

### 3. **ป้องกันผิดพลาด - ไม่ส่งผิด network**
- บังคับให้ตรวจสอบก่อนส่ง
- ลดความเสี่ยงจากความผิดพลาดของมนุษย์
- ป้องกันการสูญเสียเงินจากความลืม

### 4. **มาตรฐาน - ตาม Bitcoin**
- Bitcoin แนะนำการแยกกระเป๋า
- ทุก exchange ใช้กระเป๋าแยกกัน
- เป็น best practice ของ crypto

## 🚀 วิธีสร้างกระเป๋าแยก

### Mainnet Wallet (เงินจริง)
```bash
./target/release/bitquan-node wallet-gen \
  --algo dilithium3 \
  --output mainnet-wallet.keystore \
  --password "strong_password_here"
```

### Testnet Wallet (ทดสอบ)
```bash
./target/release/bitquan-node wallet-gen \
  --algo dilithium3 \
  --output testnet-wallet.keystore \
  --password "test_password"
```

### Devnet Wallet (พัฒนา)
```bash
./target/release/bitquan-node wallet-gen \
  --algo dilithium3 \
  --output devnet-wallet.keystore \
  --password "dev_password"
```

## 📋 การตั้งชื่อที่ชัดเจน

### ✅ ชื่อที่ดี
- `mainnet-trading.keystore`
- `testnet-faucet.keystore`
- `devnet-experiments.keystore`

### ❌ ชื่อที่ไม่ดี
- `wallet.keystore` (ไม่รู้ network)
- `bq-wallet` (กำกวม)
- `my-wallet` (ไม่ปลอดภัย)

## 🔒 กฎความปลอดภัย

### 1. **Password Policy**
- Mainnet: รหัสผ่านยาวๆ ผสมตัวอักษรพิเศษ
- Testnet: รหัสผ่านธรรมดา
- Devnet: รหัสผ่านง่าย (หรือไม่มีก็ได้)

### 2. **Backup Strategy**
```bash
# Mainnet - backup หลายที่
cp mainnet-wallet.keystore ~/backup/
cp mainnet-wallet.keystore usb-drive/
cp mainnet-wallet.keystore cloud-storage/

# Testnet/Devnet - backup ที่เดียวพอ
cp testnet-wallet.keystore ~/backup/
```

### 3. **Environment Variables**
```bash
export BITQUAN_MAINNET_WALLET="$HOME/mainnet-wallet.keystore"
export BITQUAN_TESTNET_WALLET="$HOME/testnet-wallet.keystore"
export BITQUAN_DEVNET_WALLET="$HOME/devnet-wallet.keystore"
```

## 🎯 การใช้งานจริง

### การทำธุรกรรม Mainnet
```bash
./target/release/bitquan-node wallet-send \
  --keystore mainnet-wallet.keystore \
  --to bq1... \
  --amount 100000000 \
  --password "strong_password"
```

### การทดสอบบน Testnet
```bash
./target/release/bitquan-node wallet-send \
  --keystore testnet-wallet.keystore \
  --to tbq1... \
  --amount 100000000 \
  --password "test_password"
```

## ⚠️ คำเตือนสำคัญ

1. **ไม่เคยใช้ mainnet wallet บน testnet**
2. **ตรวจสอบ network ก่อนทุกการทำธุรกรรม**
3. **เก็บรหัสผ่าน mainnet แยกจากที่อื่น**
4. **backup mainnet wallet บน media หลายชนิด**

## 📊 สรุป

| Network | ความเสี่ยง | ความปลอดภัย | การ backup |
|---------|------------|--------------|-----------|
| Mainnet | สูง | สูงสุด | หลายที่ |
| Testnet | ต่ำ | ปานกลาง | ที่เดียว |
| Devnet | ต่ำสุด | ต่ำ | ไม่จำเป็น |

**การแยกกระเป๋าคือการป้องกันที่ดีที่สุดสำหรับเงินของคุณ!** 🛡️