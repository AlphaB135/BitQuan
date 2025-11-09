# BitQuan Checkpoint System - คู่มือการใช้งาน

## 📋 ภาพรวม

BitQuan Checkpoint System เป็นระบบความปลอดภัยระดับสูงที่ออกแบบมาเพื่อ:

- 🛡️ **ป้องกันการโจมตี**: ปกป้อง blockchain จาก consensus attacks
- 🔄 **การกู้คืน**: กู้คืน blockchain จาก mining bugs หรือ vulnerabilities
- 🚨 **ตอบสนองฉุกเฉิน**: จัดการเหตุการณ์ฉุกเฉินได้อย่างรวดเร็ว
- 📊 **การตรวจสอบ**: ตรวจสอบความถูกต้องของ blocks อัตโนมัติ

---

## 🚀 เริ่มต้นใช้งาน

### การติดตั้ง

```bash
# Clone repository
git clone https://github.com/AlphaB135/BitQuan.git
cd BitQuan

# Build project
cargo build --release

# Run tests
cargo test -p bitquan-consensus checkpoint
```

### การตั้งค่าพื้นฐาน

```rust
use bitquan_consensus::{CheckpointManager, EmergencyManager, EmergencyConfig};

// สร้าง checkpoint manager
let mut checkpoint_manager = CheckpointManager::new(false);

// ตั้งค่า emergency manager
let config = EmergencyConfig {
    enabled: true,
    required_signatures: 3,
    response_window: 3600,
    authorized_operators: vec!["operator1".to_string()],
};

let mut emergency_manager = EmergencyManager::new(config);
```

---

## 🛠️ วิธีการใช้งาน

### 1. การสร้าง Emergency Checkpoint

```rust
// สร้าง checkpoint ที่ block ที่ปลอดภัย
let safe_height = 750000;
let safe_hash = [0x12; 32]; // 32-byte hash

emergency_manager.create_emergency_checkpoint(
    safe_height,
    safe_hash,
    "Mining bug rollback".to_string(),
    "operator1"
)?;
```

### 2. การเปิดใช้งาน Checkpoint Validation

```rust
use bitquan_consensus::EmergencyAction;

// เปิด checkpoint validation
let action = EmergencyAction::EnableCheckpoints;
emergency_manager.execute_action(action, "operator1")?;
```

### 3. การหยุดการประมวลผลฉุกเฉิน

```rust
// หยุดการประมวลผลทันที
let action = EmergencyAction::PauseProcessing;
emergency_manager.execute_action(action, "operator1")?;
```

### 4. การแบน Malicious Peers

```rust
// แบน peers ที่เป็นอันตราย
let action = EmergencyAction::BanPeers {
    peer_ids: vec!["attacker1".to_string(), "attacker2".to_string()],
};
emergency_manager.execute_action(action, "operator1")?;
```

---

## 🖥️ การใช้งานผ่าน CLI

### การติดตั้ง CLI

```bash
# Build CLI tool
cargo build --release --bin checkpoint_cli

# หรือรันตรงๆ
cargo run --bin checkpoint_cli -- --help
```

### คำสั่งพื้นฐาน

```bash
# ดูสถานะระบบ
cargo run --bin checkpoint_cli -- status

# ดูสถานะแบบละเอียด
cargo run --bin checkpoint_cli -- status --verbose

# สร้าง checkpoint ใหม่
cargo run --bin checkpoint_cli -- create \
  --height 750000 \
  --hash 1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef \
  --reason "Emergency rollback" \
  --operator operator1

# ดูรายการ checkpoints
cargo run --bin checkpoint_cli -- list

# ดู checkpoints ถึง height ที่กำหนด
cargo run --bin checkpoint_cli -- list --up-to 700000

# Rollback checkpoints
cargo run --bin checkpoint_cli -- rollback --height 700000

# เปิด/ปิด checkpoint validation
cargo run --bin checkpoint_cli -- toggle --enable true
cargo run --bin checkpoint_cli -- toggle --enable false

# แบน peer
cargo run --bin checkpoint_cli -- ban --peer-id attacker1 --reason "Malicious activity"

# ส่ง alert
cargo run --bin checkpoint_cli -- alert --message "Critical: Update required immediately"
```

