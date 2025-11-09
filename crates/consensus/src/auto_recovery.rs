//! BitQuan Auto-Recovery System - ระบบกู้คืนอัตโนมัติ
//! 
//! แนวคิด: บันทึกทุก block และ rollback อัตโนมัติเมื่อตรวจพบปัญหา
//! โดยมี manual override เฉพาะกรณีฉุกเฉินเท่านั้น

use std::collections::{BTreeMap, HashMap};
use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// ค่าคอนฟิกกูร์สำหรับ auto-recovery
pub struct AutoRecoveryConfig {
    /// จำนวน block ที่เก็บไว้ในหน่วยความจำ
    pub memory_blocks: usize,
    /// จำนวน confirmations ที่ต้องการก่อนถือว่า block ปลอดภัย
    pub safe_confirmations: u64,
    /// เปอร์เซ็นต์ความผิดปกติที่จะ trigger auto-rollback
    pub anomaly_threshold: f64,
    /// ระยะเวลาที่รอก่อน rollback (วินาที)
    pub rollback_delay: u64,
    /// จำนวน operators ที่ต้องยืนยันสำหรับ manual override
    pub override_signatures: u8,
}

impl Default for AutoRecoveryConfig {
    fn default() -> Self {
        Self {
            memory_blocks: 10000,      // เก็บ 10,000 blocks ล่าสุด
            safe_confirmations: 100,     // 100 confirmations = ปลอดภัย
            anomaly_threshold: 0.05,     // 5% anomaly trigger rollback
            rollback_delay: 300,         // รอ 5 นาทีก่อน rollback
            override_signatures: 3,       // ต้องการ 3 signatures สำหรับ override
        }
    }
}

/// Block snapshot สำหรับการ rollback
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockSnapshot {
    /// Block height
    pub height: u64,
    /// Block hash
    pub hash: [u8; 32],
    /// Block timestamp
    pub timestamp: u64,
    /// Parent hash
    pub parent_hash: [u8; 32],
    /// Merkle root
    pub merkle_root: [u8; 32],
    /// Difficulty
    pub difficulty: u32,
    /// Nonce
    pub nonce: u64,
    /// Transaction count
    pub tx_count: usize,
    /// Block size
    pub size: usize,
    /// Validation metrics
    pub metrics: BlockMetrics,
    /// เวลาที่บันทึก
    pub recorded_at: u64,
}

/// Metrics สำหรับตรวจสอบความผิดปกติ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockMetrics {
    /// จำนวน transactions
    pub transaction_count: usize,
    /// ขนาด block เฉลี่ย (100 blocks ล่าสุด)
    pub avg_size_100: usize,
    /// จำนวน signatures
    pub signature_count: usize,
    /// Gas used
    pub gas_used: u64,
    /// จำนวน orphan blocks ในช่วงเวลาเดียวกัน
    pub orphan_rate: f64,
    /// ความยากในการขุดเฉลี่ย
    pub avg_difficulty: f64,
}

/// ประเภทของปัญหาที่ตรวจพบ
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnomalyType {
    /// Block size ผิดปกติ (ใหญ่หรือเล็กเกินไป)
    InvalidSize,
    /// Transaction count ผิดปกติ
    InvalidTransactionCount,
    /// Signature verification ล้มเหลว
    SignatureFailure,
    /// Difficulty ผิดปกติ
    InvalidDifficulty,
    /// Orphan rate สูงผิดปกติ
    HighOrphanRate,
    /// Timestamp ผิดปกติ
    InvalidTimestamp,
    /// Gas usage ผิดปกติ
    InvalidGasUsage,
    /// การโจมตีทาง consensus
    ConsensusAttack,
    /// ปัญหาที่ไม่รู้จัก
    Unknown,
}

/// รายงานความผิดปกติ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyReport {
    /// ประเภทของความผิดปกติ
    pub anomaly_type: AnomalyType,
    /// Block height ที่เกิดปัญหา
    pub height: u64,
    /// Block hash ที่เกิดปัญหา
    pub hash: [u8; 32],
    /// รายละเอียดของปัญหา
    pub description: String,
    /// คะแนนความรุนแรง (0-100)
    pub severity: u8,
    /// เวลาที่ตรวจพบ
    pub detected_at: u64,
    /// ข้อเสนอแนะการแก้ไข
    pub recommendation: String,
}

/// สถานะของ auto-recovery
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecoveryStatus {
    /// ปกติ
    Normal,
    /// ตรวจพบความผิดปกติ
    AnomalyDetected,
    /// กำลังรอการยืนยัน rollback
    PendingRollback,
    /// กำลัง rollback
    RollingBack,
    /// กู้คืนสำเร็จ
    Recovered,
    /// ต้องการ manual intervention
    ManualIntervention,
}

