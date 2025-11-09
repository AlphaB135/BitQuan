# BitQuan Checkpoint System - คู่มือการใช้งาน

## 📋 ภาพรวมการใช้งาน

Checkpoint system ใน BitQuan ถูกออกแบบมาเพื่อการกู้คืน blockchain ในสถานการณ์ฉุกเฉิน โดยมีวัตถุประสงค์หลักคือ:

- 🛡️ ป้องกันการโจมตีทาง consensus
- 🔄 กู้คืน blockchain จาก mining bugs
- 🚨 ตอบสนองต่อเหตุการณ์ฉุกเฉิน
- 📊 ตรวจสอบความถูกต้องของ blocks

---

## 🚀 การเริ่มต้นใช้งาน

### 1. การตั้งค่าเบื้องต้น

```rust
use bitquan_consensus::{CheckpointManager, EmergencyManager, EmergencyConfig};

// สร้าง checkpoint manager (ปกติจะปิดไว้ก่อน)
let mut checkpoint_manager = CheckpointManager::new(false);

// ตั้งค่า emergency manager
let config = EmergencyConfig {
    enabled: true,
    required_signatures: 3,  // ต้องการ 3 ลายเซ็น
    response_window: 3600,    // 1 ชั่วโมง
    authorized_operators: vec![
        "operator1".to_string(),
        "operator2".to_string(),
        "operator3".to_string(),
    ],
};

let mut emergency_manager = EmergencyManager::new(config);
```

### 2. การอัปเดตความสูงของ blockchain

```rust
// อัปเดตความสูงปัจจุบันเสมอ
emergency_manager.update_height(current_block_height);
checkpoint_manager.update_height(current_block_height);
```

---

## 🔐 การสร้าง Checkpoint

### สถานการณ์ที่ต้องสร้าง Checkpoint

1. **Mining Bug** - เกิดข้อผิดพลาดใน mining algorithm
2. **Network Attack** - มีการโจมตีทาง network
3. **Consensus Failure** - consensus rules มีปัญหา
4. **Software Vulnerability** - พบช่องโหว่ด้านความปลอดภัย

### วิธีการสร้าง Emergency Checkpoint

```rust
use bitquan_consensus::Checkpoint;

// 1. ระบุ block ที่ปลอดภัย (ต้องเก่ากว่า 1000 blocks)
let safe_height = 750000;
let safe_hash = [0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0,
                 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88,
                 0x99, 0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff, 0x00,
                 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88];

// 2. สร้าง checkpoint
let result = emergency_manager.create_emergency_checkpoint(
    safe_height,
    safe_hash,
    "Mining bug fix - rollback to safe state".to_string(),
    "operator1"  // operator ID ที่ได้รับอนุญาต
);

match result {
    Ok(()) => println!("✅ Checkpoint created successfully"),
    Err(e) => println!("❌ Failed to create checkpoint: {}", e),
}
```

### การสร้าง Checkpoint แบบ Manual

```rust
// สร้าง checkpoint โดยตรง (ถ้ามีสิทธิ์)
let checkpoint = Checkpoint::new(
    750000,
    safe_hash,
    "Manual checkpoint for testing".to_string()
);

// เพิ่มเข้าไปใน manager
checkpoint_manager.add_checkpoint(checkpoint)?;
```

---

## 🛡️ การใช้งาน Emergency Actions

### 1. การหยุดการประมวลผล (Pause Processing)

```rust
use bitquan_consensus::EmergencyAction;

// หยุดการประมวลผลทันที
let action = EmergencyAction::PauseProcessing;
emergency_manager.execute_action(action, "operator1")?;

// ตรวจสอบสถานะ
if emergency_manager.is_processing_paused() {
    println!("⏸️ Block processing is paused");
}
```

### 2. การเปิดใช้งาน Checkpoint Validation

```rust
// เปิด checkpoint validation
let action = EmergencyAction::EnableCheckpoints;
emergency_manager.execute_action(action, "operator1")?;

// ตรวจสอบว่าเปิดใช้งานแล้ว
if emergency_manager.checkpoint_manager().is_enabled() {
    println!("✅ Checkpoint validation is enabled");
}
```

### 3. การ Rollback ไปยังความสูงที่กำหนด

```rust
// Rollback ไปยังความสูงที่ปลอดภัย
let action = EmergencyAction::RollbackTo { height: 740000 };
emergency_manager.execute_action(action, "operator1")?;
```

### 4. การแบน Peer ที่เป็นอันตราย

```rust
// แบน malicious peers
let action = EmergencyAction::BanPeers {
    peer_ids: vec![
        "malicious_peer_1".to_string(),
        "attacker_node".to_string(),
    ],
};
emergency_manager.execute_action(action, "operator1")?;

// ตรวจสอบว่า peer โดนแบนหรือไม่
if emergency_manager.is_peer_banned("malicious_peer_1") {
    println!("🚫 Peer is banned");
}
```

