# JWT Authentication Implementation Plan

**Goal**: Replace Basic Auth with JWT (JSON Web Tokens) for secure, stateless authentication

**Timeline**: 1-2 weeks
**Priority**: P0 (Critical Security)

---

## Current State ❌

```rust
// Basic Auth (INSECURE for production)
pub struct RpcAuth {
    username: String,
    password: String,  // Sent in every request!
}

// Authorization: Basic base64(username:password)
```

**Problems**:
1. ❌ Credentials sent with EVERY request
2. ❌ No expiration (永久有效)
3. ❌ No role-based access control (RBAC)
4. ❌ No token revocation
5. ❌ Hard to audit
6. ❌ Vulnerable to replay attacks

---

## Target State ✅

```rust
// JWT Auth (SECURE)
pub struct JwtAuth {
    secret_key: SecretKey,
    token_lifetime: Duration,
    roles: HashMap<String, Role>,
}

// Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...
```

**Benefits**:
1. ✅ Credentials sent ONCE (login)
2. ✅ Tokens expire (configurable)
3. ✅ Role-based permissions
4. ✅ Token revocation support
5. ✅ Full audit trail
6. ✅ Stateless (no session storage)

---

## Implementation Plan (2 Weeks)

### Week 1: Core JWT Infrastructure

#### Day 1-2: Dependencies & JWT Types
```toml
[dependencies]
jsonwebtoken = "9.2"        # JWT encoding/decoding
serde = { version = "1.0", features = ["derive"] }
chrono = "0.4"              # Timestamps
hmac = "0.12"               # HMAC for signing
sha2 = "0.10"               # SHA-256 for keys
```

```rust
// crates/rpc/src/jwt/mod.rs
pub mod claims;
pub mod token;
pub mod middleware;
pub mod roles;
```

#### Day 3-4: JWT Token Generation
```rust
// crates/rpc/src/jwt/token.rs
pub struct JwtToken {
    pub token: String,
    pub expires_at: i64,
    pub token_type: String,  // "Bearer"
}

pub struct TokenGenerator {
    secret: SecretKey,
    issuer: String,
    audience: String,
}

impl TokenGenerator {
    pub fn generate(&self, user: &User, lifetime: Duration) -> Result<JwtToken>;
    pub fn verify(&self, token: &str) -> Result<Claims>;
    pub fn refresh(&self, token: &str) -> Result<JwtToken>;
}
```

#### Day 5-7: Claims & RBAC
```rust
// crates/rpc/src/jwt/claims.rs
#[derive(Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,        // Subject (user ID)
    pub iat: i64,           // Issued at
    pub exp: i64,           // Expiration
    pub nbf: i64,           // Not before
    pub iss: String,        // Issuer
    pub aud: String,        // Audience
    pub roles: Vec<Role>,   // User roles
    pub permissions: Vec<Permission>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub enum Role {
    Admin,      // Full access
    Miner,      // Mining + query
    ReadOnly,   // Query only
    Custom(String),
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub enum Permission {
    // Wallet operations
    WalletCreate,
    WalletSign,
    WalletQuery,

    // Mining operations
    MineBlock,
    SubmitBlock,

    // Query operations
    QueryBlock,
    QueryTransaction,
    QueryMempool,

    // Admin operations
    AdminShutdown,
    AdminConfig,
    AdminUsers,
}
```

### Week 2: Integration & Testing

#### Day 8-9: Authentication Endpoints
```rust
// POST /auth/login
pub async fn login(
    username: String,
    password: String,
) -> Result<LoginResponse>;

// POST /auth/refresh
pub async fn refresh_token(
    refresh_token: String,
) -> Result<RefreshResponse>;

// POST /auth/logout
pub async fn logout(
    token: String,
) -> Result<()>;

// GET /auth/verify
pub async fn verify_token(
    token: String,
) -> Result<Claims>;
```

#### Day 10-11: Middleware & Authorization
```rust
// crates/rpc/src/jwt/middleware.rs
pub struct JwtMiddleware {
    token_generator: TokenGenerator,
    revoked_tokens: Arc<RwLock<HashSet<String>>>,
}

impl JwtMiddleware {
    pub fn authenticate(&self, auth_header: &str) -> Result<Claims>;
    pub fn authorize(&self, claims: &Claims, permission: Permission) -> Result<()>;
    pub fn revoke_token(&self, token: &str) -> Result<()>;
}
```

#### Day 12-13: Migration & Testing
```rust
// Backward compatibility layer
pub enum AuthMethod {
    Basic(RpcAuth),      // Deprecated
    Bearer(JwtAuth),     // Preferred
}

// Hybrid support during migration
fn authenticate_request(headers: &[String], auth: &AuthMethod) -> Result<AuthContext>;
```

