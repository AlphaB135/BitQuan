// crates/wallet/src/keystore.rs
// Requires deps: argon2, aes-gcm, rand, base64, serde, serde_json, zeroize, secrecy
use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use argon2::{Argon2, Params};
use base64::{engine::general_purpose, Engine as _};
use rand::rngs::OsRng;
use rand::RngCore;
use secrecy::{ExposeSecret, SecretVec};
use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use zeroize::Zeroize;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct KdfParams {
    pub mem_kib: u32,
    pub time_cost: u32,
    pub parallelism: u8,
    pub salt_b64: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct KeystoreFile {
    pub magic: String,
    pub version: u8,
    pub created: u64,
    pub kdf: KdfParams,
    pub nonce_b64: String,
    pub ciphertext_b64: String,
    pub meta: Option<serde_json::Value>,
}

impl KeystoreFile {
    pub fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string_pretty(self)
    }
}

pub const MAGIC: &str = "BQK1";
pub const CURRENT_VERSION: u8 = 1;

pub const DEFAULT_MEM_KIB: u32 = 65536;
pub const DEFAULT_TIME_COST: u32 = 3;
pub const DEFAULT_PARALLELISM: u8 = 1;
pub const SALT_LEN: usize = 16;
pub const NONCE_LEN: usize = 12;

#[derive(Debug, Clone, Copy)]
pub enum KdfProfile {
    Tight,
    Medium,
    Light,
    Mobile,
}

impl KdfProfile {
    pub fn params(&self) -> (u32, u32, u8) {
        match self {
            KdfProfile::Tight => (65536, 3, 1),
            KdfProfile::Medium => (32768, 3, 1),
            KdfProfile::Light => (16384, 3, 1),
            KdfProfile::Mobile => (8192, 3, 1),
        }
    }
}

fn derive_key(
    password: &SecretVec<u8>,
    salt: &[u8],
    mem_kib: u32,
    time_cost: u32,
    parallelism: u8,
) -> [u8; 32] {
    let params = Params::new(mem_kib, time_cost, parallelism.into(), None).expect("argon params");
    let argon2 = Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params);

    let mut key = [0u8; 32];
    argon2
        .hash_password_into(password.expose_secret(), salt, &mut key)
        .expect("Argon2 derive failed");
    key
}

pub fn encrypt_keystore(
    plaintext: &[u8],
    password: &str,
    meta: Option<serde_json::Value>,
    mem_kib: u32,
    time_cost: u32,
    parallelism: u8,
) -> KeystoreFile {
    let pw = SecretVec::new(password.as_bytes().to_vec());

    let mut salt = vec![0u8; SALT_LEN];
    OsRng.fill_bytes(&mut salt);

    let mut nonce_bytes = vec![0u8; NONCE_LEN];
    OsRng.fill_bytes(&mut nonce_bytes);

    let mut key_bytes = derive_key(&pw, &salt, mem_kib, time_cost, parallelism);

    #[allow(deprecated)]
    let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);
    #[allow(deprecated)]
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(
            nonce,
            Payload {
                msg: plaintext,
                aad: b"",
            },
        )
        .expect("encryption failure");

    key_bytes.zeroize();

    let created = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    KeystoreFile {
        magic: MAGIC.to_string(),
        version: CURRENT_VERSION,
        created,
        kdf: KdfParams {
            mem_kib,
            time_cost,
            parallelism,
            salt_b64: general_purpose::STANDARD.encode(&salt),
        },
        nonce_b64: general_purpose::STANDARD.encode(&nonce_bytes),
        ciphertext_b64: general_purpose::STANDARD.encode(&ciphertext),
        meta,
    }
}

