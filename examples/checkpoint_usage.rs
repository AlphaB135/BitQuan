#!/usr/bin/env rust-script
//! BitQuan Checkpoint System - ตัวอย่างการใช้งานจริง
//! 
//! สคริปต์นี้แสดงวิธีการใช้งาน checkpoint system ในสถานการณ์ต่างๆ
//! 
//! การรัน: `cargo run --bin checkpoint_example`

use std::error::Error;
use bitquan_consensus::{
    CheckpointManager, EmergencyManager, EmergencyConfig, EmergencyAction
};

fn main() -> Result<(), Box<dyn Error>> {
    println!("🚀 BitQuan Checkpoint System - ตัวอย่างการใช้งาน");
    println!("=" .repeat(50));

    // 1. การตั้งค่าเบื้องต้น
    setup_example()?;

    // 2. สถานการณ์ Mining Bug
    mining_bug_scenario()?;

    // 3. สถานการณ์ Network Attack  
    network_attack_scenario()?;

    // 4. การตรวจสอบสถานะ
    monitoring_example()?;

    println!("\n✅ ตัวอย่างทั้งหมดเสร็จสิ้น!");
    Ok(())
}

fn setup_example() -> Result<(), Box<dyn Error>> {
    println!("\n📋 1. การตั้งค่าเบื้องต้น");
    println!("-".repeat(30));

    // สร้าง emergency manager สำหรับ development
    let config = EmergencyConfig {
        enabled: true,
        required_signatures: 1,  // ลดลงใน dev
        response_window: 300,    // 5 นาที
        authorized_operators: vec!["dev_operator".to_string()],
    };

    let mut emergency_manager = EmergencyManager::new(config);
    emergency_manager.update_height(100000);

    println!("✅ Emergency manager พร้อมใช้งาน");
    println!("   - Current height: 100000");
    println!("   - Required signatures: 1");
    println!("   - Authorized operators: 1");

    Ok(())
}

fn mining_bug_scenario() -> Result<(), Box<dyn Error>> {
    println!("\n🚨 2. สถานการณ์: Mining Bug Detection");
    println!("-".repeat(30));

    let config = EmergencyConfig {
        enabled: true,
        required_signatures: 1,
        response_window: 300,
        authorized_operators: vec!["operator1".to_string()],
    };

    let mut emergency_manager = EmergencyManager::new(config);
    emergency_manager.update_height(800000);

    println!("🔍 ตรวจพบ mining bug ที่ height 800000");

    // Step 1: หยุดการประมวลผล
    println!("\n⏸️  Step 1: หยุดการประมวลผลทันที");
    let pause_action = EmergencyAction::PauseProcessing;
    emergency_manager.execute_action(pause_action, "operator1")?;
    println!("   ✅ Block processing ถูกหยุดแล้ว");

    // Step 2: หา block สุดท้ายที่ถูกต้อง
    println!("\n🔍 Step 2: ค้นหา block สุดท้ายที่ปลอดภัย");
    let last_safe_height = 798500;
    let last_safe_hash = [0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0,
                         0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88,
                         0x99, 0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff, 0x00,
                         0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88];
    println!("   📍 Safe height: {}", last_safe_height);
    println!("   🔐 Safe hash: {:02x}{:02x}{:02x}{:02x}...", 
             last_safe_hash[0], last_safe_hash[1], 
             last_safe_hash[2], last_safe_hash[3]);

    // Step 3: สร้าง emergency checkpoint
    println!("\n🛡️ Step 3: สร้าง emergency checkpoint");
    emergency_manager.create_emergency_checkpoint(
        last_safe_height,
        last_safe_hash,
        "Mining bug rollback - invalid blocks after 798500".to_string(),
        "operator1"
    )?;
    println!("   ✅ Checkpoint สร้างสำเร็จ");

    // Step 4: เปิด checkpoint validation
    println!("\n🔓 Step 4: เปิด checkpoint validation");
    let enable_action = EmergencyAction::EnableCheckpoints;
    emergency_manager.execute_action(enable_action, "operator1")?;
    println!("   ✅ Checkpoint validation เปิดใช้งานแล้ว");

    // Step 5: แจ้ง miners
    println!("\n📢 Step 5: แจ้ง miners ให้อัปเดต");
    let alert_action = EmergencyAction::SendAlert {
        message: "🚨 MINING BUG: Update to v1.2.5 immediately. Rollback to 798500.".to_string(),
    };
    emergency_manager.execute_action(alert_action, "operator1")?;
    println!("   ✅ ส่ง alert ให้ miners แล้ว");

    // แสดงสถานะสุดท้าย
    let status = emergency_manager.get_status();
    println!("\n📊 สถานะหลังจัดการ:");
    println!("   - Processing Paused: {}", status.processing_paused);
    println!("   - Checkpoints Enabled: {}", status.checkpoints_enabled);
    println!("   - Checkpoint Count: {}", status.checkpoint_count);

    Ok(())
}

