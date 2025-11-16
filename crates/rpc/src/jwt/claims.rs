//! JWT Claims structure
use serde::{Deserialize, Serialize};

/// JWT token claims
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    /// Subject (username)
    pub sub: String,
    /// User role (admin, miner, readonly)
    pub role: String,
    /// Expiration time (Unix timestamp)
    pub exp: i64,
    /// Issued at (Unix timestamp)
    pub iat: i64,
    /// Whether this is a refresh token
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh: Option<bool>,
}

impl Claims {
    /// Create new access token claims
    pub fn new(username: String, role: String, expires_in_secs: i64) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            sub: username,
            role,
            exp: now + expires_in_secs,
            iat: now,
            refresh: None,
        }
    }

    /// Create new refresh token claims
    pub fn new_refresh_token(username: String, role: String, expires_in_secs: i64) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            sub: username,
            role,
            exp: now + expires_in_secs,
            iat: now,
            refresh: Some(true),
        }
    }

    /// Check if this is a refresh token
    pub fn is_refresh_token(&self) -> bool {
        self.refresh.unwrap_or(false)
    }

    /// Check if token has expired
    pub fn is_expired(&self) -> bool {
        chrono::Utc::now().timestamp() > self.exp
    }

    /// Check if user has admin role
    pub fn is_admin(&self) -> bool {
        self.role == "admin"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_claims_creation() {
        let claims = Claims::new("alice".to_string(), "admin".to_string(), 3600);
        assert_eq!(claims.sub, "alice");
        assert!(!claims.is_expired());
    }
}