#### Day 14: Documentation & Examples

---

## Detailed Implementation

### Phase 1: JWT Core (Day 1-4)

#### 1.1 Create JWT Module Structure
```bash
mkdir -p crates/rpc/src/jwt
touch crates/rpc/src/jwt/mod.rs
touch crates/rpc/src/jwt/token.rs
touch crates/rpc/src/jwt/claims.rs
touch crates/rpc/src/jwt/roles.rs
touch crates/rpc/src/jwt/middleware.rs
```

#### 1.2 Add Dependencies
```toml
[dependencies]
jsonwebtoken = "9.2"
serde = { version = "1.0", features = ["derive"] }
chrono = "0.4"
```

#### 1.3 Implement Token Generator
```rust
// crates/rpc/src/jwt/token.rs
use jsonwebtoken::{encode, decode, Header, Validation, EncodingKey, DecodingKey, Algorithm};
use serde::{Serialize, Deserialize};
use chrono::{Utc, Duration};

pub struct TokenGenerator {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    issuer: String,
    audience: String,
    algorithm: Algorithm,
}

impl TokenGenerator {
    pub fn new(secret: &[u8], issuer: String, audience: String) -> Self {
        Self {
            encoding_key: EncodingKey::from_secret(secret),
            decoding_key: DecodingKey::from_secret(secret),
            issuer,
            audience,
            algorithm: Algorithm::HS256,
        }
    }

    pub fn generate_token(&self, user_id: &str, roles: Vec<String>, lifetime: Duration) -> Result<String, Error> {
        let now = Utc::now();
        let claims = Claims {
            sub: user_id.to_string(),
            iat: now.timestamp(),
            exp: (now + lifetime).timestamp(),
            nbf: now.timestamp(),
            iss: self.issuer.clone(),
            aud: self.audience.clone(),
            roles,
        };

        let header = Header::new(self.algorithm);
        encode(&header, &claims, &self.encoding_key)
            .map_err(|e| Error::TokenGeneration(e.to_string()))
    }

    pub fn verify_token(&self, token: &str) -> Result<Claims, Error> {
        let mut validation = Validation::new(self.algorithm);
        validation.set_audience(&[&self.audience]);
        validation.set_issuer(&[&self.issuer]);

        decode::<Claims>(token, &self.decoding_key, &validation)
            .map(|data| data.claims)
            .map_err(|e| Error::InvalidToken(e.to_string()))
    }
}
```

### Phase 2: RBAC System (Day 5-7)

#### 2.1 Define Roles & Permissions
```rust
// crates/rpc/src/jwt/roles.rs
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Role {
    Admin,
    Miner,
    ReadOnly,
}

impl Role {
    pub fn permissions(&self) -> Vec<Permission> {
        match self {
            Role::Admin => vec![
                Permission::All,
            ],
            Role::Miner => vec![
                Permission::MineBlock,
                Permission::SubmitBlock,
                Permission::QueryBlock,
                Permission::QueryTransaction,
            ],
            Role::ReadOnly => vec![
                Permission::QueryBlock,
                Permission::QueryTransaction,
                Permission::QueryMempool,
            ],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Permission {
    All,
    MineBlock,
    SubmitBlock,
    QueryBlock,
    QueryTransaction,
    QueryMempool,
    WalletCreate,
    WalletSign,
    AdminShutdown,
}
```

#### 2.2 Authorization Middleware
```rust
// crates/rpc/src/jwt/middleware.rs
pub struct AuthorizationMiddleware {
    token_generator: Arc<TokenGenerator>,
    revoked_tokens: Arc<RwLock<HashSet<String>>>,
}

impl AuthorizationMiddleware {
    pub fn check_permission(&self, token: &str, required: Permission) -> Result<(), Error> {
        // 1. Verify token
        let claims = self.token_generator.verify_token(token)?;

        // 2. Check if revoked
        if self.is_revoked(token) {
            return Err(Error::TokenRevoked);
        }

        // 3. Check expiration
        if claims.is_expired() {
            return Err(Error::TokenExpired);
        }

        // 4. Check permission
        if !self.has_permission(&claims, &required) {
            return Err(Error::InsufficientPermissions);
        }

        Ok(())
    }

    fn has_permission(&self, claims: &Claims, required: &Permission) -> bool {
        claims.roles.iter().any(|role| {
            let role_permissions = role.permissions();
            role_permissions.contains(&Permission::All) ||
            role_permissions.contains(required)
        })
    }
}
```

### Phase 3: API Integration (Day 8-11)

