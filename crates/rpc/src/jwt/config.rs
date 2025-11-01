//! JWT configuration and user management

use serde::{Deserialize, Serialize};
use std::path::Path;
use std::fs;

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

impl JwtConfig {
    /// Load configuration from TOML file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let content = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read config: {}", e))?;
        
        toml::from_str(&content)
            .map_err(|e| format!("Failed to parse config: {}", e))
    }
    
    /// Create default configuration
    pub fn default() -> Self {
        Self {
            secret: "CHANGE_THIS_SECRET_IN_PRODUCTION".to_string(),
            users: vec![
                JwtUserConfig {
                    username: "admin".to_string(),
                    password_hash: "$argon2id$v=19$m=19456,t=2,p=1$...".to_string(),
                    role: "admin".to_string(),
                },
            ],
        }
    }
    
    /// Save configuration to file
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), String> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize config: {}", e))?;
        
        fs::write(path, content)
            .map_err(|e| format!("Failed to write config: {}", e))?;
        
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
    }
}