---

## 📋 สถานการณ์การใช้งานจริง

### Scenario 1: Mining Bug Detection

```bash
# 1. หยุดการประมวลผลทันที
cargo run --bin checkpoint_cli -- toggle --enable false

# 2. สร้าง checkpoint ที่ block สุดท้ายที่ปลอดภัย
cargo run --bin checkpoint_cli -- create \
  --height 798500 \
  --hash a1b2c3d4e5f6789012345678901234567890123456789012345678901234567890 \
  --reason "Mining bug rollback - invalid blocks after 798500" \
  --operator operator1

# 3. เปิด checkpoint validation
cargo run --bin checkpoint_cli -- toggle --enable true

# 4. แจ้ง miners
cargo run --bin checkpoint_cli -- alert --message "🚨 MINING BUG: Update to v1.2.5 immediately"
```

### Scenario 2: Network Attack Response

```bash
# 1. แบน malicious peers
cargo run --bin checkpoint_cli -- ban --peer-id attacker1 --reason "Network attack"
cargo run --bin checkpoint_cli -- ban --peer-id attacker2 --reason "Network attack"

# 2. ส่ง alert
cargo run --bin checkpoint_cli -- alert --message "🚨 NETWORK ATTACK: Malicious peers banned"

# 3. ถ้าจำเป็นต้อง rollback
cargo run --bin checkpoint_cli -- rollback --height 750000
```

---

## 🔧 การตั้งค่า Configuration

### Environment Variables

```bash
export BITQUAN_EMERGENCY_ENABLED=true
export BITQUAN_EMERGENCY_SIGNATURES=3
export BITQUAN_EMERGENCY_WINDOW=3600
export BITQUAN_AUTHORIZED_OPERATORS="op1,op2,op3"
```

### Configuration File (emergency.toml)

```toml
[emergency]
enabled = false
required_signatures = 3
response_window = 3600
authorized_operators = [
    "operator1",
    "operator2", 
    "operator3"
]

[checkpoint]
max_checkpoints = 100
min_interval = 1000
```

---

## 🧪 การทดสอบ

### การรัน Tests

```bash
# ทดสอบ checkpoint system
cargo test -p bitquan-consensus checkpoint

# ทดสอบ emergency system
cargo test -p bitquan-consensus emergency

# ทดสอบทั้งหมด
cargo test -p bitquan-consensus test_emergency

# ทดสอบ integration
cargo test -p bitquan-consensus test_checkpoint_validation
```

### การทดสอบด้วย CLI

```bash
# ทดสอบการสร้าง checkpoint
cargo run --bin checkpoint_cli -- create \
  --height 1000 \
  --hash 1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef \
  --reason "Test checkpoint" \
  --operator test_operator

# ทดสอบการดูสถานะ
cargo run --bin checkpoint_cli -- status --verbose

# ทดสอบการแบน peer
cargo run --bin checkpoint_cli -- ban --peer-id test_peer --reason "Test ban"
```

---

## 📊 Monitoring และ Maintenance

### การตรวจสอบสถานะ

```bash
# ดูสถานะทั้งหมด
cargo run --bin checkpoint_cli -- status --verbose

# ดู checkpoints ทั้งหมด
cargo run --bin checkpoint_cli -- list

# ตรวจสอบว่ามี checkpoint ที่ height นั้นหรือไม่
cargo run --bin checkpoint_cli -- list --up-to 750000
```

### การ Maintenance

```bash
# Cleanup checkpoints เก่า
cargo run --bin checkpoint_cli -- rollback --height 700000

# ปิด checkpoint validation หลังฉุกเฉิน
cargo run --bin checkpoint_cli -- toggle --enable false

# ดู action history
cargo run --bin checkpoint_cli -- status --verbose
```

---

## 🚨 ข้อควรระวัง

### สิ่งที่ต้องหลีกเหลี่ยม

- ❌ **อย่าสร้าง checkpoint บ่อยเกินไป** (ขั้นต่ำ 1000 blocks)
- ❌ **อย่าสร้าง checkpoint ในอนาคต** (ต้องเป็น block ที่ผ่านมาแล้ว)
- ❌ **อย่าเปิด emergency system ตลอดเวลา** (เปิดเฉพาะตอนฉุกเฉิน)
- ❌ **อย่าใช้ operator ID ที่ไม่ได้รับอนุญาต**
- ❌ **อย่า rollback โดยไม่ตรวจสอบให้ดี**

