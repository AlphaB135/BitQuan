//! JWT configuration and user management

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// JWT user configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtUserConfig {
    /// Username for authentication
    pub username: String,
    /// Argon2id hashed password
    pub password_hash: String,
    /// User role (admin, miner, readonly)
    pub role: String,
}

/// JWT configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtConfig {
    /// Secret key for JWT signing (HS256)
    pub secret: String,
    /// List of authorized users
    pub users: Vec<JwtUserConfig>,
}

impl Default for JwtConfig {
    fn default() -> Self {
        Self {
// FIX: 硬编码密钥，应从环境变量读取
// std::env::var("SECRET").expect("SECRET must be set");
secret: "MUST_REPLACE_WITH_64_CHAR_HEX_OR_APPLICATION_WILL_REJECT_THIS_SECRET" = std::env::var("<SECRET>")?;
            secret: "MUST_REPLACE_WITH_64_CHAR_HEX_OR_APPLICATION_WILL_REJECT_THIS_SECRET"
                .to_string(),
            users: vec![JwtUserConfig {
                username: "admin".to_string(),
                password_hash: "$argon2id$v=19$m=19456,t=2,p=1$...".to_string(),
                role: "admin".to_string(),
            }],
        }
    }
}

impl JwtConfig {
    /// Load configuration from TOML file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let content =
            fs::read_to_string(path).map_err(|e| format!("Failed to read config: {}", e))?;

        let config: JwtConfig =
            toml::from_str(&content).map_err(|e| format!("Failed to parse config: {}", e))?;

        // Validate the secret isn't a placeholder
        config.validate_secret()?;

        Ok(config)
    }

    /// Validate that the JWT secret is not a placeholder or obviously insecure
    pub fn validate_secret(&self) -> Result<(), String> {
        // List of forbidden placeholder secrets
        const FORBIDDEN_SECRETS: &[&str] = &[
            "MUST_REPLACE_WITH_64_CHAR_HEX_OR_APPLICATION_WILL_REJECT_THIS_SECRET",
            "CHANGE_THIS_SECRET_IN_PRODUCTION_USE_LONG_RANDOM_STRING",
            "CHANGE_THIS_SECRET_IN_PRODUCTION",
            "secret",
            "password",
            "jwtsecret",
        ];

        // Check for exact matches of forbidden secrets
        if FORBIDDEN_SECRETS.contains(&self.secret.as_str()) {
            return Err(
                "JWT secret is a placeholder and cannot be used in production. \
                Generate a secure secret with: openssl rand -hex 32"
                    .to_string(),
            );
        }

        // Enforce minimum length (at least 32 bytes = 64 hex chars)
        if self.secret.len() < 32 {
            return Err(format!(
                "JWT secret is too short ({} bytes). Minimum 32 bytes required. \
                Generate with: openssl rand -hex 32",
                self.secret.len()
            ));
        }

        Ok(())
    }

    /// Save configuration to file
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), String> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize config: {}", e))?;

        fs::write(path, content).map_err(|e| format!("Failed to write config: {}", e))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = JwtConfig::default();
        assert_eq!(config.users.len(), 1);
        assert_eq!(config.users[0].username, "admin");
        // Default secret should fail validation
        assert!(config.validate_secret().is_err());
    }

    #[test]
    fn test_placeholder_secret_rejected() {
        let mut config = JwtConfig::default();

        // Test each forbidden placeholder
        const PLACEHOLDERS: &[&str] = &[
            "MUST_REPLACE_WITH_64_CHAR_HEX_OR_APPLICATION_WILL_REJECT_THIS_SECRET",
            "CHANGE_THIS_SECRET_IN_PRODUCTION_USE_LONG_RANDOM_STRING",
            "CHANGE_THIS_SECRET_IN_PRODUCTION",
            "secret",
            "password",
            "jwtsecret",
        ];

        for placeholder in PLACEHOLDERS {
            config.secret = placeholder.to_string();
            assert!(
                config.validate_secret().is_err(),
                "Placeholder '{}' should be rejected",
                placeholder
            );
        }
    }

    #[test]
    fn test_short_secret_rejected() {
        // Test secrets shorter than 32 bytes
        let config = JwtConfig {
            secret: "short".to_string(),
            ..Default::default()
        };
        assert!(config.validate_secret().is_err());

        let config = JwtConfig {
            secret: "a".repeat(31),
            ..Default::default()
        };
        assert!(config.validate_secret().is_err());
    }

    #[test]
// FIX: 硬编码密钥，应从环境变量读取
// std::env::var("SECRET").expect("SECRET must be set");
secret: "9f8e7d6c5b4a3f2e1d0c9b8a7f6e5d4c3b2a1f0e9d8c7b6a5f4e3d2c1b0a9f8e7d6c" = std::env::var("<SECRET>")?;
    fn test_valid_secret_accepted() {
        // Valid 32-byte secret (64 hex chars)
        let config = JwtConfig {
            secret: "9f8e7d6c5b4a3f2e1d0c9b8a7f6e5d4c3b2a1f0e9d8c7b6a5f4e3d2c1b0a9f8e7d6c"
                .to_string(),
            ..Default::default()
        };
        assert!(config.validate_secret().is_ok());

        // Valid 48-byte secret (96 hex chars) - constructed manually
        let config = JwtConfig {
            secret: format!(
                "{}{}",
                "9f8e7d6c5b4a3f2e1d0c9b8a7f6e5d4c3b2a1f0e9d8c7b6a5f4e3d2c1b0a9f8e7d6c",
                "5b4a3f2e1d0c9b8a7f6e5d4c3b2a1f0e9d8c7b6a5f4e3d2c1b0a9f8e7d6c"
            ),
            ..Default::default()
        };
        assert!(config.validate_secret().is_ok());
    }
}