### 5. การส่ง Alert ให้ Network Operators

```rust
// ส่งข้อความแจ้งเตือน
let action = EmergencyAction::SendAlert {
    message: "🚨 CRITICAL: Mining bug detected. Update to v1.2.4 immediately!".to_string(),
};
emergency_manager.execute_action(action, "operator1")?;
```

---

## 🔍 การตรวจสอบและ Monitoring

### การตรวจสอบสถานะ Emergency System

```rust
// ดูสถานะทั้งหมด
let status = emergency_manager.get_status();
println!("Emergency Status:");
println!("  Enabled: {}", status.enabled);
println!("  Processing Paused: {}", status.processing_paused);
println!("  Checkpoints Enabled: {}", status.checkpoints_enabled);
println!("  Checkpoint Count: {}", status.checkpoint_count);
println!("  Banned Peers: {}", status.banned_peers_count);
println!("  Current Height: {}", status.current_height);
```

### การตรวจสอบ Checkpoints

```rust
// ดู checkpoints ทั้งหมด
let checkpoints = checkpoint_manager.export();
for cp in checkpoints {
    println!("Checkpoint at height {}: {}", cp.height, cp.reason);
}

// ดู checkpoint ล่าสุด
if let Some(latest) = checkpoint_manager.get_latest_checkpoint(current_height) {
    println!("Latest checkpoint: height {}, created at {}", 
             latest.height, latest.created_at);
}

// ตรวจสอบว่ามี checkpoint ที่ความสูงนั้นหรือไม่
if checkpoint_manager.has_checkpoint(750000) {
    println!("✅ Checkpoint exists at height 750000");
}
```

### การตรวจสอบ Banned Peers

```rust
// ดู peers ที่โดนแบนทั้งหมด
let banned_peers = emergency_manager.get_banned_peers();
for (peer_id, reason) in banned_peers {
    println!("Banned peer {}: {}", peer_id, reason);
}

// ตรวจสอบว่า peer โดนแบนด้วยเหตุผลอะไร
if let Some(reason) = emergency_manager.get_ban_reason("malicious_peer_1") {
    println!("Ban reason: {}", reason);
}
```

---

## ⚙️ การตั้งค่าใน Production

### Configuration File (TOML)

```toml
# emergency.toml
[emergency]
enabled = false  # ปิดไว้ก่อน จะเปิดเฉพาะตอนฉุกเฉิน
required_signatures = 5  # ต้องการ 5 ลายเซ็นใน production
response_window = 7200    # 2 ชั่วโมง
authorized_operators = [
    "mainnet_operator_1",
    "mainnet_operator_2", 
    "mainnet_operator_3",
    "mainnet_operator_4",
    "mainnet_operator_5"
]

[checkpoint]
max_checkpoints = 100
min_interval = 1000
```

### Environment Variables

```bash
# สำหรับการตั้งค่าแบบ dynamic
export BITQUAN_EMERGENCY_ENABLED=false
export BITQUAN_EMERGENCY_SIGNATURES=5
export BITQUAN_EMERGENCY_WINDOW=7200
export BITQUAN_AUTHORIZED_OPERATORS="op1,op2,op3,op4,op5"
```

---

## 🧪 การทดสอบระบบ

### การทดสอบใน Development

```rust
// สร้าง test environment
let config = EmergencyConfig {
    enabled: true,
    required_signatures: 1,  // ลดลงใน dev
    response_window: 300,    // 5 นาที
    authorized_operators: vec!["dev_operator".to_string()],
};

let mut manager = EmergencyManager::new(config);
manager.update_height(10000);

// ทดสอบการสร้าง checkpoint
let test_hash = [0x42; 32];
manager.create_emergency_checkpoint(
    5000,
    test_hash,
    "Test checkpoint".to_string(),
    "dev_operator"
)?;

// ทดสอบการ validate block
let result = manager.validate_block_emergency(5000, &test_hash);
assert!(result.is_ok());
```

### การรัน Tests

```bash
# ทดสอบ checkpoint system
cargo test -p bitquan-consensus checkpoint

# ทดสอบ emergency system  
cargo test -p bitquan-consensus emergency

# ทดสอบทั้งหมด
cargo test -p bitquan-consensus test_emergency
```

---

## 📋 ขั้นตอนการใช้งานจริง (Real-world Usage)

### Scenario 1: Mining Bug Detection