### สิ่งที่ควรทำ

- ✅ **ตรวจสอบ block hash จากหลายแหล่ง** ก่อนสร้าง checkpoint
- ✅ **บันทึกทุกการกระทำ** เพื่อการ audit
- ✅ **ทดสอบระบบ定期** (quarterly)
- ✅ **เก็บ backup ของ checkpoints**
- ✅ **ติดต่อ team ก่อนทำ emergency actions**

---

## 📞 การติดต่อและ Support

### ในกรณีฉุกเฉิน

- 📧 **Primary**: security@bitquan.network
- 🚨 **Emergency Hotline**: [Phone number]
- 💬 **Discord**: [Private channel]
- 📱 **SMS**: [Emergency number]

### Documentation

- 📖 [Full Emergency Procedures](docs/security/EMERGENCY_PROCEDURES.md)
- 📋 [Quick Reference](docs/security/EMERGENCY_QUICK_REFERENCE.md)
- 🔧 [API Documentation](docs/rpc/API_REFERENCE.md)
- 🧪 [Testing Guide](docs/dev/TESTING.md)

### Community

- 💬 **Discord**: [BitQuan Discord]
- 🐙 **GitHub**: [Issues and Discussions]
- 📧 **Mailing List**: [developers@bitquan.network]

---

## 📚 Examples และ Resources

### Code Examples

- 📁 [`examples/checkpoint_usage.rs`](examples/checkpoint_usage.rs) - ตัวอย่างการใช้งานจริง
- 📁 [`examples/checkpoint_cli.rs`](examples/checkpoint_cli.rs) - CLI tool
- 📁 [`crates/consensus/src/checkpoint.rs`](crates/consensus/src/checkpoint.rs) - Core implementation
- 📁 [`crates/consensus/src/emergency.rs`](crates/consensus/src/emergency.rs) - Emergency system

### Test Files

- 🧪 [`crates/consensus/src/tests.rs`](crates/consensus/src/tests.rs) - Comprehensive tests
- 📋 [`tests/`](tests/) - Integration tests

### Configuration

- ⚙️ [`config/`](config/) - Network configurations
- 🔐 [`docs/security/`](docs/security/) - Security documentation

---

## 📈 Performance และ Limits

### ระบบ Limits

- **Maximum Checkpoints**: 100 checkpoints
- **Minimum Interval**: 1000 blocks between checkpoints
- **Genesis Block**: Cannot be checkpointed
- **Future Checkpoints**: Not allowed
- **Required Signatures**: 3-5 (configurable)

### Performance Metrics

- **Checkpoint Creation**: < 1ms
- **Block Validation**: < 0.1ms per checkpoint
- **Memory Usage**: ~10MB for 100 checkpoints
- **Storage**: ~32KB per checkpoint

---

## 🔒 Security Considerations

### Access Control

- 🔐 **Multi-signature authorization**: ต้องการ 3-5 ลายเซ็น
- 👥 **Authorized operators**: รายชื่อ operators ที่ได้รับอนุญาต
- ⏰ **Time windows**: จำกัดเวลาในการตอบสนอง
- 📊 **Audit trail**: บันทึกทุกการกระทำ

### Protection Mechanisms

- 🛡️ **Rate limiting**: ป้องกันการสร้าง checkpoint บ่อยเกินไป
- 🚫 **Input validation**: ตรวจสอบข้อมูลทุกอย่าง
- 🔍 **Hash verification**: ตรวจสอบ block hash อย่างละเอียด
- 📝 **Immutable logs**: ไม่สามารถแก้ไขประวัติได้

---

**⚠️ Important**: Checkpoint system เป็นเครื่องมือด้านความปลอดภัยระดับสูง การใช้งานต้องมีการพิจารณาอย่างรอบคอบและได้รับอนุญาตจาก authorized operators เท่านั้น!

---

*Last Updated: 2025-11-09*  
*Version: 1.0*  
*Review Required: Every 6 months*