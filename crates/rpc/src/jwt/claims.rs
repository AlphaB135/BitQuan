//! JWT Claims structure
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String,
    pub role: String,
    pub exp: i64,
    pub iat: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh: Option<bool>, // true if this is a refresh token
}

impl Claims {
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
    
    pub fn is_refresh_token(&self) -> bool {
        self.refresh.unwrap_or(false)
    }
    
    pub fn is_expired(&self) -> bool {
        chrono::Utc::now().timestamp() > self.exp
    }
    
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
