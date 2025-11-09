//! BitQuan Auto-Recovery System Demo
//! 
//! ตัวอย่างการใช้งานระบบกู้คืนอัตโนมัติ

use bitquan_consensus::{
    AutoRecoveryConfig, AutoRecoveryManager, BlockSnapshot
};
use bitquan_consensus::auto_recovery::{BlockMetrics, AnomalyType, RecoveryStatus};
use std::time::{SystemTime, UNIX_EPOCH};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("BitQuan: BitQuan Auto-Recovery System Demo");
    println!("=====================================");
    
    // 1. สร้าง Auto-Recovery Manager
    let config = AutoRecoveryConfig {
        memory_blocks: 1000,
        safe_confirmations: 50,
        anomaly_threshold: 0.05,
        rollback_delay: 10, // 10 วินาทีสำหรับ demo
        override_signatures: 2,
    };
    
    let mut manager = AutoRecoveryManager::new(config);
    

    
    println!("SUCCESS: Auto-Recovery Manager initialized");
    println!("Status: Initial status: {:?}", manager.get_status());
    println!();
    
    // 3. บันทึก blocks ปกติ
    println!("Recording: Recording normal blocks...");
    for height in 1000..=1010 {
        let snapshot = create_normal_block(height);
        manager.record_block(snapshot)?;
        print!(".");
    }
    println!(" Done!");
    println!("Status: Status after normal blocks: {:?}", manager.get_status());
    println!();
    
    // 4. สร้าง block ที่มีปัญหา (Block size ใหญ่ผิดปกติ)
    println!("Alert: Creating problematic block (oversized)...");
    let mut bad_block = create_normal_block(1011);
    bad_block.size = 50_000_000; // 50MB - ผิดปกติ
    bad_block.tx_count = 5000;
    
    manager.record_block(bad_block)?;
    println!("Status: Status after bad block: {:?}", manager.get_status());
    println!();
    
    // 5. แสดงรายงานความผิดปกติ
    println!("Reports: Anomaly Reports:");
    for (i, anomaly) in manager.get_anomalies().iter().enumerate() {
        println!("  {}. {:?}", i+1, anomaly.anomaly_type);
        println!("     Height: {}", anomaly.height);
        println!("     Severity: {}/100", anomaly.severity);
        println!("     Description: {}", anomaly.description);
    }
    println!();
    
    // 6. แสดงสถิติ
    let stats = manager.get_statistics();
    println!("Statistics: Recovery Statistics:");
    println!("  Total snapshots: {}", stats.total_snapshots);
    println!("  Total anomalies: {}", stats.total_anomalies);
    println!("  Last safe height: {}", stats.last_safe_height);
    println!("  Memory usage: {} bytes", stats.memory_usage);
    println!();
    
    // 7. ทดลอง manual override
    println!("Override: Attempting manual override...");
    let result = manager.manual_override(
        "sig_abc123", 
        "False positive - legitimate large block"
    );
    
    match result {
        Ok(_) => println!("SUCCESS: Override accepted from auto-recovery system"),
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
    
    println!("Status: Final status: {:?}", manager.get_status());
    println!();
    
    // 9. ทดลอง manual rollback
    println!("Manual: Performing manual rollback to height 1005...");
    let result = manager.manual_rollback(1005, "Manual rollback test");
    
    match result {
        Ok(_) => println!("SUCCESS: Manual rollback successful"),
        Err(e) => println!("Error: Manual rollback failed: {}", e),
    }
    
    // 10. สถิติสุดท้าย
    let final_stats = manager.get_statistics();
    println!();
    println!("Status: Final Statistics:");
    println!("  Total snapshots: {}", final_stats.total_snapshots);
    println!("  Total anomalies: {}", final_stats.total_anomalies);
    println!("  Last safe height: {}", final_stats.last_safe_height);
    println!("  Current status: {:?}", final_stats.current_status);
    
    // 11. แสดง snapshots ล่าสุด
    println!();
    println!("Snapshots: Recent Snapshots:");
    for snapshot in manager.get_recent_snapshots(3) {
        println!("  Height {}: {} bytes, {} txs", 
                 snapshot.height, snapshot.size, snapshot.tx_count);
    }
    
    println!();
    println!("Complete: Demo completed successfully!");
    
    Ok(())
}

/// สร้าง block snapshot ปกติ
fn create_normal_block(height: u64) -> BlockSnapshot {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    
    BlockSnapshot {
        height,
        hash: [height as u8; 32],
        timestamp: now,
        parent_hash: [(height-1) as u8; 32],
        merkle_root: [0u8; 32],
        difficulty: 0x1d00ffff,
        nonce: height,
        tx_count: 100 + (height % 50) as usize,
        size: 1_000_000 + (height % 100000) as usize,
        metrics: BlockMetrics {
            transaction_count: 100 + (height % 50) as usize,
            avg_size_100: 1_000_000,
            signature_count: 100 + (height % 50) as usize,
            gas_used: 21000 * (100 + (height % 50) as u64),
            orphan_rate: 0.01,
            avg_difficulty: 1000.0,
        },
        recorded_at: now,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_demo_flow() {
        // ทดสอบ flow การทำงานของ demo
        let config = AutoRecoveryConfig::default();
        let mut manager = AutoRecoveryManager::new(config);

        
        // บันทึก block ปกติ
        let snapshot = create_normal_block(1000);
        assert!(manager.record_block(snapshot).is_ok());
        assert_eq!(manager.get_status(), RecoveryStatus::Normal);
        
        // บันทึก block ที่มีปัญหา
        let mut bad_snapshot = create_normal_block(1001);
        bad_snapshot.size = 50_000_000;
        assert!(manager.record_block(bad_snapshot).is_ok());
        
        // ตรวจสอบว่ามี anomaly
        assert!(!manager.get_anomalies().is_empty());
    }
}