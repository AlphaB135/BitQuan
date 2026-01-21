//! Simple JWT Authentication Tests

use bitquan_rpc::jwt::{JwtAuth, JwtConfig, JwtUserConfig};

#[test]
fn test_jwt_auth_creation() {
    let mut jwt_auth = JwtAuth::new("test-secret-key");

    // Create admin user explicitly (security fix: no default users)
    jwt_auth
        .add_user_plaintext("admin", "admin123", "admin")
        .expect("Failed to create admin user");

    // Should be able to login with admin user
    let token = jwt_auth.login("admin", "admin123");
    assert!(token.is_ok(), "Login should succeed with admin user");

    let token_str = token.expect("Failed to get login token");
    assert!(!token_str.is_empty(), "Token should not be empty");

    // Verify the token
    let claims = jwt_auth.verify_token(&token_str);
    assert!(claims.is_ok(), "Token verification should succeed");

    let claims = claims.expect("Failed to verify token claims");
    assert_eq!(claims.sub, "admin");
    assert_eq!(claims.role, "admin");
    assert!(!claims.is_expired(), "Token should not be expired");
}

#[test]
fn test_jwt_auth_invalid_password() {
    let mut jwt_auth = JwtAuth::new("test-secret");

    // Create admin user explicitly
    jwt_auth
        .add_user_plaintext("admin", "admin123", "admin")
        .expect("Failed to create admin user");

    let result = jwt_auth.login("admin", "wrongpassword");
    assert!(result.is_err(), "Login should fail with wrong password");
    assert_eq!(result.unwrap_err(), "Invalid password");
}

#[test]
fn test_jwt_auth_invalid_user() {
    let jwt_auth = JwtAuth::new("test-secret");

    let result = jwt_auth.login("nonexistent", "anypassword");
    assert!(result.is_err(), "Login should fail with non-existent user");
    assert_eq!(result.unwrap_err(), "User not found");
}

#[test]
fn test_jwt_auth_add_user() {
    let mut jwt_auth = JwtAuth::new("test-secret");

    // Add a custom user
    let result = jwt_auth.add_user_plaintext("alice", "alicepass", "miner");
    assert!(result.is_ok(), "Adding user should succeed");

    // Login with the new user
    let token = jwt_auth.login("alice", "alicepass");
    assert!(token.is_ok(), "Login with new user should succeed");

    let claims = jwt_auth
        .verify_token(&token.expect("Failed to get login token"))
        .expect("Failed to verify token claims");
    assert_eq!(claims.sub, "alice");
    assert_eq!(claims.role, "miner");
}

#[test]
fn test_jwt_config_default() {
    let config = JwtConfig::default();

    assert_eq!(config.secret, "CHANGE_THIS_SECRET_IN_PRODUCTION");
    assert_eq!(config.users.len(), 1);
    assert_eq!(config.users[0].username, "admin");
    assert_eq!(config.users[0].role, "admin");
}

#[test]
fn test_jwt_from_config() {
    let config = JwtConfig {
        secret: "my-secret".to_string(),
        users: vec![
            JwtUserConfig {
                username: "testuser".to_string(),
                password_hash: "$argon2id$v=19$m=19456,t=2,p=1$c29tZXNhbHQxMjM0NTY3OA$qrvAoAhjhZvPXXZBz+JqMRRdC4mXvMr5dN3wQTu6g5E".to_string(),
                role: "admin".to_string(),
            },
        ],
    };

    let jwt_auth = JwtAuth::from_config(&config);
    assert!(
        jwt_auth.is_ok(),
        "JWT auth creation from config should succeed"
    );
}

#[test]
fn test_jwt_token_verification_fails_with_wrong_secret() {
    let mut jwt_auth1 = JwtAuth::new("secret1");
    let jwt_auth2 = JwtAuth::new("secret2");

    // Create admin user in jwt_auth1
    jwt_auth1
        .add_user_plaintext("admin", "admin123", "admin")
        .expect("Failed to create admin user");

    let token = jwt_auth1
        .login("admin", "admin123")
        .expect("Failed to login with admin credentials");

    // Token from jwt_auth1 should not verify with jwt_auth2
    let result = jwt_auth2.verify_token(&token);
    assert!(
        result.is_err(),
        "Token should not verify with different secret"
    );
}