```rust
// 1. ตรวจพบปัญหา
println!("🚨 Mining bug detected at height 800000!");

// 2. หยุดการประมวลผลทันที
let pause_action = EmergencyAction::PauseProcessing;
emergency_manager.execute_action(pause_action, "operator1")?;

// 3. หา block สุดท้ายที่ถูกต้อง
let last_safe_height = 798500;
let last_safe_hash = get_block_hash(last_safe_height)?;

// 4. สร้าง emergency checkpoint
emergency_manager.create_emergency_checkpoint(
    last_safe_height,
    last_safe_hash,
    "Mining bug rollback - invalid blocks after 798500".to_string(),
    "operator1"
)?;

// 5. เปิด checkpoint validation
let enable_action = EmergencyAction::EnableCheckpoints;
emergency_manager.execute_action(enable_action, "operator1")?;

// 6. แจ้ง miners ให้อัปเดต software
let alert_action = EmergencyAction::SendAlert {
    message: "🚨 MINING BUG: Update to v1.2.5 immediately. Rollback to 798500.".to_string(),
};
emergency_manager.execute_action(alert_action, "operator1")?;

// 7. รอให้ network อัปเดต แล้วค่อยเปิดการประมวลผลอีกครั้ง
```

### Scenario 2: Network Attack Response

```rust
// 1. ตรวจพบการโจมตี
println!("🚨 Network attack detected!");

// 2. ระบุ malicious peers
let malicious_peers = vec![
    "attacker_node_1".to_string(),
    "attacker_node_2".to_string(),
];

// 3. แบน peers ทันที
let ban_action = EmergencyAction::BanPeers {
    peer_ids: malicious_peers.clone(),
};
emergency_manager.execute_action(ban_action, "operator1")?;

// 4. ส่ง alert ให้ทุกคนรู้
let alert_action = EmergencyAction::SendAlert {
    message: "🚨 NETWORK ATTACK: Malicious peers banned. Update firewall rules.".to_string(),
};
emergency_manager.execute_action(alert_action, "operator1")?;

// 5. ถ้า chain state เสียหาย ให้ rollback
if chain_state_compromised() {
    let rollback_action = EmergencyAction::RollbackTo { height: 750000 };
    emergency_manager.execute_action(rollback_action, "operator1")?;
}
```

---

## 🔧 การบำรุงรักษา

### การ Cleanup Checkpoints เก่า

```rust
// ลบ checkpoints ที่เก่ากว่า 30 วัน
let thirty_days_ago = current_timestamp() - (30 * 24 * 60 * 60);
let mut to_remove = Vec::new();

for checkpoint in checkpoint_manager.export() {
    if checkpoint.created_at < thirty_days_ago {
        to_remove.push(checkpoint.height);
    }
}

for height in to_remove {
    checkpoint_manager.rollback_to(height - 1);
}
```

### การ Export/Import Checkpoints

```rust
// Export สำหรับ backup
let checkpoints = checkpoint_manager.export();
let backup_data = serde_json::to_string(&checkpoints)?;

// Import จาก backup
let backup_checkpoints: Vec<Checkpoint> = serde_json::from_str(&backup_data)?;
checkpoint_manager.import(backup_checkpoints)?;
```

---

## 🚨 ข้อควรระวัง

### สิ่งที่ต้องหลีกเหลี่ยง

1. **❌ อย่าสร้าง checkpoint บ่อยเกินไป** (ขั้นต่ำ 1000 blocks)
2. **❌ อย่าสร้าง checkpoint ในอนาคต** (ต้องเป็น block ที่ผ่านมาแล้ว)
3. **❌ อย่าเปิด emergency system ตลอดเวลา** (เปิดเฉพาะตอนฉุกเฉิน)
4. **❌ อย่าใช้ operator ID ที่ไม่ได้รับอนุญาต**

### สิ่งที่ควรทำ

1. **✅ ตรวจสอบ block hash จากหลายแหล่งก่อนสร้าง checkpoint**
2. **✅ บันทึกทุกการกระทำเพื่อการ audit**
3. **✅ ทดสอบระบบ定期**
4. **✅ เก็บ backup ของ checkpoints**

---

## 📞 การติดต่อและ Support

### ในกรณีฉุกเฉิน

1. **Primary Contact**: security@bitquan.network
2. **Emergency Channel**: [Discord/Telegram private channel]
3. **Operator Hotline**: [Phone number]

### Documentation

- 📖 [Full Emergency Procedures](EMERGENCY_PROCEDURES.md)
- 📋 [Quick Reference](EMERGENCY_QUICK_REFERENCE.md)
- 🔧 [API Documentation](../rpc/API_REFERENCE.md)

---

**⚠️ Important**: Checkpoint system เป็นเครื่องมือด้านความปลอดภัยระดับสูง การใช้งานต้องมีการพิจารณาอย่างรอบคอบและได้รับอนุญาตจาก authorized operators เท่านั้น!