pub fn decrypt_keystore(ks: &KeystoreFile, password: &str) -> Result<Vec<u8>, String> {
    if ks.magic != MAGIC {
        return Err(format!(
            "invalid magic: expected {}, got {}",
            MAGIC, ks.magic
        ));
    }
    if ks.version > CURRENT_VERSION {
        return Err(format!(
            "unsupported version: {} (max {})",
            ks.version, CURRENT_VERSION
        ));
    }

    let pw = SecretVec::new(password.as_bytes().to_vec());

    let salt = general_purpose::STANDARD
        .decode(&ks.kdf.salt_b64)
        .map_err(|e| format!("bad salt b64: {}", e))?;
    let nonce_bytes = general_purpose::STANDARD
        .decode(&ks.nonce_b64)
        .map_err(|e| format!("bad nonce b64: {}", e))?;
    let ciphertext = general_purpose::STANDARD
        .decode(&ks.ciphertext_b64)
        .map_err(|e| format!("bad cipher b64: {}", e))?;

    let mut key_bytes = derive_key(
        &pw,
        &salt,
        ks.kdf.mem_kib,
        ks.kdf.time_cost,
        ks.kdf.parallelism,
    );

    #[allow(deprecated)]
    let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);
    #[allow(deprecated)]
    let nonce = Nonce::from_slice(&nonce_bytes);

    let res = cipher
        .decrypt(
            nonce,
            Payload {
                msg: ciphertext.as_ref(),
                aad: b"",
            },
        )
        .map_err(|e| format!("decrypt failed: {:?}", e));

    key_bytes.zeroize();

    res
}

pub fn rotate_keystore(
    ks: &KeystoreFile,
    old_password: &str,
    new_password: &str,
    mem_kib: u32,
    time_cost: u32,
    parallelism: u8,
) -> Result<KeystoreFile, String> {
    let plaintext = decrypt_keystore(ks, old_password)?;
    let new_ks = encrypt_keystore(
        &plaintext,
        new_password,
        ks.meta.clone(),
        mem_kib,
        time_cost,
        parallelism,
    );
    Ok(new_ks)
}

pub fn write_keystore_file_atomic<P: AsRef<Path>>(
    path: P,
    ks: &KeystoreFile,
) -> std::io::Result<()> {
    let path = path.as_ref();
    let tmp_path = path.with_extension("tmp");
    let mut f = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&tmp_path)?;
    let json = ks.to_json().map_err(std::io::Error::other)?;
    f.write_all(json.as_bytes())?;
    f.flush()?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&tmp_path, std::fs::Permissions::from_mode(0o600))?;
    }

    #[cfg(windows)]
    {
        eprintln!(
            "WARNING: Windows file permissions not enforced. Use BitLocker/EFS to encrypt folder."
        );
    }

    std::fs::rename(tmp_path, path)?;
    Ok(())
}