/// Auto-Recovery Manager
pub struct AutoRecoveryManager {
    /// ค่าคอนฟิก
    config: AutoRecoveryConfig,
    /// Block snapshots ที่บันทึกไว้
    snapshots: BTreeMap<u64, BlockSnapshot>,
    /// รายงานความผิดปกติ
    anomalies: Vec<AnomalyReport>,
    /// สถานะปัจจุบัน
    status: RecoveryStatus,
    /// Block height ที่ปลอดภัยล่าสุด
    last_safe_height: u64,
    /// Block hash ที่ปลอดภัยล่าสุด
    last_safe_hash: [u8; 32],
    /// เวลาที่ตรวจพบปัญหาล่าสุด
    last_anomaly_time: u64,
    /// Operators ที่ได้รับอนุญาต
    authorized_operators: Vec<String>,
    /// Override signatures ที่รับมา
    override_signatures: HashMap<String, String>,
}

impl AutoRecoveryManager {
    /// สร้าง auto-recovery manager ใหม่
    pub fn new(config: AutoRecoveryConfig) -> Self {
        Self {
            config,
            snapshots: BTreeMap::new(),
            anomalies: Vec::new(),
            status: RecoveryStatus::Normal,
            last_safe_height: 0,
            last_safe_hash: [0u8; 32],
            last_anomaly_time: 0,
            authorized_operators: Vec::new(),
            override_signatures: HashMap::new(),
        }
    }

    /// ตั้งค่า authorized operators
    pub fn set_authorized_operators(&mut self, operators: Vec<String>) {
        self.authorized_operators = operators;
    }

    /// บันทึก block snapshot
    pub fn record_block(&mut self, snapshot: BlockSnapshot) -> Result<(), AutoRecoveryError> {
        // บันทึก snapshot
        self.snapshots.insert(snapshot.height, snapshot.clone());

        // ตรวจสอบความผิดปกติ
        if let Some(anomaly) = self.detect_anomaly(&snapshot) {
            self.handle_anomaly(anomaly)?;
        }

        // ถ้าไม่มีปัญหา อัปเดต safe height
        if self.status == RecoveryStatus::Normal {
            self.last_safe_height = snapshot.height;
            self.last_safe_hash = snapshot.hash;
        }

        // ลบ snapshots เก่าเกินไป
        self.cleanup_old_snapshots();

        Ok(())
    }

    /// ตรวจสอบความผิดปกติใน block
    fn detect_anomaly(&self, snapshot: &BlockSnapshot) -> Option<AnomalyReport> {
        let mut anomalies = Vec::new();

        // ตรวจสอบ block size
        if snapshot.size > 10_000_000 || snapshot.size < 1000 {
            anomalies.push(AnomalyReport {
                anomaly_type: AnomalyType::InvalidSize,
                height: snapshot.height,
                hash: snapshot.hash,
                description: format!("Block size {} is abnormal", snapshot.size),
                severity: 80,
                detected_at: current_timestamp(),
                recommendation: "Rollback to previous safe block".to_string(),
            });
        }

        // ตรวจสอบ transaction count
        if snapshot.tx_count > 10000 || snapshot.tx_count == 0 {
            anomalies.push(AnomalyReport {
                anomaly_type: AnomalyType::InvalidTransactionCount,
                height: snapshot.height,
                hash: snapshot.hash,
                description: format!("Transaction count {} is abnormal", snapshot.tx_count),
                severity: 70,
                detected_at: current_timestamp(),
                recommendation: "Investigate transaction pattern".to_string(),
            });
        }

        // ตรวจสอบ orphan rate
        if snapshot.metrics.orphan_rate > 0.1 {
            anomalies.push(AnomalyReport {
                anomaly_type: AnomalyType::HighOrphanRate,
                height: snapshot.height,
                hash: snapshot.hash,
                description: format!("Orphan rate {:.2}% is too high", snapshot.metrics.orphan_rate * 100.0),
                severity: 90,
                detected_at: current_timestamp(),
                recommendation: "Immediate rollback required".to_string(),
            });
        }

        // ตรวจสอบ timestamp
        let now = current_timestamp();
        if snapshot.timestamp > now + 3600 || snapshot.timestamp < now - 86400 {
            anomalies.push(AnomalyReport {
                anomaly_type: AnomalyType::InvalidTimestamp,
                height: snapshot.height,
                hash: snapshot.hash,
                description: "Block timestamp is out of range".to_string(),
                severity: 85,
                detected_at: current_timestamp(),
                recommendation: "Rollback and investigate clock sync".to_string(),
            });
        }

        // เลือก anomaly ที่รุนแรงที่สุด
        anomalies.into_iter().max_by_key(|a| a.severity)
    }

