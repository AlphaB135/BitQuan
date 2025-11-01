//! JWT token generation
use jsonwebtoken::{encode, decode, Header, Validation, EncodingKey, DecodingKey, errors::Error};
use super::claims::Claims;

/// JWT token generator and verifier
pub struct TokenGenerator {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
}

impl TokenGenerator {
    /// Create new token generator with secret key
    pub fn new(secret: &str) -> Self {
        let bytes = secret.as_bytes();
        Self {
            encoding_key: EncodingKey::from_secret(bytes),
            decoding_key: DecodingKey::from_secret(bytes),
        }
    }
    
    /// Generate access token (expires in 1 hour)
    pub fn generate(&self, username: &str, role: &str) -> Result<String, Error> {
        let claims = Claims::new(username.to_string(), role.to_string(), 3600);
        encode(&Header::default(), &claims, &self.encoding_key)
    }
    
    /// Generate refresh token (expires in 7 days)
    pub fn generate_refresh_token(&self, username: &str, role: &str) -> Result<String, Error> {
        // Refresh token expires in 7 days
        let claims = Claims::new_refresh_token(username.to_string(), role.to_string(), 604800);
        encode(&Header::default(), &claims, &self.encoding_key)
    }
    
    /// Refresh access token using refresh token
    pub fn refresh(&self, refresh_token: &str) -> Result<String, String> {
        // Verify the refresh token
        let claims = self.verify(refresh_token).map_err(|e| e.to_string())?;
        
        // Must be a refresh token
        if !claims.is_refresh_token() {
            return Err("Not a refresh token".to_string());
        }
        
        // Generate new access token with same role
        self.generate(&claims.sub, &claims.role).map_err(|e| e.to_string())
    }
    
    /// Verify token and extract claims
    pub fn verify(&self, token: &str) -> Result<Claims, Error> {
        decode::<Claims>(token, &self.decoding_key, &Validation::default())
            .map(|data| data.claims)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_token_roundtrip() {
        let gen = TokenGenerator::new("test-secret");
        let token = gen.generate("alice", "admin").unwrap();
        let claims = gen.verify(&token).unwrap();
        assert_eq!(claims.sub, "alice");
    }
}
