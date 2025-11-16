//! Integration tests for wallet password rotation.

use std::fs;
use tempfile::tempdir;
use wallet::keystore::{decrypt_keystore, encrypt_keystore, rotate_keystore, KeystoreFile};

#[test]
fn test_password_rotation_roundtrip() {
    let old_password = "old_secure_pass";
    let new_password = "new_secure_pass";
    let plaintext = b"sensitive wallet data";

    // Create keystore with old password (using light parameters for faster test)
    let keystore = encrypt_keystore(plaintext, old_password, None, 8192, 1, 1);

    // Verify old password works
    let decrypted = decrypt_keystore(&keystore, old_password)
        .expect("decrypt with old password should succeed");
    assert_eq!(decrypted, plaintext);

    // Rotate to new password
    let rotated = rotate_keystore(&keystore, old_password, new_password, 8192, 1, 1)
        .expect("password rotation should succeed");

    // Old password should no longer work
    let old_result = decrypt_keystore(&rotated, old_password);
    assert!(
        old_result.is_err(),
        "old password should fail after rotation"
    );

    // New password should work
    let decrypted_new =
        decrypt_keystore(&rotated, new_password).expect("decrypt with new password should succeed");
    assert_eq!(decrypted_new, plaintext);
}

#[test]
fn test_password_rotation_persists() {
    let dir = tempdir().expect("temp dir");
    let path = dir.path().join("keystore.json");

    let old_pw = "password1";
    let new_pw = "password2";
    let data = b"wallet secret key";

    // Create and save keystore
    let keystore = encrypt_keystore(data, old_pw, None, 8192, 1, 1);
    let json = keystore.to_json().expect("serialize");
    fs::write(&path, &json).expect("write file");

    // Load and rotate
    let loaded_json = fs::read_to_string(&path).expect("read file");
    let loaded: KeystoreFile = serde_json::from_str(&loaded_json).expect("deserialize");
    let rotated = rotate_keystore(&loaded, old_pw, new_pw, 8192, 1, 1).expect("rotate");

    // Save rotated version
    let rotated_json = rotated.to_json().expect("serialize rotated");
    fs::write(&path, &rotated_json).expect("write rotated");

    // Reload and verify
    let final_json = fs::read_to_string(&path).expect("read final");
    let final_ks: KeystoreFile = serde_json::from_str(&final_json).expect("deserialize final");
    let decrypted = decrypt_keystore(&final_ks, new_pw).expect("decrypt final");
    assert_eq!(decrypted, data);
}

#[test]
fn test_password_rotation_wrong_old_password() {
    let old_password = "correct_old";
    let wrong_password = "wrong_old";
    let new_password = "new_pass";
    let data = b"test data";

    let keystore = encrypt_keystore(data, old_password, None, 8192, 1, 1);

    // Attempt rotation with wrong old password should fail
    let result = rotate_keystore(&keystore, wrong_password, new_password, 8192, 1, 1);
    assert!(
        result.is_err(),
        "rotation with wrong old password should fail"
    );
}

#[test]
fn test_multiple_password_rotations() {
    let data = b"multi-rotation test";
    let passwords = ["pass1", "pass2", "pass3", "pass4"];

    let mut keystore = encrypt_keystore(data, passwords[0], None, 8192, 1, 1);

    // Rotate through multiple passwords
    for i in 0..passwords.len() - 1 {
        keystore = rotate_keystore(&keystore, passwords[i], passwords[i + 1], 8192, 1, 1)
            .expect("Failed to rotate keystore");

        // Verify new password works
        let decrypted = decrypt_keystore(&keystore, passwords[i + 1])
            .expect("Failed to decrypt with new password");
        assert_eq!(decrypted, data);

        // Verify old password no longer works
        let old_result = decrypt_keystore(&keystore, passwords[i]);
        assert!(old_result.is_err(), "old password {} should not work", i);
    }
}
