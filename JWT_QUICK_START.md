# JWT Authentication - Quick Start (MVP in 2 hours!)

**Goal**: Get working JWT auth ASAP, iterate later

**Timeline**: 2-3 hours for MVP, then 1-2 weeks for full features

---

## ⚡ Phase 0: MVP (2-3 hours) - DO THIS FIRST

### What we'll build:
- ✅ JWT token generation
- ✅ JWT token verification  
- ✅ Bearer token authentication
- ✅ Basic role checking
- ✅ Login endpoint
- ✅ Backward compatibility with Basic Auth

### What we'll skip (for now):
- ❌ Token refresh
- ❌ Token revocation (can add later)
- ❌ Complex RBAC (start with 2-3 roles)
- ❌ Token caching (optimize later)

---

## Step 1: Create JWT Module (10 min)

```bash
# Create files
mkdir -p crates/rpc/src/jwt
touch crates/rpc/src/jwt/mod.rs
touch crates/rpc/src/jwt/token.rs
touch crates/rpc/src/jwt/claims.rs
touch crates/rpc/src/jwt/auth.rs
```

---

## Step 2: Implement Claims (15 min)

```rust
// crates/rpc/src/jwt/claims.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String,        // Subject (username)
    pub role: String,       // Role (admin/miner/readonly)
    pub exp: i64,           // Expiration timestamp
    pub iat: i64,           // Issued at
}

impl Claims {
    pub fn new(username: String, role: String, expires_in_secs: i64) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            sub: username,
            role,
            exp: now + expires_in_secs,
            iat: now,
        }
    }
    
    pub fn is_expired(&self) -> bool {
        chrono::Utc::now().timestamp() > self.exp
    }
    
    pub fn is_admin(&self) -> bool {
        self.role == "admin"
    }
}
```

---

## Step 3: Token Generator (20 min)

```rust
// crates/rpc/src/jwt/token.rs
use jsonwebtoken::{encode, decode, Header, Validation, EncodingKey, DecodingKey, Algorithm, errors::Error};
use super::claims::Claims;

pub struct TokenGenerator {
    secret: Vec<u8>,
}

impl TokenGenerator {
    pub fn new(secret: &str) -> Self {
        Self {
            secret: secret.as_bytes().to_vec(),
        }
    }
    
    pub fn generate(&self, username: &str, role: &str) -> Result<String, Error> {
        let claims = Claims::new(
            username.to_string(),
            role.to_string(),
            3600,  // 1 hour
        );
        
        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(&self.secret),
        )
    }
    
    pub fn verify(&self, token: &str) -> Result<Claims, Error> {
        decode::<Claims>(
            token,
            &DecodingKey::from_secret(&self.secret),
            &Validation::default(),
        )
        .map(|data| data.claims)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_token_roundtrip() {
        let gen = TokenGenerator::new("test-secret-key-123");
        let token = gen.generate("alice", "admin").unwrap();
        let claims = gen.verify(&token).unwrap();
        assert_eq!(claims.sub, "alice");
        assert_eq!(claims.role, "admin");
    }
}
```

---

## Step 4: JWT Auth Struct (20 min)

```rust
// crates/rpc/src/jwt/auth.rs
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
        
        // Default admin user (TODO: load from config)
        users.insert(
            "admin".to_string(),
            UserCredentials {
                password: "admin123".to_string(),  // TODO: hash this!
                role: "admin".to_string(),
            },
        );
        
        Self {
            token_gen: TokenGenerator::new(secret),
            users,
        }
    }
    
    pub fn login(&self, username: &str, password: &str) -> Result<String, String> {
        match self.users.get(username) {
            Some(creds) if creds.password == password => {
                self.token_gen
                    .generate(username, &creds.role)
                    .map_err(|e| e.to_string())
            }
            _ => Err("Invalid credentials".to_string()),
        }
    }
    
    pub fn verify_token(&self, token: &str) -> Result<Claims, String> {
        self.token_gen
            .verify(token)
            .map_err(|e| e.to_string())
    }
}
```

---

## Step 5: Module Integration (10 min)

```rust
// crates/rpc/src/jwt/mod.rs
pub mod token;
pub mod claims;
pub mod auth;

pub use token::TokenGenerator;
pub use claims::Claims;
pub use auth::JwtAuth;
```

```rust
// crates/rpc/src/lib.rs
pub mod jwt;  // Add this line
```

---

## Step 6: Update Server (30 min)

