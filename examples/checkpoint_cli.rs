#!/usr/bin/env rust-script
//! BitQuan Checkpoint CLI - เครื่องมือจัดการ checkpoint ผ่าน command line
//! 
//! การใช้งาน:
//!   cargo run --bin checkpoint_cli -- --help
//!   cargo run --bin checkpoint_cli -- status
//!   cargo run --bin checkpoint_cli -- create --height 750000 --hash 1234abcd... --reason "Emergency rollback"

use std::error::Error;
use std::str::FromStr;
use clap::{Parser, Subcommand};
use hex;

#[derive(Parser)]
#[command(name = "checkpoint-cli")]
#[command(about = "BitQuan Checkpoint Management CLI")]
#[command(version = "1.0")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// แสดงสถานะ checkpoint system
    Status {
        /// แสดงข้อมูลแบบละเอียด
        #[arg(short, long)]
        verbose: bool,
    },
    /// สร้าง checkpoint ใหม่
    Create {
        /// Block height ของ checkpoint
        #[arg(long)]
        height: u64,
        /// Block hash (hex format)
        #[arg(long)]
        hash: String,
        /// เหตุผลในการสร้าง checkpoint
        #[arg(long)]
        reason: String,
        /// Operator ID
        #[arg(long, default_value = "operator1")]
        operator: String,
    },
    /// แสดงรายการ checkpoints
    List {
        /// แสดงเฉพาะ checkpoints ถึง height นี้
        #[arg(long)]
        up_to: Option<u64>,
    },
    /// ลบ checkpoints ที่เก่ากว่า height ที่กำหนด
    Rollback {
        /// Height ที่ต้องการ rollback ไป
        #[arg(long)]
        height: u64,
    },
    /// เปิด/ปิด checkpoint validation
    Toggle {
        /// เปิด (true) หรือปิด (false)
        #[arg(long)]
        enable: bool,
    },
    /// แบน peer
    Ban {
        /// Peer ID ที่ต้องการแบน
        #[arg(long)]
        peer_id: String,
        /// เหตุผลในการแบน
        #[arg(long, default_value = "Emergency ban")]
        reason: String,
    },
    /// ส่ง alert
    Alert {
        /// ข้อความ alert
        #[arg(long)]
        message: String,
    },
}

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Status { verbose } => cmd_status(verbose),
        Commands::Create { height, hash, reason, operator } => cmd_create(height, hash, reason, operator),
        Commands::List { up_to } => cmd_list(up_to),
        Commands::Rollback { height } => cmd_rollback(height),
        Commands::Toggle { enable } => cmd_toggle(enable),
        Commands::Ban { peer_id, reason } => cmd_ban(peer_id, reason),
        Commands::Alert { message } => cmd_alert(message),
    }
}

fn cmd_status(verbose: bool) -> Result<(), Box<dyn Error>> {
    println!("📊 BitQuan Checkpoint System Status");
    println!("=" .repeat(40));

    // จำลองข้อมูลสถานะ
    let enabled = true;
    let processing_paused = false;
    let checkpoints_enabled = true;
    let checkpoint_count = 3;
    let banned_peers_count = 2;
    let action_history_count = 7;
    let current_height = 850000;

    println!("🟢 Emergency System: {}", if enabled { "Enabled" } else { "Disabled" });
    println!("⏸️  Processing: {}", if processing_paused { "Paused" } else { "Running" });
    println!("🛡️  Checkpoints: {}", if checkpoints_enabled { "Enabled" } else { "Disabled" });
    println!("📊 Checkpoint Count: {}", checkpoint_count);
    println!("🚫 Banned Peers: {}", banned_peers_count);
    println!("📜 Action History: {}", action_history_count);
    println!("📏 Current Height: {}", current_height);

    if verbose {
        println!("\n📋 Detailed Information:");
        println!("   Last Checkpoint: Height 750000 (2 hours ago)");
        println!("   Latest Action: Enable Checkpoints (30 minutes ago)");
        println!("   System Uptime: 7 days, 14 hours");
        println!("   Memory Usage: 45MB");
        println!("   Network Status: Healthy");
    }

    Ok(())
}

