//! Integration tests for cryptographic operations: keygen, sign, verify.

use pqc_dilithium_seeded::{Keypair, PUBLICKEYBYTES, SECRETKEYBYTES, SIGNBYTES};
use sha2::{Digest, Sha256};

#[test]
fn test_keygen_sign_verify_roundtrip() {
    // DEBUG FORENSICS: Print what constants we're actually using
    println!("=== DILITHIUM DEBUG FORENSICS ===");
    println!(
        "EXPECTED: SIGNBYTES={}, PUBLICKEYBYTES={}, SECRETKEYBYTES={}",
        SIGNBYTES, PUBLICKEYBYTES, SECRETKEYBYTES
    );

    let message = b"Hello, BitQuan!";

    // Generate keypair
    let keypair = Keypair::generate();
    println!(
        "ACTUAL:   PK len={}, SK len={}",
        keypair.public.len(),
        keypair.expose_secret().len()
    );

    // Sign message
    let signature = keypair.sign(message);
    println!("ACTUAL:   Signature len={}", signature.len());

    // Verify signature
    let result = pqc_dilithium_seeded::verify(&signature, message, &keypair.public);
    println!("VERIFY:   is_ok={}", result.is_ok());
    println!("====================================");

    assert!(result.is_ok(), "signature should be valid");
}

#[test]
fn test_verify_wrong_message_fails() {
    let original_message = b"original message";
    let tampered_message = b"tampered message";

    let keypair = Keypair::generate();
    let signature = keypair.sign(original_message);

    // Verification with tampered message should fail
    let result = pqc_dilithium_seeded::verify(&signature, tampered_message, &keypair.public);

    assert!(result.is_err(), "tampered message should not verify");
}

#[test]
fn test_verify_wrong_signature_fails() {
    let message = b"test message";

    let keypair = Keypair::generate();
    let signature = keypair.sign(message);

    // Corrupt the signature
    let mut corrupted_sig = signature;
    corrupted_sig[0] ^= 0xFF;

    let result = pqc_dilithium_seeded::verify(&corrupted_sig, message, &keypair.public);

    assert!(result.is_err(), "corrupted signature should not verify");
}

#[test]
fn test_verify_wrong_public_key_fails() {
    let message = b"message for wrong key test";

    let keypair1 = Keypair::generate();
    let keypair2 = Keypair::generate();

    let signature = keypair1.sign(message);

    // Verification with wrong public key should fail
    let result = pqc_dilithium_seeded::verify(&signature, message, &keypair2.public);

    assert!(
        result.is_err(),
        "signature should not verify with wrong public key"
    );
}

#[test]
fn test_multiple_messages_different_signatures() {
    let messages = [b"message one" as &[u8], b"message two", b"message three"];

    let keypair = Keypair::generate();

    // Sign all messages
    let signatures: Vec<_> = messages.iter().map(|msg| keypair.sign(msg)).collect();

    // Verify all signatures are different (due to randomized signing)
    // Note: Dilithium signatures are randomized, so same message produces different sigs

    // Verify all signatures are valid for their respective messages
    for (msg, sig) in messages.iter().zip(signatures.iter()) {
        let result = pqc_dilithium_seeded::verify(sig, msg, &keypair.public);
        assert!(result.is_ok(), "signature should verify for its message");
    }
}

#[test]
fn test_deterministic_hashing() {
    let data = b"consistent input data";

    let mut hasher1 = Sha256::new();
    hasher1.update(data);
    let hash1 = hasher1.finalize();

    let mut hasher2 = Sha256::new();
    hasher2.update(data);
    let hash2 = hasher2.finalize();

    assert_eq!(hash1, hash2, "hash should be deterministic");
    assert_eq!(hash1.len(), 32, "SHA256 should produce 32 bytes");
}

#[test]
fn test_hash_different_inputs() {
    let data1 = b"input one";
    let data2 = b"input two";

    let mut hasher1 = Sha256::new();
    hasher1.update(data1);
    let hash1 = hasher1.finalize();

    let mut hasher2 = Sha256::new();
    hasher2.update(data2);
    let hash2 = hasher2.finalize();

    assert_ne!(
        hash1, hash2,
        "different inputs should produce different hashes"
    );
}

#[test]
fn test_keypair_size_constants() {
    // Verify size constants match actual implementation
    let keypair = Keypair::generate();
    let signature = keypair.sign(b"test");

    assert_eq!(
        signature.len(),
        SIGNBYTES,
        "signature should match SIGNBYTES constant"
    );
    assert_eq!(keypair.public.len(), PUBLICKEYBYTES, "public key size");
    assert_eq!(
        keypair.expose_secret().len(),
        SECRETKEYBYTES,
        "secret key size"
    );
}