#[test]
fn test_jwt_token_claims_structure() {
    let mut jwt_auth = JwtAuth::new("test-secret");

    // Create admin user
    jwt_auth
        .add_user_plaintext("admin", "admin123", "admin")
        .expect("Failed to create admin user");

    let token = jwt_auth
        .login("admin", "admin123")
        .expect("Failed to login with admin credentials");
    let claims = jwt_auth
        .verify_token(&token)
        .expect("Failed to verify token claims");

    // Check all required fields
    assert!(
        !claims.sub.is_empty(),
        "Subject (username) should not be empty"
    );
    assert!(!claims.role.is_empty(), "Role should not be empty");
    assert!(claims.exp > 0, "Expiration time should be set");
    assert!(claims.iat > 0, "Issued at time should be set");
    assert!(
        claims.exp > claims.iat,
        "Expiration should be after issued time"
    );
}

#[test]
fn test_jwt_admin_role_check() {
    let mut jwt_auth = JwtAuth::new("test-secret");

    // Create admin user
    jwt_auth
        .add_user_plaintext("admin", "admin123", "admin")
        .expect("Failed to create admin user");

    let token = jwt_auth
        .login("admin", "admin123")
        .expect("Failed to login with admin credentials");
    let claims = jwt_auth
        .verify_token(&token)
        .expect("Failed to verify token claims");

    assert!(claims.is_admin(), "Admin user should have admin role");
}

#[test]
fn test_jwt_refresh_token() {
    let mut jwt_auth = JwtAuth::new("test-secret");

    // Create admin user
    jwt_auth
        .add_user_plaintext("admin", "admin123", "admin")
        .expect("Failed to create admin user");

    // Login with refresh token
    let (access_token, refresh_token) = jwt_auth
        .login_with_refresh("admin", "admin123")
        .expect("Failed to login with refresh token");

    assert!(!access_token.is_empty(), "Access token should not be empty");
    assert!(
        !refresh_token.is_empty(),
        "Refresh token should not be empty"
    );

    // Verify access token
    let access_claims = jwt_auth
        .verify_token(&access_token)
        .expect("Failed to verify access token claims");
    assert_eq!(access_claims.sub, "admin");
    assert!(!access_claims.is_refresh_token());

    // Verify refresh token
    let refresh_claims = jwt_auth
        .verify_token(&refresh_token)
        .expect("Failed to verify refresh token claims");
    assert_eq!(refresh_claims.sub, "admin");
    assert!(refresh_claims.is_refresh_token());

    // Use refresh token to get new access token
    let new_access_token = jwt_auth
        .refresh_token(&refresh_token)
        .expect("Failed to refresh token");
    assert!(!new_access_token.is_empty());

    let new_claims = jwt_auth
        .verify_token(&new_access_token)
        .expect("Failed to verify new access token claims");
    assert_eq!(new_claims.sub, "admin");
    assert!(!new_claims.is_refresh_token());
}

#[test]
fn test_jwt_refresh_with_access_token_fails() {
    let mut jwt_auth = JwtAuth::new("test-secret");

    // Create admin user
    jwt_auth
        .add_user_plaintext("admin", "admin123", "admin")
        .expect("Failed to create admin user");

    // Get regular access token
    let access_token = jwt_auth
        .login("admin", "admin123")
        .expect("Failed to login with admin credentials");

    // Try to refresh with access token (should fail)
    let result = jwt_auth.refresh_token(&access_token);
    assert!(
        result.is_err(),
        "Should not be able to refresh with access token"
    );
}

#[test]
fn test_jwt_refresh_token_expiration() {
    let mut jwt_auth = JwtAuth::new("test-secret");

    // Create admin user
    jwt_auth
        .add_user_plaintext("admin", "admin123", "admin")
        .expect("Failed to create admin user");

    let (_, refresh_token) = jwt_auth
        .login_with_refresh("admin", "admin123")
        .expect("Failed to login with refresh token");

    let claims = jwt_auth
        .verify_token(&refresh_token)
        .expect("Failed to verify refresh token claims");

    // Refresh token should expire in 7 days (604800 seconds)
    let expected_lifetime = 604800;
    let actual_lifetime = claims.exp - claims.iat;

    assert!(
        (actual_lifetime - expected_lifetime).abs() < 10,
        "Refresh token lifetime should be approximately 7 days"
    );
}