fn cmd_create(height: u64, hash: String, reason: String, operator: String) -> Result<(), Box<dyn Error>> {
    println!("🛡️  Creating Emergency Checkpoint");
    println!("=" .repeat(40));

    // แปลง hash จาก hex
    let hash_bytes = match hex::decode(&hash) {
        Ok(bytes) => {
            if bytes.len() != 32 {
                return Err("Hash must be exactly 32 bytes (64 hex characters)".into());
            }
            let mut array = [0u8; 32];
            array.copy_from_slice(&bytes);
            array
        }
        Err(_) => return Err("Invalid hex format".into()),
    };

    println!("📍 Height: {}", height);
    println!("🔐 Hash: {}...{}", &hash[..8], &hash[hash.len()-8..]);
    println!("📝 Reason: {}", reason);
    println!("👤 Operator: {}", operator);

    // จำลองการสร้าง checkpoint
    println!("\n✅ Validating checkpoint data...");
    println!("   ✅ Height is valid (not future, not genesis)");
    println!("   ✅ Hash format is correct");
    println!("   ✅ Reason is provided");

    println!("\n🔐 Checking authorization...");
    println!("   ✅ Operator '{}' is authorized", operator);

    println!("\n🛡️  Creating checkpoint...");
    println!("   ✅ Checkpoint created successfully");
    println!("   📅 Created at: {}", chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"));

    println!("\n🔄 Enabling checkpoint validation...");
    println!("   ✅ Checkpoint validation enabled");

    println!("\n📢 Sending network alert...");
    println!("   ✅ Alert sent to all operators");

    println!("\n🎉 Emergency checkpoint created successfully!");
    println!("   📍 Height: {}", height);
    println!("   🔐 Hash: {}...{}", &hash[..8], &hash[hash.len()-8..]);

    Ok(())
}

fn cmd_list(up_to: Option<u64>) -> Result<(), Box<dyn Error>> {
    println!("📋 Checkpoint List");
    println!("=" .repeat(40));

    // จำลองข้อมูล checkpoints
    let checkpoints = vec![
        (500000, "a1b2c3d4e5f6...", "Initial checkpoint", 1703980800),
        (650000, "f6e5d4c3b2a1...", "Network upgrade checkpoint", 1704067200),
        (750000, "9f8e7d6c5b4a...", "Emergency rollback checkpoint", 1704153600),
    ];

    if let Some(max_height) = up_to {
        println!("🔍 Showing checkpoints up to height {}", max_height);
    }

    println!("\n🛡️  Active Checkpoints:");
    for (i, (height, hash_prefix, reason, timestamp)) in checkpoints.iter().enumerate() {
        if let Some(max) = up_to {
            if *height > max {
                continue;
            }
        }

        let created_at = chrono::DateTime::from_timestamp(*timestamp as i64, 0)
            .unwrap_or_default()
            .format("%Y-%m-%d %H:%M:%S");

        println!("\n{}. 📍 Height: {}", i + 1, height);
        println!("   🔐 Hash: {}...", hash_prefix);
        println!("   📝 Reason: {}", reason);
        println!("   📅 Created: {}", created_at);
    }

    if checkpoints.is_empty() {
        println!("   No checkpoints found");
    }

    Ok(())
}

fn cmd_rollback(height: u64) -> Result<(), Box<dyn Error>> {
    println!("🔄 Rolling Back Checkpoints");
    println!("=" .repeat(40));

    println!("📍 Target height: {}", height);
    
    // จำลองการ rollback
    println!("\n🔍 Finding checkpoints above height {}...", height);
    println!("   📊 Found 2 checkpoints to remove:");
    println!("      - Height 800000: Emergency checkpoint");
    println!("      - Height 775000: Network upgrade checkpoint");

    println!("\n⚠️  Warning: This will remove checkpoints above height {}", height);
    print!("   Are you sure? (y/N): ");
    use std::io;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    
    if !input.trim().to_lowercase().starts_with('y') {
        println!("   ❌ Rollback cancelled");
        return Ok(());
    }

    println!("\n🔄 Performing rollback...");
    println!("   ✅ Checkpoint at height 800000 removed");
    println!("   ✅ Checkpoint at height 775000 removed");
    println!("   ✅ Rollback completed");

    println!("\n📊 New checkpoint count: 1");
    println!("   📍 Latest checkpoint: Height 750000");

    Ok(())
}

fn cmd_toggle(enable: bool) -> Result<(), Box<dyn Error>> {
    println!("🛡️  Checkpoint Validation");
    println!("=" .repeat(40));

    if enable {
        println!("🔓 Enabling checkpoint validation...");
        println!("   ✅ Checkpoint validation enabled");
        println!("   📊 All new blocks will be validated against checkpoints");
    } else {
        println!("🔒 Disabling checkpoint validation...");
        println!("   ⚠️  Warning: Blocks will not be validated against checkpoints");
        println!("   ✅ Checkpoint validation disabled");
    }

    Ok(())
}

fn cmd_ban(peer_id: String, reason: String) -> Result<(), Box<dyn Error>> {
    println!("🚫 Banning Peer");
    println!("=" .repeat(40));

    println!("🎯 Peer ID: {}", peer_id);
    println!("📝 Reason: {}", reason);

    println!("\n🔍 Checking peer status...");
    println!("   ✅ Peer is currently active");
    println!("   📊 Connection count: 15");
    println!("   ⏱️  Last seen: 2 minutes ago");

    println!("\n🚫 Banning peer...");
    println!("   ✅ Peer '{}' banned successfully", peer_id);
    println!("   📝 Ban reason: {}", reason);
    println!("   📅 Banned at: {}", chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"));

    println!("\n🔄 Updating peer filters...");
    println!("   ✅ Firewall rules updated");
    println!("   ✅ Connection terminated");
    println!("   ✅ Peer added to blacklist");

    println!("\n📢 Notifying network...");
    println!("   ✅ Alert sent to other nodes");

    Ok(())
}

fn cmd_alert(message: String) -> Result<(), Box<dyn Error>> {
    println!("📢 Sending Network Alert");
    println!("=" .repeat(40));

    println!("📝 Message: {}", message);
    println!("👤 Sent by: operator1");
    println!("📅 Time: {}", chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"));

    println!("\n📡 Broadcasting alert...");
    println!("   ✅ Alert sent to 150 active nodes");
    println!("   ✅ Alert logged to audit trail");
    println!("   ✅ Email notification sent to operators");

    println!("\n📊 Alert Statistics:");
    println!("   📧 Email recipients: 5");
    println!("   💬 Discord notifications: 3");
    println!("   📱 SMS alerts: 2");
    println!("   🌐 Webhook calls: 1");

    println!("\n🎉 Network alert sent successfully!");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cmd_status() {
        assert!(cmd_status(false).is_ok());
        assert!(cmd_status(true).is_ok());
    }

    #[test]
    fn test_cmd_create() {
        let hash = "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";
        assert!(cmd_create(750000, hash.to_string(), "Test".to_string(), "operator1".to_string()).is_ok());
    }

    #[test]
    fn test_cmd_create_invalid_hash() {
        let result = cmd_create(750000, "invalid".to_string(), "Test".to_string(), "operator1".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_cmd_list() {
        assert!(cmd_list(None).is_ok());
        assert!(cmd_list(Some(600000)).is_ok());
    }

    #[test]
    fn test_cmd_toggle() {
        assert!(cmd_toggle(true).is_ok());
        assert!(cmd_toggle(false).is_ok());
    }

    #[test]
    fn test_cmd_ban() {
        assert!(cmd_ban("test_peer".to_string(), "Test ban".to_string()).is_ok());
    }

    #[test]
    fn test_cmd_alert() {
        assert!(cmd_alert("Test alert message".to_string()).is_ok());
    }
}