pub fn read_keystore_file<P: AsRef<Path>>(path: P) -> std::io::Result<KeystoreFile> {
    let mut s = String::new();
    let mut f = File::open(path)?;
    f.read_to_string(&mut s)?;
    let ks: KeystoreFile = serde_json::from_str(&s)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    Ok(ks)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::tempdir;

    #[test]
    fn roundtrip_encrypt_decrypt() {
        let secret = b"this-is-my-private-key-bytes";
        let ks = encrypt_keystore(
            secret,
            "correct horse battery staple",
            Some(json!({"hint": "test"})),
            DEFAULT_MEM_KIB,
            DEFAULT_TIME_COST,
            DEFAULT_PARALLELISM,
        );
        let pt = decrypt_keystore(&ks, "correct horse battery staple").expect("should decrypt");
        assert_eq!(pt, secret);
    }

    #[test]
    fn wrong_password_fails() {
        let secret = b"abcdef012345";
        let ks = encrypt_keystore(
            secret,
            "pw1",
            None,
            DEFAULT_MEM_KIB,
            DEFAULT_TIME_COST,
            DEFAULT_PARALLELISM,
        );
        let res = decrypt_keystore(&ks, "pw2");
        assert!(res.is_err());
    }

    #[test]
    fn write_and_read_atomic_file() {
        let secret = b"file-secret";
        let ks = encrypt_keystore(
            secret,
            "pw",
            None,
            DEFAULT_MEM_KIB,
            DEFAULT_TIME_COST,
            DEFAULT_PARALLELISM,
        );
        let dir = tempdir().unwrap();
        let p = dir.path().join("keystore.json");
        write_keystore_file_atomic(&p, &ks).expect("write");
        let ks2 = read_keystore_file(&p).expect("read");
        let pt = decrypt_keystore(&ks2, "pw").expect("decrypt");
        assert_eq!(pt, secret);
    }

    #[test]
    fn tamper_cipher_rejected() {
        let secret = b"abc";
        let ks = encrypt_keystore(secret, "pw", None, 8 * 1024, 1, 1);
        let mut ks_bad = ks.clone();
        let mut c = general_purpose::STANDARD
            .decode(&ks_bad.ciphertext_b64)
            .unwrap();
        c[0] ^= 0xFF;
        ks_bad.ciphertext_b64 = general_purpose::STANDARD.encode(&c);
        assert!(decrypt_keystore(&ks_bad, "pw").is_err());
    }

    #[test]
    fn invalid_magic_rejected() {
        let secret = b"test";
        let mut ks = encrypt_keystore(secret, "pw", None, 8 * 1024, 1, 1);
        ks.magic = "FAKE".to_string();
        assert!(decrypt_keystore(&ks, "pw").is_err());
    }

    #[test]
    fn future_version_rejected() {
        let secret = b"test";
        let mut ks = encrypt_keystore(secret, "pw", None, 8 * 1024, 1, 1);
        ks.version = 99;
        let result = decrypt_keystore(&ks, "pw");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("unsupported version"));
    }

    #[test]
    fn large_secret_roundtrip() {
        let secret = vec![0x42u8; 32 * 1024];
        let ks = encrypt_keystore(&secret, "longpw", None, 8 * 1024, 1, 1);
        let pt = decrypt_keystore(&ks, "longpw").expect("decrypt");
        assert_eq!(pt, secret);
    }

    #[test]
    fn kdf_profile_params() {
        let (mem, time, par) = KdfProfile::Tight.params();
        assert_eq!(mem, 65536);
        assert_eq!(time, 3);
        assert_eq!(par, 1);

        let (mem, time, par) = KdfProfile::Mobile.params();
        assert_eq!(mem, 8192);
        assert_eq!(time, 3);
        assert_eq!(par, 1);
    }

    #[test]
    fn rotate_password() {
        let secret = b"my-key";
        let ks = encrypt_keystore(secret, "old-pw", None, 8 * 1024, 1, 1);
        let rotated = rotate_keystore(&ks, "old-pw", "new-pw", 8 * 1024, 1, 1).expect("rotate");

        assert!(decrypt_keystore(&rotated, "old-pw").is_err());

        let pt = decrypt_keystore(&rotated, "new-pw").expect("decrypt with new pw");
        assert_eq!(pt, secret);
    }
}

#[test]
fn corrupted_file_handling() {
    use tempfile::tempdir;
    let dir = tempdir().unwrap();
    let path = dir.path().join("corrupt.keystore");

    // Invalid JSON
    std::fs::write(&path, b"{invalid}").unwrap();
    assert!(read_keystore_file(&path).is_err());

    // Truncated valid keystore
    let ks = encrypt_keystore(b"test", "pw", None, 8 * 1024, 1, 1);
    let json = ks.to_json().unwrap();
    std::fs::write(&path, &json[..json.len() / 2]).unwrap();
    assert!(read_keystore_file(&path).is_err());

    // Missing required fields
    std::fs::write(&path, r#"{"version": 1}"#).unwrap();
    assert!(read_keystore_file(&path).is_err());
}

#[test]
fn decrypt_corrupted_fields() {
    let ks = encrypt_keystore(b"test", "pw", None, 8 * 1024, 1, 1);

    // Corrupt salt
    let mut bad = ks.clone();
    bad.kdf.salt_b64 = "invalid!!!base64".to_string();
    assert!(decrypt_keystore(&bad, "pw").is_err());

    // Corrupt nonce
    let mut bad = ks.clone();
    bad.nonce_b64 = "bad".to_string();
    assert!(decrypt_keystore(&bad, "pw").is_err());

    // Corrupt ciphertext
    let mut bad = ks.clone();
    bad.ciphertext_b64 = "xyz".to_string();
    assert!(decrypt_keystore(&bad, "pw").is_err());
}
