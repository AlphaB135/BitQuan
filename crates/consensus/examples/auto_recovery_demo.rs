//! BitQuan Auto-Recovery System Demo
//! 
//! ตัวอย่างการใช้งานระบบกู้คืนอัตโนมัติ

use bitquan_consensus::{
    RecoveryConfig, AutoRecoveryManager, StateSnapshot
};

use std::time::{SystemTime, UNIX_EPOCH};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("BitQuan: BitQuan Auto-Recovery System Demo");
    println!("=====================================");
    
    // 1. สร้าง Auto-Recovery Manager
    let config = RecoveryConfig {
        enabled: true,
        max_rollback_blocks: 1000,
        min_confirmations: 50,
        override_signatures: 2,
        anomaly_threshold_percent: 5,
    };
    
    let checkpoint_manager = bitquan_consensus::CheckpointManager::new(true);
    let monitor = bitquan_consensus::Monitor::default();
    let mut manager = AutoRecoveryManager::new(config, checkpoint_manager, monitor);
    

    
    println!("SUCCESS: Auto-Recovery Manager initialized");
    println!("Status: Initial status: {:?}", manager.status());
    println!();
    
    // 3. บันทึก blocks ปกติ
    println!("Recording: Recording normal blocks...");
    for height in 1000..=1010 {
        let hash = [height as u8; 32];
        let state_root = [(height + 100) as u8; 32];
        manager.process_block(height, hash, state_root)?;
        print!(".");
    }
    println!(" Done!");
    println!("Status: Status after normal blocks: {:?}", manager.status());
    println!();
    
    // 4. สร้าง block ที่มีปัญหา (Hash mismatch)
    println!("Alert: Creating problematic block (hash mismatch)...");
    let bad_hash = [0xFFu8; 32];
    let state_root = [(1011 + 100) as u8; 32];
    
    manager.process_block(1011, bad_hash, state_root)?;
    println!("Status: Status after bad block: {:?}", manager.status());
    println!();
    
    // 5. แสดงรายงานความผิดปกติ
    println!("Reports: Anomaly Reports:");
    for (i, anomaly) in manager.get_recovery_history().iter().enumerate() {
        println!("  {}. {:?}", i+1, anomaly.anomaly_type);
        println!("     Height: {}", anomaly.height);
        println!("     Context: {}", anomaly.context);
    }
    println!();
    
    // 6. แสดงสถิติ
    let stats = manager.get_recovery_stats();
    println!("Statistics: Recovery Statistics:");
    println!("  Snapshot count: {}", stats.snapshot_count);
    println!("  Recovery count: {}", stats.recovery_count);
    println!("  Last safe height: {}", stats.last_safe_height);
    println!("  Override signatures: {}", stats.override_signatures);
    println!();
    
    // 7. ทดลอง manual override
    println!("Override: Attempting manual override...");
    let result = manager.manual_override(
        "sig_abc123", 
        "False positive - legitimate block"
    );
    
    match result {
        Ok(_) => println!("SUCCESS: Override signature added"),
        Err(e) => println!("Error: Override failed: {}", e),
    }
    
    // 8. ต้องการ signature อีกอัน
    println!();
    println!("Override: Adding second signature...");
    let result = manager.manual_override(
        "sig_def456", 
        "Confirmed false positive"
    );
    
    match result {
        Ok(_) => println!("SUCCESS: Manual override activated!"),
        Err(e) => println!("Error: Override failed: {}", e),
    }
    
    println!("Status: Final status: {:?}", manager.status());
    println!();
    
    // 9. ทดลอง manual rollback
    println!("Manual: Performing manual rollback to height 1005...");
    let signatures = vec!["sig_abc123".to_string(), "sig_def456".to_string()];
    let result = manager.manual_rollback(1005, signatures);
    
    match result {
        Ok(_) => println!("SUCCESS: Manual rollback successful"),
        Err(e) => println!("Error: Manual rollback failed: {}", e),
    }
    
    // 10. สถิติสุดท้าย
    let final_stats = manager.get_recovery_stats();
    println!();
    println!("Status: Final Statistics:");
    println!("  Snapshot count: {}", final_stats.snapshot_count);
    println!("  Recovery count: {}", final_stats.recovery_count);
    println!("  Last safe height: {}", final_stats.last_safe_height);
    println!("  Current status: {:?}", final_stats.status);
    
    // 11. แสดง snapshots ล่าสุด
    println!();
    println!("Snapshots: Recent Snapshots:");
    let history = manager.get_recovery_history();
    for (i, snapshot) in history.iter().take(3).enumerate() {
        println!("  {}. Height: {}, Type: {:?}", 
                 i+1, snapshot.height, snapshot.anomaly_type);
    }
    
    println!();
    println!("Complete: Demo completed successfully!");
    
    Ok(())
}

/// สร้าง state snapshot ปกติ
#[allow(dead_code)]
fn create_normal_state_snapshot(height: u64) -> StateSnapshot {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    
    StateSnapshot {
        height,
        hash: [height as u8; 32],
        state_root: [(height + 100) as u8; 32],
        timestamp: now,
        verified_safe: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_demo_flow() {
        // ทดสอบ flow การทำงานของ demo
        let config = RecoveryConfig::default();
        let checkpoint_manager = bitquan_consensus::CheckpointManager::new(true);
        let monitor = bitquan_consensus::Monitor::default();
        let mut manager = AutoRecoveryManager::new(config, checkpoint_manager, monitor);
        
        // บันทึก block ปกติ
        let hash = [1000u8; 32];
        let state_root = [1100u8; 32];
        assert!(manager.process_block(1000, hash, state_root).is_ok());
        assert_eq!(manager.get_status(), RecoveryStatus::Normal);
        
        // บันทึก block ที่มีปัญหา (hash mismatch)
        let bad_hash = [0xFFu8; 32];
        let bad_state_root = [1101u8; 32];
        assert!(manager.process_block(1001, bad_hash, bad_state_root).is_ok());
        
        // ตรวจสอบว่ามี anomaly
        assert!(!manager.get_recovery_history().is_empty());
    }
}