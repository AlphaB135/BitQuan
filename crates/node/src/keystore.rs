//! Thin wrapper around the wallet keystore primitives from `bq-crypto`.

use std::path::Path;

use bq_crypto::wallet::{
    Keystore as CryptoKeystore, KeystoreError, SecurePrivateKey, SecureString,
};

/// Alias used by legacy code paths.
pub type KeystoreFile = CryptoKeystore;

/// Encrypts JSON key material with the provided password and address metadata.
pub fn encrypt_keypair(
    keypair_json: &str,
    password: &str,
    address: &str,
) -> Result<KeystoreFile, KeystoreError> {
    let secure_key = SecurePrivateKey::new(keypair_json.as_bytes().to_vec());
    let password = SecureString::new(password.to_owned());
    CryptoKeystore::new(&secure_key, &password, address.to_owned())
}

/// Decrypts the keypair JSON from the keystore using the provided password.
pub fn decrypt_keypair(keystore: &KeystoreFile, password: &str) -> Result<String, KeystoreError> {
    let password = SecureString::new(password.to_owned());
    let decrypted = keystore.unlock(&password)?;
    let json = String::from_utf8(decrypted.as_slice().to_vec())
        .map_err(|_| KeystoreError::InvalidPassword)?;
    Ok(json)
}

/// Saves the keystore to disk.
pub fn save_keystore(keystore: &KeystoreFile, path: &Path) -> Result<(), KeystoreError> {
    keystore.save_to_file(path)
}

/// Loads a keystore from disk.
pub fn load_keystore(path: &Path) -> Result<KeystoreFile, KeystoreError> {
    CryptoKeystore::load_from_file(path)
}
