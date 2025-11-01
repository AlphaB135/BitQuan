//! JWT authentication
use super::token::TokenGenerator;
use super::claims::Claims;
use std::collections::HashMap;

pub struct JwtAuth {
    token_gen: TokenGenerator,
    users: HashMap<String, UserCredentials>,
}

struct UserCredentials {
    password: String,
    role: String,
}

impl JwtAuth {
    pub fn new(secret: &str) -> Self {
        let mut users = HashMap::new();
        users.insert("admin".to_string(), UserCredentials {
            password: "admin123".to_string(),
            role: "admin".to_string(),
        });
        
        Self {
            token_gen: TokenGenerator::new(secret),
            users,
        }
    }
    
    pub fn login(&self, username: &str, password: &str) -> Result<String, String> {
        match self.users.get(username) {
            Some(creds) if creds.password == password => {
                self.token_gen.generate(username, &creds.role)
                    .map_err(|e| e.to_string())
            }
            _ => Err("Invalid credentials".to_string()),
        }
    }
    
    pub fn verify_token(&self, token: &str) -> Result<Claims, String> {
        self.token_gen.verify(token)
            .map_err(|e| e.to_string())
            .and_then(|claims| {
                if claims.is_expired() {
                    Err("Token expired".to_string())
                } else {
                    Ok(claims)
                }
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_jwt_login() {
        let jwt = JwtAuth::new("test-secret");
        let token = jwt.login("admin", "admin123").unwrap();
        assert!(!token.is_empty());
    }
}