    /// จัดการกับความผิดปกติที่ตรวจพบ
    fn handle_anomaly(&mut self, anomaly: AnomalyReport) -> Result<(), AutoRecoveryError> {
        self.anomalies.push(anomaly.clone());
        self.last_anomaly_time = anomaly.detected_at;

        // ถ้า severity สูง เริ่มกระบวนการ rollback
        if anomaly.severity >= 80 {
            self.status = RecoveryStatus::PendingRollback;
            
            println!("🚨 CRITICAL ANOMALY DETECTED:");
            println!("   Type: {:?}", anomaly.anomaly_type);
            println!("   Height: {}", anomaly.height);
            println!("   Severity: {}/100", anomaly.severity);
            println!("   Description: {}", anomaly.description);
            println!("   Recommendation: {}", anomaly.recommendation);

            // เริ่มนับถอยหลังสำหรับ auto-rollback
            self.schedule_auto_rollback(&anomaly)?;
        } else {
            self.status = RecoveryStatus::AnomalyDetected;
            println!("⚠️  Anomaly detected - monitoring closely");
        }

        Ok(())
    }

    /// กำหนดเวลา auto-rollback
    fn schedule_auto_rollback(&mut self, anomaly: &AnomalyReport) -> Result<(), AutoRecoveryError> {
        println!("⏰ Scheduling auto-rollback in {} seconds...", self.config.rollback_delay);
        println!("   Operators can override with manual intervention");
        
        // ในระบบจริงจะใช้ scheduler หรือ background thread
        // ตอนนี้จำลองว่า rollback เกิดขึ้นทันที
        self.execute_auto_rollback(anomaly)
    }

    /// ดำเนินการ auto-rollback
    fn execute_auto_rollback(&mut self, anomaly: &AnomalyReport) -> Result<(), AutoRecoveryError> {
        self.status = RecoveryStatus::RollingBack;

        println!("🔄 EXECUTING AUTO-ROLLBACK:");
        println!("   From height: {}", anomaly.height);
        println!("   To height: {}", self.last_safe_height);
        println!("   Reason: {}", anomaly.description);

        // ลบ snapshots ที่มากกว่า safe height
        self.snapshots.split_off(&(self.last_safe_height + 1));

        // ส่ง alert ให้ทุกคนรู้
        self.send_recovery_alert(anomaly)?;

        self.status = RecoveryStatus::Recovered;
        println!("✅ AUTO-RECOVERY COMPLETED:");
        println!("   Network is now at safe height: {}", self.last_safe_height);
        println!("   Safe hash: {:02x}{:02x}...{:02x}{:02x}", 
                 self.last_safe_hash[0], self.last_safe_hash[1],
                 self.last_safe_hash[30], self.last_safe_hash[31]);

        Ok(())
    }

    /// Manual override สำหรับยกเลิก auto-rollback
    pub fn manual_override(
        &mut self,
        operator_id: &str,
        signature: &str,
        reason: &str,
    ) -> Result<(), AutoRecoveryError> {
        // ตรวจสอบว่า operator ได้รับอนุญาต
        if !self.authorized_operators.contains(&operator_id.to_string()) {
            return Err(AutoRecoveryError::Unauthorized {
                operator: operator_id.to_string(),
            });
        }

        // บันทึก signature
        self.override_signatures.insert(operator_id.to_string(), signature.to_string());

        println!("🔐 Manual override received from: {}", operator_id);
        println!("   Reason: {}", reason);
        println!("   Signatures: {}/{}", 
                 self.override_signatures.len(), 
                 self.config.override_signatures);

        // ถ้ามี signatures ครบ
        if self.override_signatures.len() >= self.config.override_signatures as usize {
            self.status = RecoveryStatus::ManualIntervention;
            println!("✅ MANUAL OVERRIDE ACTIVATED:");
            println!("   Auto-rollback cancelled");
            println!("   Manual investigation required");
            
            // ส่ง alert
            self.send_override_alert(operator_id, reason)?;
        }

        Ok(())
    }

