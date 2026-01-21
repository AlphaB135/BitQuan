//! JWT secret management (Ethereum Engine API style)
//!
//! Manages a 32-byte hex secret stored in `jwt.hex` file.
//! If the file doesn't exist, a new secret is generated and saved.

use rand::RngCore;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use tracing::{info, warn};

/// Default JWT secret filename
pub const JWT_SECRET_FILENAME: &str = "jwt.hex";

/// JWT secret size in bytes (256 bits)
pub const JWT_SECRET_BYTES: usize = 32;

/// JWT Secret Manager
///
/// Handles loading or generating the JWT secret from a hex file.
/// Compatible with Ethereum Engine API's jwt.hex format.
#[derive(Debug, Clone)]
pub struct JwtSecretManager {
    /// The raw 32-byte secret
    secret: [u8; JWT_SECRET_BYTES],
    /// Path to the jwt.hex file
    path: PathBuf,
}

impl JwtSecretManager {
    /// Load or generate JWT secret from the given data directory.
    ///
    /// If `jwt.hex` exists, reads and validates it.
    /// If not, generates a new 32-byte random secret and saves it.
    ///
    /// # Arguments
    /// * `data_dir` - The data directory where jwt.hex should be stored
    ///
    /// # Returns
    /// * `Ok(JwtSecretManager)` - The secret manager with loaded/generated secret
    /// * `Err(io::Error)` - If file operations fail or secret is invalid
    pub fn load_or_generate<P: AsRef<Path>>(data_dir: P) -> io::Result<Self> {
        let path = data_dir.as_ref().join(JWT_SECRET_FILENAME);

        if path.exists() {
            Self::load_from_file(&path)
        } else {
            Self::generate_and_save(&path)
        }
    }

    /// Load JWT secret from an existing file.
    ///
    /// The file should contain a 64-character hex string (optionally with 0x prefix).
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let path = path.as_ref().to_path_buf();
        let content = fs::read_to_string(&path)?;

        let hex_str = content.trim();

        // Strip optional 0x prefix (for compatibility)
        let hex_str = hex_str.strip_prefix("0x").unwrap_or(hex_str);

        // Validate length
        if hex_str.len() != JWT_SECRET_BYTES * 2 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "JWT secret must be {} hex characters, got {}",
                    JWT_SECRET_BYTES * 2,
                    hex_str.len()
                ),
            ));
        }

        // Decode hex
        let bytes = hex::decode(hex_str).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid hex in JWT secret: {}", e),
            )
        })?;

        let mut secret = [0u8; JWT_SECRET_BYTES];
        secret.copy_from_slice(&bytes);

        info!("Loaded JWT secret from {}", path.display());

        Ok(Self { secret, path })
    }

    /// Generate a new random secret and save it to file.
    fn generate_and_save<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let path = path.as_ref().to_path_buf();

        // Generate 32 random bytes
        let mut secret = [0u8; JWT_SECRET_BYTES];
        rand::thread_rng().fill_bytes(&mut secret);

        // Encode as hex
        let hex_str = hex::encode(secret);

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Write to file with restricted permissions
        fs::write(&path, &hex_str)?;

        // On Unix, set file permissions to 0600 (owner read/write only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let permissions = fs::Permissions::from_mode(0o600);
            fs::set_permissions(&path, permissions)?;
        }

        info!(
            "Generated new JWT secret and saved to {} (keep this file secure!)",
            path.display()
        );
        warn!(
            "⚠️  JWT secret file created. Protect {} from unauthorized access!",
            path.display()
        );

        Ok(Self { secret, path })
    }

    /// Get the secret as a hex string (for JWT signing).
    ///
    /// Returns the 64-character hex representation of the secret.
    pub fn as_hex(&self) -> String {
        hex::encode(self.secret)
    }

    /// Get the raw secret bytes.
    ///
    /// Use with caution - this exposes the raw cryptographic material.
    pub fn as_bytes(&self) -> &[u8; JWT_SECRET_BYTES] {
        &self.secret
    }

    /// Get the path to the jwt.hex file.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Create a JwtAuth instance using this secret.
    pub fn into_jwt_auth(self) -> crate::jwt::JwtAuth {
        crate::jwt::JwtAuth::new(&self.as_hex())
    }
}

impl Drop for JwtSecretManager {
    fn drop(&mut self) {
        // Zeroize secret on drop for security
        self.secret.fill(0);
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;

    #[test]
    fn test_generate_and_load() {
        let temp_dir = std::env::temp_dir().join("bitquan_jwt_test");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).expect("Failed to create temp dir");

        // Generate new secret
        let manager1 =
            JwtSecretManager::load_or_generate(&temp_dir).expect("Failed to generate JWT secret");

        // Reload and verify it matches
        let manager2 =
            JwtSecretManager::load_or_generate(&temp_dir).expect("Failed to load JWT secret");

        assert_eq!(manager1.as_hex(), manager2.as_hex());
        assert_eq!(manager1.as_hex().len(), 64);

        // Cleanup
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_load_with_0x_prefix() {
        let temp_dir = std::env::temp_dir().join("bitquan_jwt_test_0x");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).expect("Failed to create temp dir");

        // Write a secret with 0x prefix
        let secret_hex = "0x".to_string() + &"a".repeat(64);
        let path = temp_dir.join(JWT_SECRET_FILENAME);
        let mut file = fs::File::create(&path).expect("Failed to create file");
        file.write_all(secret_hex.as_bytes())
            .expect("Failed to write");

        // Load and verify
        let manager =
            JwtSecretManager::load_from_file(&path).expect("Failed to load with 0x prefix");
        assert_eq!(manager.as_hex(), "a".repeat(64));

        // Cleanup
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_invalid_secret_length() {
        let temp_dir = std::env::temp_dir().join("bitquan_jwt_test_invalid");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).expect("Failed to create temp dir");

        // Write a short secret
        let path = temp_dir.join(JWT_SECRET_FILENAME);
        fs::write(&path, "abcd1234").expect("Failed to write");

        // Should fail
        let result = JwtSecretManager::load_from_file(&path);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("must be 64 hex characters"));

        // Cleanup
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_into_jwt_auth() {
        let temp_dir = std::env::temp_dir().join("bitquan_jwt_test_auth");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).expect("Failed to create temp dir");

        let manager =
            JwtSecretManager::load_or_generate(&temp_dir).expect("Failed to generate JWT secret");

        let mut jwt_auth = manager.into_jwt_auth();

        // Create test user for authentication
        jwt_auth
            .add_user_plaintext("admin", "admin123", "admin")
            .expect("Failed to create test user");

        // Login should work
        let result = jwt_auth.login("admin", "admin123");
        assert!(result.is_ok());

        // Cleanup
        let _ = fs::remove_dir_all(&temp_dir);
    }
}