fn network_attack_scenario() -> Result<(), Box<dyn Error>> {
    println!("\n🚨 3. สถานการณ์: Network Attack Response");
    println!("-".repeat(30));

    let config = EmergencyConfig {
        enabled: true,
        required_signatures: 1,
        response_window: 300,
        authorized_operators: vec!["operator1".to_string()],
    };

    let mut emergency_manager = EmergencyManager::new(config);
    emergency_manager.update_height(600000);

    println!("🔍 ตรวจพบการโจมตีทาง network");

    // Step 1: ระบุ malicious peers
    println!("\n🎯 Step 1: ระบุ malicious peers");
    let malicious_peers = vec![
        "attacker_node_1".to_string(),
        "attacker_node_2".to_string(),
        "suspicious_peer_3".to_string(),
    ];
    println!("   🚫 พบ {} malicious peers", malicious_peers.len());
    for peer in &malicious_peers {
        println!("      - {}", peer);
    }

    // Step 2: แบน peers ทันที
    println!("\n🔒 Step 2: แบน malicious peers");
    let ban_action = EmergencyAction::BanPeers {
        peer_ids: malicious_peers.clone(),
    };
    emergency_manager.execute_action(ban_action, "operator1")?;
    println!("   ✅ แบน peers สำเร็จ");

    // Step 3: ตรวจสอบว่าโดนแบนจริง
    println!("\n🔍 Step 3: ตรวจสอบสถานะการแบน");
    for peer in &malicious_peers {
        if emergency_manager.is_peer_banned(peer) {
            let reason = emergency_manager.get_ban_reason(peer).unwrap_or("Unknown");
            println!("   ✅ {} ถูกแบน (เหตุผล: {})", peer, reason);
        }
    }

    // Step 4: ส่ง alert
    println!("\n📢 Step 4: ส่ง alert ให้ network operators");
    let alert_action = EmergencyAction::SendAlert {
        message: "🚨 NETWORK ATTACK: 3 malicious peers banned. Update firewall rules.".to_string(),
    };
    emergency_manager.execute_action(alert_action, "operator1")?;
    println!("   ✅ ส่ง alert สำเร็จ");

    // Step 5: ถ้าจำเป็นต้อง rollback
    println!("\n🔄 Step 5: ประเมินความจำเป็นในการ rollback");
    let chain_compromised = true; // สมมติว่า chain state เสียหาย
    
    if chain_compromised {
        println!("   ⚠️  Chain state เสียหาย ต้อง rollback");
        let rollback_action = EmergencyAction::RollbackTo { height: 580000 };
        emergency_manager.execute_action(rollback_action, "operator1")?;
        println!("   ✅ Rollback ไปยัง height 580000 สำเร็จ");
    } else {
        println!("   ✅ Chain state ปลอดภัย ไม่ต้อง rollback");
    }

    Ok(())
}

fn monitoring_example() -> Result<(), Box<dyn Error>> {
    println!("\n📊 4. การตรวจสอบและ Monitoring");
    println!("-".repeat(30));

    let config = EmergencyConfig {
        enabled: true,
        required_signatures: 3,
        response_window: 3600,
        authorized_operators: vec![
            "operator1".to_string(),
            "operator2".to_string(), 
            "operator3".to_string(),
        ],
    };

    let mut emergency_manager = EmergencyManager::new(config);
    emergency_manager.update_height(900000);

    // เพิ่มข้อมูลตัวอย่าง
    let test_hash = [0x99; 32];
    emergency_manager.create_emergency_checkpoint(
        850000,
        test_hash,
        "Test checkpoint for monitoring".to_string(),
        "operator1"
    )?;

    // แสดงสถานะทั้งหมด
    println!("📈 สถานะ Emergency System:");
    let status = emergency_manager.get_status();
    println!("   🟢 Enabled: {}", status.enabled);
    println!("   ⏸️  Processing Paused: {}", status.processing_paused);
    println!("   🛡️  Checkpoints Enabled: {}", status.checkpoints_enabled);
    println!("   📊 Checkpoint Count: {}", status.checkpoint_count);
    println!("   🚫 Banned Peers: {}", status.banned_peers_count);
    println!("   📜 Action History: {}", status.action_history_count);
    println!("   📏 Current Height: {}", status.current_height);

    // แสดง checkpoints
    println!("\n🛡️ Checkpoints:");
    let checkpoint_manager = emergency_manager.checkpoint_manager();
    let checkpoints = checkpoint_manager.export();
    
    for (i, cp) in checkpoints.iter().enumerate() {
        println!("   {}. Height {}: {}", i + 1, cp.height, cp.reason);
        println!("      Created: {}", cp.created_at);
        println!("      Hash: {:02x}{:02x}...{:02x}{:02x}", 
                 cp.hash[0], cp.hash[1], cp.hash[30], cp.hash[31]);
    }

    // แสดง action history
    println!("\n📜 Action History:");
    let history = emergency_manager.get_action_history();
    for (i, action) in history.iter().enumerate() {
        match action {
            EmergencyAction::PauseProcessing => {
                println!("   {}. ⏸️  Pause Processing", i + 1);
            }
            EmergencyAction::EnableCheckpoints => {
                println!("   {}. 🛡️  Enable Checkpoints", i + 1);
            }
            EmergencyAction::AddCheckpoint(_) => {
                println!("   {}. 📍 Add Checkpoint", i + 1);
            }
            EmergencyAction::RollbackTo { height } => {
                println!("   {}. 🔄 Rollback to {}", i + 1, height);
            }
            EmergencyAction::BanPeers { peer_ids } => {
                println!("   {}. 🚫 Ban {} peers", i + 1, peer_ids.len());
            }
            EmergencyAction::SendAlert { message } => {
                println!("   {}. 📢 Send Alert: {}", i + 1, 
                         message.chars().take(50).collect::<String>());
            }
        }
    }

    // แสดง banned peers
    let banned_peers = emergency_manager.get_banned_peers();
    if !banned_peers.is_empty() {
        println!("\n🚫 Banned Peers:");
        for (peer_id, reason) in banned_peers {
            println!("   - {}: {}", peer_id, reason);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_setup_example() {
        assert!(setup_example().is_ok());
    }

    #[test]
    fn test_mining_bug_scenario() {
        assert!(mining_bug_scenario().is_ok());
    }

    #[test]
    fn test_network_attack_scenario() {
        assert!(network_attack_scenario().is_ok());
    }

    #[test]
    fn test_monitoring_example() {
        assert!(monitoring_example().is_ok());
    }
}