    /// ดำเนินการ manual rollback
    pub fn manual_rollback(
        &mut self,
        operator_id: &str,
        target_height: u64,
        reason: &str,
    ) -> Result<(), AutoRecoveryError> {
        if !self.authorized_operators.contains(&operator_id.to_string()) {
            return Err(AutoRecoveryError::Unauthorized {
                operator: operator_id.to_string(),
            });
        }

        // ตรวจสอบว่ามี snapshot ที่ target height
        if !self.snapshots.contains_key(&target_height) {
            return Err(AutoRecoveryError::TargetNotFound { height: target_height });
        }

        println!("🔧 MANUAL ROLLBACK INITIATED:");
        println!("   Operator: {}", operator_id);
        println!("   Target height: {}", target_height);
        println!("   Reason: {}", reason);

        // ดำเนินการ rollback
        self.snapshots.split_off(&(target_height + 1));
        self.last_safe_height = target_height;
        
        if let Some(snapshot) = self.snapshots.get(&target_height) {
            self.last_safe_hash = snapshot.hash;
        }

        self.status = RecoveryStatus::Recovered;
        println!("✅ MANUAL ROLLBACK COMPLETED:");
        println!("   Network is now at height: {}", target_height);

        Ok(())
    }

    /// ลบ snapshots เก่า
    fn cleanup_old_snapshots(&mut self) {
        if self.snapshots.len() > self.config.memory_blocks {
            let mut heights: Vec<u64> = self.snapshots.keys().cloned().collect();
            heights.sort();
            
            let to_remove = heights.len() - self.config.memory_blocks;
            for i in 0..to_remove {
                if let Some(height) = heights.get(i) {
                    self.snapshots.remove(height);
                }
            }
        }
    }

