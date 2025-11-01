//! JWT authentication
use super::claims::Claims;
use super::token::TokenGenerator;
use argon2::{
    password_hash::{rand_core::OsRng, SaltString},
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
};
use std::collections::HashMap;

/// JWT authentication manager
pub struct JwtAuth {
    token_gen: TokenGenerator,
    users: HashMap<String, UserCredentials>,
}

struct UserCredentials {
    password_hash: String,
    role: String,
}

impl UserCredentials {
    /// Create credentials with hashed password
    fn new(password: &str, role: &str) -> Result<Self, String> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();

        let password_hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| format!("Password hashing failed: {}", e))?
            .to_string();

        Ok(Self {
            password_hash,
            role: role.to_string(),
        })
    }

    /// Verify password against hash
    fn verify_password(&self, password: &str) -> bool {
        let parsed_hash = match PasswordHash::new(&self.password_hash) {
            Ok(h) => h,
            Err(_) => return false,
        };

        Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok()
    }
}

impl JwtAuth {
    /// Create JWT auth with default admin user
    pub fn new(secret: &str) -> Self {
        let mut users = HashMap::new();

        // Create default admin with hashed password
        if let Ok(admin_creds) = UserCredentials::new("admin123", "admin") {
            users.insert("admin".to_string(), admin_creds);
        }

        Self {
            token_gen: TokenGenerator::new(secret),
            users,
        }
    }

    /// Create JWT auth from config file
    pub fn from_config(config: &crate::jwt::JwtConfig) -> Result<Self, String> {
        let mut jwt_auth = Self::new_empty(&config.secret);

        for user in &config.users {
            jwt_auth.add_user_hashed(
                user.username.clone(),
                user.password_hash.clone(),
                user.role.clone(),
            );
        }

        Ok(jwt_auth)
    }

    /// Create empty JWT auth (for loading from config)
    pub fn new_empty(secret: &str) -> Self {
        Self {
            token_gen: TokenGenerator::new(secret),
            users: HashMap::new(),
        }
    }

    /// Add user with plaintext password (will be hashed)
    pub fn add_user_plaintext(
        &mut self,
        username: &str,
        password: &str,
        role: &str,
    ) -> Result<(), String> {
        let creds = UserCredentials::new(password, role)?;
        self.users.insert(username.to_string(), creds);
        Ok(())
    }

    /// Add user with pre-hashed password (e.g., from config file)
    pub fn add_user_hashed(&mut self, username: String, password_hash: String, role: String) {
        self.users.insert(
            username,
            UserCredentials {
                password_hash,
                role,
            },
        );
    }

    /// Login with username and password
    pub fn login(&self, username: &str, password: &str) -> Result<String, String> {
        match self.users.get(username) {
            Some(creds) if creds.verify_password(password) => self
                .token_gen
                .generate(username, &creds.role)
                .map_err(|e| e.to_string()),
            Some(_) => Err("Invalid password".to_string()),
            None => Err("User not found".to_string()),
        }
    }

    /// Login and get both access and refresh tokens
    pub fn login_with_refresh(
        &self,
        username: &str,
        password: &str,
    ) -> Result<(String, String), String> {
        match self.users.get(username) {
            Some(creds) if creds.verify_password(password) => {
                let access_token = self
                    .token_gen
                    .generate(username, &creds.role)
                    .map_err(|e| e.to_string())?;
                let refresh_token = self
                    .token_gen
                    .generate_refresh_token(username, &creds.role)
                    .map_err(|e| e.to_string())?;
                Ok((access_token, refresh_token))
            }
            Some(_) => Err("Invalid password".to_string()),
            None => Err("User not found".to_string()),
        }
    }

    /// Refresh an access token using a refresh token
    pub fn refresh_token(&self, refresh_token: &str) -> Result<String, String> {
        self.token_gen
            .refresh(refresh_token)
            .map_err(|e| e.to_string())
    }

    /// Verify token and return claims
    pub fn verify_token(&self, token: &str) -> Result<Claims, String> {
        self.token_gen
            .verify(token)
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
