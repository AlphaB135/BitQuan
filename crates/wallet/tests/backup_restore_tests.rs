//! Integration tests for wallet backup and restore functionality.

use tempfile::tempdir;
use wallet::backup::{Network, WalletBackup};

#[test]
fn test_backup_and_restore_roundtrip() {
    let password = "secure_password_123";
    let wallet_data = b"test wallet data with keys and addresses";

    // Create backup
    let backup = WalletBackup::create(wallet_data, password, Network::Mainnet, None)
        .expect("backup creation should succeed");

    // Restore from backup
    let restored = backup.restore(password).expect("restore should succeed");

    assert_eq!(restored, wallet_data);
}

#[test]
fn test_backup_save_and_load() {
    let dir = tempdir().expect("temp dir");
    let path = dir.path().join("wallet.backup");

    let password = "test_password";
    let wallet_data = b"test wallet keystore data";

    // Create and save backup
    let backup = WalletBackup::create(wallet_data, password, Network::Devnet, None)
        .expect("backup creation");
    backup.save(&path).expect("save should succeed");

    // Load backup
    let loaded = WalletBackup::load(&path).expect("load should succeed");
    let restored = loaded.restore(password).expect("restore should succeed");

    assert_eq!(restored, wallet_data);
}

#[test]
fn test_backup_wrong_password_fails() {
    let password = "correct_password";
    let wrong_password = "wrong_password";

    let wallet_data = b"sensitive wallet data";

    let backup = WalletBackup::create(wallet_data, password, Network::Mainnet, None)
        .expect("backup creation");

    // Should fail with wrong password
    let result = backup.restore(wrong_password);
    assert!(result.is_err(), "restore with wrong password should fail");
}

#[test]
fn test_backup_network_preservation() {
    let password = "pass123";
    let wallet_data = b"network test wallet data";

    // Test mainnet
    let backup_main = WalletBackup::create(wallet_data, password, Network::Mainnet, None)
        .expect("mainnet backup");
    assert_eq!(backup_main.network, Network::Mainnet);
    let restored_main = backup_main.restore(password).expect("restore mainnet");
    assert_eq!(restored_main, wallet_data);

    // Test devnet
    let backup_dev =
        WalletBackup::create(wallet_data, password, Network::Devnet, None).expect("devnet backup");
    assert_eq!(backup_dev.network, Network::Devnet);
    let restored_dev = backup_dev.restore(password).expect("restore devnet");
    assert_eq!(restored_dev, wallet_data);
}