```rust
// crates/rpc/src/server.rs

// Add new auth enum
pub enum AuthMethod {
    Basic(RpcAuth),          // Old way (deprecated)
    Jwt(Arc<JwtAuth>),       // New way
}

// Update RpcServer
pub struct RpcServer<T> {
    handler: Arc<T>,
    addr: String,
    auth: Option<AuthMethod>,  // Changed type
    // ... rest
}

// Update authentication logic
fn handle_connection() {
    // Extract Authorization header
    let auth_header = extract_auth_header(&headers)?;
    
    match &auth {
        Some(AuthMethod::Basic(basic_auth)) => {
            // Old way - Basic Auth
            if auth_header.starts_with("Basic ") {
                authenticate_basic(auth_header, basic_auth)?;
            }
        }
        Some(AuthMethod::Jwt(jwt_auth)) => {
            // New way - JWT
            if auth_header.starts_with("Bearer ") {
                let token = &auth_header[7..];  // Strip "Bearer "
                jwt_auth.verify_token(token)?;
            } else if auth_header.starts_with("Basic ") {
                return Err("Basic Auth deprecated. Use JWT.");
            }
        }
        None => {
            // No auth required
        }
    }
}
```

---

## Step 7: Add Login Endpoint (30 min)

```rust
// Add to handle_connection() or dispatch_call()

if method == "POST" && path == "/auth/login" {
    // Parse login request
    let body: LoginRequest = serde_json::from_slice(&body)?;
    
    if let Some(AuthMethod::Jwt(jwt_auth)) = &auth {
        match jwt_auth.login(&body.username, &body.password) {
            Ok(token) => {
                let response = json!({
                    "access_token": token,
                    "token_type": "Bearer",
                    "expires_in": 3600,
                });
                respond_json(stream, &response, config)?;
            }
            Err(e) => {
                let error = json!({
                    "error": "Invalid credentials",
                    "message": e,
                });
                respond_error(stream, &error, 401)?;
            }
        }
    }
    return Ok(());
}

#[derive(Deserialize)]
struct LoginRequest {
    username: String,
    password: String,
}
```

---

## Step 8: Test It! (20 min)

```bash
# 1. Start node with JWT
cargo run --bin bitquan-node -- --rpc-jwt-secret "my-super-secret-key"

# 2. Login
curl -X POST http://localhost:8332/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","password":"admin123"}'

# Should return:
# {
#   "access_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
#   "token_type": "Bearer",
#   "expires_in": 3600
# }

# 3. Use token
TOKEN="eyJhbG..."

curl -X POST http://localhost:8332 \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "getblockcount",
    "id": 1
  }'
```

---

## Step 9: Write Tests (20 min)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_jwt_login() {
        let jwt_auth = JwtAuth::new("test-secret");
        let token = jwt_auth.login("admin", "admin123").unwrap();
        assert!(!token.is_empty());
    }
    
    #[test]
    fn test_jwt_verification() {
        let jwt_auth = JwtAuth::new("test-secret");
        let token = jwt_auth.login("admin", "admin123").unwrap();
        let claims = jwt_auth.verify_token(&token).unwrap();
        assert_eq!(claims.sub, "admin");
        assert!(claims.is_admin());
    }
    
    #[test]
    fn test_invalid_credentials() {
        let jwt_auth = JwtAuth::new("test-secret");
        assert!(jwt_auth.login("admin", "wrong").is_err());
    }
}
```

---

## ✅ MVP Checklist (2-3 hours)

- [ ] Create JWT module structure (10 min)
- [ ] Implement Claims (15 min)
- [ ] Implement TokenGenerator (20 min)
- [ ] Implement JwtAuth (20 min)
- [ ] Module integration (10 min)
- [ ] Update server.rs (30 min)
- [ ] Add login endpoint (30 min)
- [ ] Test manually (20 min)
- [ ] Write unit tests (20 min)

**Total**: ~2.5 hours

---

## 🚀 Phase 1: Polish (1-2 days after MVP)

Once MVP works, add:

1. **Password Hashing**
   ```rust
   use argon2::{Argon2, PasswordHasher};
   
   fn hash_password(password: &str) -> String {
       // Use argon2 (we already have it!)
   }
   ```

2. **Config File Users**
   ```toml
   [[users]]
   username = "admin"
   password_hash = "$argon2id$..."
   role = "admin"
   
   [[users]]
   username = "miner1"
   password_hash = "$argon2id$..."
   role = "miner"
   ```

3. **Token Refresh**
   ```rust
   pub fn refresh(&self, old_token: &str) -> Result<String> {
       let claims = self.verify_token(old_token)?;
       self.generate(&claims.sub, &claims.role)
   }
   ```

4. **Permissions per Method**
   ```rust
   fn get_required_permission(method: &str) -> Permission {
       match method {
           "getblockcount" => Permission::Read,
           "sendrawtransaction" => Permission::Write,
           "stop" => Permission::Admin,
           _ => Permission::Read,
       }
   }
   ```

---

## 📋 Phase 2: Production Features (1 week)

1. Token revocation list
2. Multiple roles & permissions
3. User management API
4. Audit logging
5. Rate limiting per user
6. CLI for user management

---

## 🎯 Focus for TODAY

**Just get the MVP working in 2-3 hours!**

Then we can iterate and add features incrementally.

**Start with Step 1?** 🚀