#### 3.1 Authentication Endpoints
```rust
// POST /auth/login
fn handle_login(request: LoginRequest) -> Result<LoginResponse, Error> {
    // 1. Validate credentials
    let user = validate_credentials(&request.username, &request.password)?;

    // 2. Generate access token (short-lived: 1 hour)
    let access_token = token_generator.generate_token(
        &user.id,
        user.roles,
        Duration::hours(1),
    )?;

    // 3. Generate refresh token (long-lived: 30 days)
    let refresh_token = token_generator.generate_token(
        &user.id,
        vec!["refresh".to_string()],
        Duration::days(30),
    )?;

    Ok(LoginResponse {
        access_token,
        refresh_token,
        token_type: "Bearer".to_string(),
        expires_in: 3600,
    })
}
```

#### 3.2 Update RPC Methods
```rust
// Before: Basic Auth
fn dispatch_rpc(method: &str, auth: &RpcAuth) -> Result<Response> {
    if !is_authorized_basic(auth) {
        return Err(Error::Unauthorized);
    }
    // ...
}

// After: JWT Auth
fn dispatch_rpc(method: &str, token: &str, jwt_auth: &JwtMiddleware) -> Result<Response> {
    let claims = jwt_auth.authenticate(token)?;

    let required_permission = get_required_permission(method)?;
    jwt_auth.authorize(&claims, required_permission)?;

    // ...
}
```

---

## Configuration

### JWT Config
```rust
pub struct JwtConfig {
    pub secret: SecretKey,              // HS256 secret (32+ bytes)
    pub access_token_lifetime: Duration, // Default: 1 hour
    pub refresh_token_lifetime: Duration,// Default: 30 days
    pub issuer: String,                 // "bitquan-node"
    pub audience: String,               // "bitquan-rpc"
    pub algorithm: Algorithm,           // HS256 recommended
}

impl JwtConfig {
    pub fn mainnet() -> Self {
        Self {
            secret: SecretKey::from_env(),  // Must be from secure env var
            access_token_lifetime: Duration::hours(1),
            refresh_token_lifetime: Duration::days(7),  // Shorter for prod
            issuer: "bitquan-mainnet".to_string(),
            audience: "bitquan-rpc".to_string(),
            algorithm: Algorithm::HS256,
        }
    }

    pub fn devnet() -> Self {
        Self {
            access_token_lifetime: Duration::hours(24),  // Longer for dev
            refresh_token_lifetime: Duration::days(90),
            ..Self::mainnet()
        }
    }
}
```

---

## Security Considerations

### Secret Key Management
```rust
// ❌ NEVER DO THIS
let secret = "my-secret-key";  // Hard-coded

// ✅ DO THIS
let secret = std::env::var("JWT_SECRET")
    .expect("JWT_SECRET must be set");

// ✅ OR GENERATE CRYPTOGRAPHICALLY SECURE KEY
use rand::RngCore;
let mut secret = vec![0u8; 32];  // 256 bits
rand::rngs::OsRng.fill_bytes(&mut secret);
```

### Token Storage (Client-Side)
```javascript
// ❌ Don't store in localStorage (XSS vulnerable)
localStorage.setItem('token', jwt);

// ✅ Store in httpOnly cookie
document.cookie = `token=${jwt}; HttpOnly; Secure; SameSite=Strict`;

// ✅ Or in memory only (most secure)
let token = null;
```

### Token Revocation
```rust
// Maintain revocation list
pub struct TokenRevocation {
    revoked: Arc<RwLock<HashSet<String>>>,
    expiry: Arc<RwLock<HashMap<String, i64>>>,
}

impl TokenRevocation {
    pub fn revoke(&self, token: &str, expires_at: i64) {
        self.revoked.write().unwrap().insert(token.to_string());
        self.expiry.write().unwrap().insert(token.to_string(), expires_at);
    }

    pub fn is_revoked(&self, token: &str) -> bool {
        self.revoked.read().unwrap().contains(token)
    }

    pub fn cleanup_expired(&self) {
        let now = Utc::now().timestamp();
        let mut revoked = self.revoked.write().unwrap();
        let mut expiry = self.expiry.write().unwrap();

        expiry.retain(|token, exp| {
            if *exp < now {
                revoked.remove(token);
                false
            } else {
                true
            }
        });
    }
}
```

---

## Migration Strategy

### Phase 1: Dual Support (Weeks 1-2)
```rust
pub enum AuthMethod {
    Basic(RpcAuth),   // Deprecated but supported
    Bearer(JwtAuth),  // Preferred
}

fn authenticate(header: &str, auth: &AuthMethod) -> Result<AuthContext> {
    if header.starts_with("Basic ") {
        warn!("⚠️  Basic Auth is deprecated. Please switch to JWT.");
        authenticate_basic(header, auth)?
    } else if header.starts_with("Bearer ") {
        authenticate_jwt(header, auth)?
    } else {
        Err(Error::InvalidAuthHeader)
    }
}
```