    /// ส่ง recovery alert
    fn send_recovery_alert(&self, anomaly: &AnomalyReport) -> Result<(), AutoRecoveryError> {
        println!("📢 SENDING RECOVERY ALERT:");
        println!("   🚨 CRITICAL: Auto-recovery initiated");
        println!("   📍 Height: {}", anomaly.height);
        println!("   🔍 Type: {:?}", anomaly.anomaly_type);
        println!("   📝 Description: {}", anomaly.description);
        println!("   🔄 Rollback to: {}", self.last_safe_height);
        println!("   ⏰ Time: {}", chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"));
        
        Ok(())
    }

    /// ส่ง override alert
    fn send_override_alert(&self, operator: &str, reason: &str) -> Result<(), AutoRecoveryError> {
        println!("📢 SENDING OVERRIDE ALERT:");
        println!("   🔐 MANUAL OVERRIDE ACTIVATED");
        println!("   👤 Operator: {}", operator);
        println!("   📝 Reason: {}", reason);
        println!("   ⏰ Time: {}", chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"));
        
        Ok(())
    }

    /// ดูสถานะปัจจุบัน
    pub fn get_status(&self) -> RecoveryStatus {
        self.status.clone()
    }

    /// ดูข้อมูลสถิติ
    pub fn get_statistics(&self) -> RecoveryStatistics {
        RecoveryStatistics {
            total_snapshots: self.snapshots.len(),
            total_anomalies: self.anomalies.len(),
            last_safe_height: self.last_safe_height,
            last_safe_hash: self.last_safe_hash,
            current_status: self.status.clone(),
            last_anomaly_time: self.last_anomaly_time,
            memory_usage: self.estimate_memory_usage(),
        }
    }

    /// ประเมินการใช้หน่วยความจำ
    fn estimate_memory_usage(&self) -> usize {
        // ประมาณการใช้หน่วยความจำ (bytes)
        let snapshot_size = std::mem::size_of::<BlockSnapshot>();
        let anomaly_size = std::mem::size_of::<AnomalyReport>();
        
        self.snapshots.len() * snapshot_size + 
        self.anomalies.len() * anomaly_size +
        1024 // overhead
    }

    /// ดูรายการ anomalies
    pub fn get_anomalies(&self) -> &[AnomalyReport] {
        &self.anomalies
    }

    /// ดู snapshots ล่าสุด
    pub fn get_recent_snapshots(&self, count: usize) -> Vec<&BlockSnapshot> {
        self.snapshots
            .values()
            .rev()
            .take(count)
            .collect()
    }
}

/// สถิติการกู้คืน
#[derive(Debug, Clone)]
pub struct RecoveryStatistics {
    /// จำนวน snapshots ทั้งหมด
    pub total_snapshots: usize,
    /// จำนวน anomalies ทั้งหมด
    pub total_anomalies: usize,
    /// Height ล่าสุดที่ปลอดภัย
    pub last_safe_height: u64,
    /// Hash ล่าสุดที่ปลอดภัย
    pub last_safe_hash: [u8; 32],
    /// สถานะปัจจุบัน
    pub current_status: RecoveryStatus,
    /// เวลาที่ตรวจพบ anomaly ล่าสุด
    pub last_anomaly_time: u64,
    /// การใช้หน่วยความจำ (bytes)
    pub memory_usage: usize,
}

/// Errors สำหรับ auto-recovery
#[derive(Debug, Error)]
pub enum AutoRecoveryError {
    /// Operator ไม่ได้รับอนุญาต
    #[error("operator '{operator}' is not authorized")]
    Unauthorized { operator: String },
    
    /// ไม่พบ target height
    #[error("target height {height} not found in snapshots")]
    TargetNotFound { height: u64 },
    
    /// ข้อผิดพลาดในการตรวจสอบ
    #[error("validation error: {reason}")]
    ValidationError { reason: String },
    
    /// ข้อผิดพลาดในการ rollback
    #[error("rollback failed: {reason}")]
    RollbackError { reason: String },
}

/// ฟังก์ชันช่วยเหลือ
fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_snapshot(height: u64) -> BlockSnapshot {
        BlockSnapshot {
            height,
            hash: [height as u8; 32],
            timestamp: current_timestamp(),
            parent_hash: [0u8; 32],
            merkle_root: [0u8; 32],
            difficulty: 0x1d00ffff,
            nonce: height,
            tx_count: 100,
            size: 1_000_000,
            metrics: BlockMetrics {
                transaction_count: 100,
                avg_size_100: 1_000_000,
                signature_count: 100,
                gas_used: 21000,
                orphan_rate: 0.01,
                avg_difficulty: 1000.0,
            },
            recorded_at: current_timestamp(),
        }
    }

    #[test]
    fn test_normal_block_recording() {
        let config = AutoRecoveryConfig::default();
        let mut manager = AutoRecoveryManager::new(config);
        
        let snapshot = create_test_snapshot(1000);
        let result = manager.record_block(snapshot);
        
        assert!(result.is_ok());
        assert_eq!(manager.get_status(), RecoveryStatus::Normal);
        assert_eq!(manager.get_statistics().last_safe_height, 1000);
    }

    #[test]
    fn test_anomaly_detection() {
        let config = AutoRecoveryConfig::default();
        let mut manager = AutoRecoveryManager::new(config);
        
        // สร้าง block ที่มีปัญหา (size ใหญ่เกินไป)
        let mut snapshot = create_test_snapshot(1000);
        snapshot.size = 50_000_000; // 50MB - ผิดปกติ
        
        let result = manager.record_block(snapshot);
        
        assert!(result.is_ok());
        assert!(manager.get_status() != RecoveryStatus::Normal);
        assert!(!manager.get_anomalies().is_empty());
    }

    #[test]
    fn test_manual_override() {
        let config = AutoRecoveryConfig::default();
        let mut manager = AutoRecoveryManager::new(config);
        manager.set_authorized_operators(vec!["operator1".to_string()]);
        
        // บันทึก blocks ปกติก่อน
        for i in 990..=999 {
            let snapshot = create_test_snapshot(i);
            manager.record_block(snapshot).unwrap();
        }
        
        // สร้างปัญหาเพื่อ trigger rollback
        let mut snapshot = create_test_snapshot(1000);
        snapshot.size = 50_000_000;
        manager.record_block(snapshot).unwrap();
        
        // หลังจาก auto-rollback, ทำ manual rollback ไปที่ 995
        let result = manager.manual_rollback("operator1", 995, "Manual rollback test");
        
        assert!(result.is_ok());
        assert_eq!(manager.get_statistics().last_safe_height, 995);
    }

    #[test]
    fn test_unauthorized_override() {
        let config = AutoRecoveryConfig::default();
        let mut manager = AutoRecoveryManager::new(config);
        
        let result = manager.manual_override("hacker", "fake_sig", "Malicious override");
        
        assert!(result.is_err());
        match result.unwrap_err() {
            AutoRecoveryError::Unauthorized { operator } => {
                assert_eq!(operator, "hacker");
            }
            _ => panic!("Expected Unauthorized error"),
        }
    }

    #[test]
    fn test_manual_rollback() {
        let config = AutoRecoveryConfig::default();
        let mut manager = AutoRecoveryManager::new(config);
        manager.set_authorized_operators(vec!["operator1".to_string()]);
        
        // บันทึก blocks หลายๆ block
        for i in 1000..=1010 {
            let snapshot = create_test_snapshot(i);
            manager.record_block(snapshot).unwrap();
        }
        
        // rollback ไปยัง 1005
        let result = manager.manual_rollback("operator1", 1005, "Test rollback");
        
        assert!(result.is_ok());
        assert_eq!(manager.get_statistics().last_safe_height, 1005);
    }
}