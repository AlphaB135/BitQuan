//! RPC and JWT commands for BitQuan CLI
//!
//! This module contains all RPC/JWT-related commands:
//! - run_rpc_server
//! - submit_transaction_rpc
//! - jwt_user_add, jwt_user_remove, jwt_user_list
//! - generate_self_signed_cert_cli, hash_password_cli

use crate::cli::{invalid, read_password_from_stdin};
use crate::commands::p2p::get_or_create_jwt_secret;
use bitquan_types::error::{Error, Result};
use serde_json::json;
use std::path::Path;

// Helper function to get or create JWT secret

/// Run RPC server with authentication
#[allow(clippy::too_many_arguments)]
pub fn run_rpc_server(
    handler: crate::rpc::NodeRpcHandler,
    addr: String,
    jwt_config: Option<String>,
    jwt_secret: Option<String>,
    rpc_config: bitquan_rpc::RpcConfig,
    tls_config: Option<bitquan_rpc::tls::TlsConfig>,
    username: String,
    password: String,
    require_tls: bool,
    datadir: String, // For JWT secret generation
) {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap_or_else(|e| {
            eprintln!("failed to build RPC runtime: {}", e);
            std::process::exit(1);
        });

    rt.block_on(async move {
        // JWT authentication (required)
        let jwt_auth = if let Some(config_path) = jwt_config {
            println!("Loading JWT config from: {}", config_path);
            match bitquan_rpc::jwt::JwtConfig::from_file(&config_path) {
                Ok(config) => match bitquan_rpc::jwt::JwtAuth::from_config(&config) {
                    Ok(auth) => auth,
                    Err(e) => {
                        eprintln!("Failed to create JWT auth from config: {}", e);
                        return;
                    }
                },
                Err(e) => {
                    eprintln!("Failed to load JWT config: {}", e);
                    return;
                }
            }
        } else if let Some(secret) = jwt_secret {
            println!("Using JWT with provided secret");
            bitquan_rpc::jwt::JwtAuth::new(&secret)
        } else {
            // SECURITY FIX: Generate or load secure JWT secret instead of using dummy
            println!("WARNING: No JWT secret provided. Generating secure secret...");
            let generated_secret = get_or_create_jwt_secret(&datadir).unwrap_or_else(|e| {
                eprintln!("FATAL: Failed to generate JWT secret: {}", e);
                eprintln!("Cannot start RPC server without authentication!");
                std::process::exit(1);
            });
            bitquan_rpc::jwt::JwtAuth::new(&generated_secret)
        };

        let basic_auth = Some((username, password));

        let mut server = bitquan_rpc::server::RpcServer::new(
            handler,
            addr.clone(),
            jwt_auth,
            rpc_config,
            basic_auth,
        );

        if let Some(tls_cfg) = tls_config {
            server = server.with_tls_config(tls_cfg);
        }
        server = server.require_tls(require_tls);
        if let Err(e) = server.serve().await {
            eprintln!("RPC server error ({}): {}", addr, e);
        }
    });
}

/// Submit transaction via RPC to local node
///
/// **Note**: This function is currently unused and kept for future integration.
/// When启用, make the RPC URL configurable rather than hardcoded.
pub async fn submit_transaction_rpc(tx_hex: &str, rpc_url: Option<&str>) -> Result<String> {
    use serde_json::json;

    let rpc_url = rpc_url.unwrap_or("http://127.0.0.1:8332");
    let payload = json!({
      "jsonrpc": "2.0",
      "method": "submittransaction",
      "params": [tx_hex],
      "id": 1
    });

    let client = reqwest::Client::new();
    let response = client
        .post(rpc_url)
        .json(&payload)
        .header("Content-Type", "application/json")
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await
        .map_err(|e| Error::Invalid(format!("RPC connection failed: {}", e)))?;

    if !response.status().is_success() {
        return Err(Error::Invalid(format!(
            "RPC server returned status: {}",
            response.status()
        )));
    }

    let rpc_response: serde_json::Value = response
        .json()
        .await
        .map_err(|e| Error::Invalid(format!("failed to parse RPC response: {}", e)))?;

    if let Some(error) = rpc_response.get("error") {
        return Err(Error::Invalid(format!("RPC error: {}", error)));
    }

    let txid = rpc_response
        .get("result")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::Invalid("invalid RPC response: missing result".to_string()))?;

    Ok(txid.to_string())
}