### Phase 2: Deprecation Warning (Week 3-4)
```rust
// Add deprecation notice
#[deprecated(since = "0.2.0", note = "Use JWT authentication instead")]
pub struct RpcAuth { ... }
```

### Phase 3: Removal (Week 5+)
```rust
// Remove Basic Auth completely
// Only JWT supported
```

---

## Testing Strategy

### Unit Tests
```rust
#[test]
fn test_token_generation() {
    let generator = TokenGenerator::new(b"test-secret", "test", "test");
    let token = generator.generate_token("user123", vec!["admin"], Duration::hours(1)).unwrap();
    assert!(!token.is_empty());
}

#[test]
fn test_token_verification() {
    let generator = TokenGenerator::new(b"test-secret", "test", "test");
    let token = generator.generate_token("user123", vec!["admin"], Duration::hours(1)).unwrap();
    let claims = generator.verify_token(&token).unwrap();
    assert_eq!(claims.sub, "user123");
}

#[test]
fn test_expired_token() {
    let generator = TokenGenerator::new(b"test-secret", "test", "test");
    let token = generator.generate_token("user123", vec![], Duration::seconds(-1)).unwrap();
    assert!(generator.verify_token(&token).is_err());
}

#[test]
fn test_permission_check() {
    let claims = Claims {
        sub: "user123".to_string(),
        roles: vec!["miner".to_string()],
        // ...
    };
    assert!(has_permission(&claims, &Permission::MineBlock));
    assert!(!has_permission(&claims, &Permission::AdminShutdown));
}
```

### Integration Tests
```rust
#[tokio::test]
async fn test_login_flow() {
    let server = spawn_test_server();

    // 1. Login
    let response = client.post("/auth/login")
        .json(&json!({
            "username": "test",
            "password": "test123"
        }))
        .send()
        .await?;

    assert_eq!(response.status(), 200);
    let body: LoginResponse = response.json().await?;
    assert!(!body.access_token.is_empty());

    // 2. Use token
    let response = client.post("/")
        .header("Authorization", format!("Bearer {}", body.access_token))
        .json(&json!({
            "jsonrpc": "2.0",
            "method": "getblockcount",
            "id": 1
        }))
        .send()
        .await?;

    assert_eq!(response.status(), 200);
}
```

---

## Performance Considerations

### Token Caching
```rust
use lru::LruCache;

pub struct TokenCache {
    cache: Arc<Mutex<LruCache<String, Claims>>>,
}

impl TokenCache {
    pub fn get_or_verify(&self, token: &str, generator: &TokenGenerator) -> Result<Claims> {
        let mut cache = self.cache.lock().unwrap();

        if let Some(claims) = cache.get(token) {
            if !claims.is_expired() {
                return Ok(claims.clone());
            }
        }

        let claims = generator.verify_token(token)?;
        cache.put(token.to_string(), claims.clone());
        Ok(claims)
    }
}
```

### Metrics
```rust
pub struct JwtMetrics {
    pub tokens_generated: AtomicU64,
    pub tokens_verified: AtomicU64,
    pub tokens_expired: AtomicU64,
    pub tokens_revoked: AtomicU64,
    pub auth_failures: AtomicU64,
}
```

---

## Documentation

### API Documentation
```markdown
# Authentication API

## Login
POST /auth/login
Content-Type: application/json

{
  "username": "user",
  "password": "pass"
}

Response:
{
  "access_token": "eyJ...",
  "refresh_token": "eyJ...",
  "token_type": "Bearer",
  "expires_in": 3600
}

## Using the token
POST /
Authorization: Bearer eyJ...
Content-Type: application/json

{
  "jsonrpc": "2.0",
  "method": "getblockcount",
  "id": 1
}
```

---

## Deliverables Checklist

### Week 1
- [ ] JWT token generation/verification
- [ ] Claims structure
- [ ] Role & Permission system
- [ ] Token expiration handling
- [ ] Unit tests (20+ tests)

### Week 2
- [ ] Login/Refresh/Logout endpoints
- [ ] Authorization middleware
- [ ] RPC method integration
- [ ] Token revocation
- [ ] Integration tests (10+ tests)

### Documentation
- [ ] API documentation
- [ ] Migration guide
- [ ] Security best practices
- [ ] Configuration examples

---

**Estimated Total Time**: 10-14 days
**Lines of Code**: ~800 new + 200 modified
**Tests**: 30+ tests
**Security Impact**: ⭐⭐⭐⭐⭐ (Critical improvement)

Ready to start? 🚀
