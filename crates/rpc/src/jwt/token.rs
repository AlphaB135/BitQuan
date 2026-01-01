//! JWT token generation
use super::claims::Claims;
use jsonwebtoken::{
    decode, encode, errors::Error, Algorithm, DecodingKey, EncodingKey, Header, Validation,
};

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
        self.generate(&claims.sub, &claims.role)
            .map_err(|e| e.to_string())
    }

    /// Verify token and extract claims.
    ///
    /// # Security
    /// - **Enforces HS256 algorithm** - Rejects "none" and other algorithms
    /// - **Validates expiration** - Token must not be expired
    /// - **Clock drift tolerance** - 60 second leeway for time sync issues
    ///
    /// This prevents the infamous "Algorithm None" attack where attackers
    /// forge tokens by setting `alg: "none"` in the JWT header.
    pub fn verify(&self, token: &str) -> Result<Claims, Error> {
        // CRITICAL: Explicitly enforce HS256 to prevent "Algorithm None" attack
        // DO NOT use Validation::default() - it accepts ANY algorithm including "none"!
        let mut validation = Validation::new(Algorithm::HS256);

        // Allow 60 seconds of clock drift between server and client
        // This handles minor time synchronization differences
        validation.leeway = 60;

        // Ensure expiration is validated (default is true, but explicit is safer)
        validation.validate_exp = true;

        decode::<Claims>(token, &self.decoding_key, &validation).map(|data| data.claims)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};

    #[test]
    fn test_token_roundtrip() {
        let gen = TokenGenerator::new("test-secret");
        let token = gen
            .generate("alice", "admin")
            .unwrap_or_else(|e| panic!("Failed to generate token: {}", e));
        let claims = gen
            .verify(&token)
            .unwrap_or_else(|e| panic!("Failed to verify token: {}", e));
        assert_eq!(claims.sub, "alice");
    }

    /// CRITICAL SECURITY TEST: Ensure "Algorithm None" attack is rejected
    ///
    /// This test creates a forged token with `alg: "none"` (no signature)
    /// and verifies that our implementation correctly rejects it.
    ///
    /// Without this fix, an attacker could:
    /// 1. Take any valid token
    /// 2. Change the header to `{"alg":"none","typ":"JWT"}`
    /// 3. Modify claims (e.g., change role to "admin")
    /// 4. Remove the signature
    /// 5. Server would accept it!
    #[test]
    fn test_algorithm_none_attack_is_rejected() {
        let gen = TokenGenerator::new("test-secret");

        // Create a forged token with alg: "none"
        // Header: {"alg":"none","typ":"JWT"}
        let header = URL_SAFE_NO_PAD.encode(r#"{"alg":"none","typ":"JWT"}"#);

        // Payload: fake admin claims
        let now = chrono::Utc::now().timestamp();
        let payload = URL_SAFE_NO_PAD.encode(format!(
            r#"{{"sub":"hacker","role":"admin","exp":{},"iat":{}}}"#,
            now + 3600,
            now
        ));

        // Forged token with empty signature (alg: none doesn't need one)
        let forged_token = format!("{}.{}.", header, payload);

        // This MUST fail - if it passes, we have a critical vulnerability!
        let result = gen.verify(&forged_token);
        assert!(
            result.is_err(),
            "CRITICAL SECURITY FAILURE: Algorithm 'none' attack was accepted! Token: {}",
            forged_token
        );

        // Verify the error rejects the "none" algorithm
        // jsonwebtoken library doesn't even recognize "none" as a valid algorithm variant
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("InvalidAlgorithm")
                || err.contains("InvalidSignature")
                || err.contains("unknown variant `none`"),
            "Expected algorithm rejection error, got: {}",
            err
        );
    }

    /// Test that tokens signed with wrong algorithm are rejected
    #[test]
    fn test_wrong_algorithm_rejected() {
        let gen = TokenGenerator::new("test-secret");

        // Valid HS256 token should work
        let valid_token = gen.generate("alice", "user").unwrap();
        assert!(gen.verify(&valid_token).is_ok());

        // Tampered token (modified payload) should fail
        let parts: Vec<&str> = valid_token.split('.').collect();
        assert_eq!(parts.len(), 3, "JWT should have 3 parts");

        // Modify the payload
        let tampered_payload =
            URL_SAFE_NO_PAD.encode(r#"{"sub":"hacker","role":"admin","exp":9999999999,"iat":0}"#);
        let tampered_token = format!("{}.{}.{}", parts[0], tampered_payload, parts[2]);

        // Tampered token MUST be rejected
        assert!(
            gen.verify(&tampered_token).is_err(),
            "Tampered token should be rejected"
        );
    }
}