/// Generate self-signed certificate for RPC TLS
pub fn generate_self_signed_cert_cli(output_dir: &str) -> Result<()> {
    use std::path::Path;

    let path = Path::new(output_dir);
    std::fs::create_dir_all(path).map_err(|e| {
        Error::Invalid(format!(
            "failed to create output directory {}: {e}",
            path.display()
        ))
    })?;

    bitquan_rpc::tls::generate_self_signed_cert(path).map_err(|err| {
        Error::Invalid(format!("failed to generate self-signed certificate: {err}"))
    })?;

    println!("Generated self-signed certificate:");
    println!("  cert: {}/cert.pem", path.display());
    println!("  key: {}/key.pem", path.display());
    println!();
    println!(
        "Development only. For production, obtain a trusted certificate (e.g. Let's Encrypt)."
    );
    println!();
    println!("To start the node with TLS:");
    println!(" bitquan-node p2p-server \\");
    println!("  --rpc-listen 127.0.0.1:8332 \\");
    println!("  --rpc-username admin \\");
    println!("  --rpc-password <YOUR_PASSWORD> \\");
    println!("  --rpc-tls-cert {}/cert.pem \\", path.display());
    println!("  --rpc-tls-key {}/key.pem", path.display()); // Safe: example placeholder

    Ok(())
}

/// Hash a password using Argon2id
pub fn hash_password_cli(password: Option<&str>) -> Result<()> {
    use argon2::{
        password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
        Argon2,
    };

    let password = match password {
        Some(p) => p.to_string(),
        None => {
            println!("Enter password to hash:");
            read_password_from_stdin()?
        }
    };

    if password.is_empty() {
        return invalid("Password cannot be empty");
    }

    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();

    let hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| Error::Invalid(format!("Failed to hash password: {}", e)))?
        .to_string();

    println!("\nHashed password:");
    println!("{}", hash);
    println!("\nCopy this hash to your jwt.toml file");

    Ok(())
}

/// Add a user to JWT configuration
pub fn jwt_user_add(
    config_path: &str,
    username: &str,
    role: &str,
    password: Option<&str>,
) -> Result<()> {
    use argon2::{
        password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
        Argon2,
    };
    use bitquan_rpc::jwt::{JwtConfig, JwtUserConfig};

    // Validate role
    if !["admin", "miner", "readonly"].contains(&role) {
        return invalid(format!(
            "Invalid role '{}'. Must be: admin, miner, or readonly",
            role
        ));
    }

    // Load existing config or create new
    let mut config = if Path::new(config_path).exists() {
        JwtConfig::from_file(config_path)
            .map_err(|e| Error::Invalid(format!("Failed to load config: {}", e)))?
    } else {
        JwtConfig::default()
    };

    // Check if user already exists
    if config.users.iter().any(|u| u.username == username) {
        return invalid(format!("User '{}' already exists in config", username));
    }

    // Get password
    let password = match password {
        Some(p) => p.to_string(),
        None => {
            println!("Enter password for user '{}':", username);
            read_password_from_stdin()?
        }
    };

    if password.is_empty() {
        return invalid("Password cannot be empty");
    }

    // Hash password
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| Error::Invalid(format!("Failed to hash password: {}", e)))?
        .to_string();

    // Add user
    config.users.push(JwtUserConfig {
        username: username.to_string(),
        password_hash: hash,
        role: role.to_string(),
    });

    // Save config
    config
        .save_to_file(config_path)
        .map_err(|e| Error::Invalid(format!("Failed to save config: {}", e)))?;

    println!(
        "User '{}' added successfully with role '{}'",
        username, role
    );
    println!("📄 Config saved to: {}", config_path);

    Ok(())
}

/// Remove a user from JWT configuration
pub fn jwt_user_remove(config_path: &str, username: &str) -> Result<()> {
    use bitquan_rpc::jwt::JwtConfig;

    if !Path::new(config_path).exists() {
        return invalid(format!("Config file not found: {}", config_path));
    }

    let mut config = JwtConfig::from_file(config_path)
        .map_err(|e| Error::Invalid(format!("Failed to load config: {}", e)))?;

    let initial_count = config.users.len();
    config.users.retain(|u| u.username != username);

    if config.users.len() == initial_count {
        return invalid(format!("User '{}' not found in config", username));
    }

    if config.users.is_empty() {
        return invalid("Cannot remove last user. At least one user must remain.");
    }

    config
        .save_to_file(config_path)
        .map_err(|e| Error::Invalid(format!("Failed to save config: {}", e)))?;

    println!("User '{}' removed successfully", username);
    println!("📄 Config saved to: {}", config_path);

    Ok(())
}

/// List users in JWT configuration
pub fn jwt_user_list(config_path: &str) -> Result<()> {
    use bitquan_rpc::jwt::JwtConfig;

    if !Path::new(config_path).exists() {
        return invalid(format!("Config file not found: {}", config_path));
    }

    let config = JwtConfig::from_file(config_path)
        .map_err(|e| Error::Invalid(format!("Failed to load config: {}", e)))?;

    if config.users.is_empty() {
        println!("No users found in config");
        return Ok(());
    }

    println!("\n📋 Users in {}:", config_path);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("{:<20} {:<15}", "Username", "Role");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    for user in &config.users {
        println!("{:<20} {:<15}", user.username, user.role);
    }

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Total: {} user(s)\n", config.users.len());

    Ok(())
}
