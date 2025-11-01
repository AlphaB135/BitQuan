//! JWT token generation
use jsonwebtoken::{encode, decode, Header, Validation, EncodingKey, DecodingKey, errors::Error};
use super::claims::Claims;

pub struct TokenGenerator {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
}

impl TokenGenerator {
    pub fn new(secret: &str) -> Self {
        let bytes = secret.as_bytes();
        Self {
            encoding_key: EncodingKey::from_secret(bytes),
            decoding_key: DecodingKey::from_secret(bytes),
        }
    }
    
    pub fn generate(&self, username: &str, role: &str) -> Result<String, Error> {
        let claims = Claims::new(username.to_string(), role.to_string(), 3600);
        encode(&Header::default(), &claims, &self.encoding_key)
    }
    
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
