# BitQuan Enhanced Configuration System Design

## Current System Analysis (6/10 Rating)

### Strengths
- TOML configuration file format (human-readable)
- Separate config files for different networks (mainnet.toml, testnet.toml, etc.)
- Environment variable support for some components
- JWT secret management with secure file permissions
- Security configuration presets (Minimal, Standard, High, Maximum)

### Weaknesses
- Manual string parsing for config values (no type-safe loading)
- No central configuration validation
- Limited environment variable support
- No runtime configuration reload capability
- No configuration schema documentation
- Secrets management limited to JWT
- No environment-aware configuration (dev/test/prod)
- No documentation of all configuration options

## Proposed Enhanced Configuration System

### Architecture Overview

```
crates/config/
├── Cargo.toml                 # Dependencies: config, serde, etc.
├── src/
│   ├── lib.rs                 # Configuration provider
│   ├── types.rs              # Configuration structs
│   ├── env.rs                # Environment variable handling
│   ├── loader.rs             # Config loading & validation
│   ├── secrets.rs            # Secrets management
│   └── templates/            # Config templates
│       ├── base.toml
│       ├── development.toml
│       ├── testing.toml
│       └── production.toml
```

### 1. Type-Safe Configuration Structs

#### Core Configuration
```rust
// crates/config/src/types.rs
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// BitQuan Network Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Network identifier (mainnet, testnet, devnet)
    pub id: String,

    /// Genesis block hash
    pub genesis_hash: String,

    /// P2P network port
    #[validate(range(min = 1, max = 65535))]
    pub p2p_port: u16,

    /// RPC server port
    #[validate(range(min = 1, max = 65535))]
    pub rpc_port: u16,

    /// Bootstrap node addresses
    pub bootstrap_nodes: Vec<String>,

    /// Initial difficulty (hex format)
    #[validate(regex = r"^0x[0-9a-fA-F]{8}$")]
    pub difficulty_bits: String,

    /// Target block time in seconds
    #[validate(range(min = 1, max = 3600))]
    pub block_interval_seconds: u32,

    /// Network-specific features
    #[serde(flatten)]
    pub features: NetworkFeatures,
}

/// Network-specific feature flags
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkFeatures {
    /// Allow mining without peers (dev/test only)
    pub allow_mining_without_peers: bool,

    /// Enable fast sync
    pub fast_sync_enabled: bool,

    /// Enable checkpoint verification
    pub checkpoint_enabled: bool,
}

/// Consensus Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusConfig {
    /// ASERT difficulty adjustment half-life (seconds)
    #[validate(range(min = 1))]
    pub asert_half_life: u64,

    /// BurstGuard enabled
    pub burst_guard_enabled: bool,

    /// BurstGuard threshold multiplier
    #[validate(range(min = 0.1, max = 10.0))]
    pub burst_guard_threshold: f32,

    /// Maximum block weight
    #[validate(range(min = 1000))]
    pub max_block_weight: u32,

    /// Hybrid mining activation height
    pub hybrid_pow_activation_height: u32,

    /// Allowed mining algorithms
    pub allowed_algos: Vec<String>,
}

/// Security Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Security level preset
    #[serde(default = "default_security_level")]
    pub security_level: String,

    /// Rate limiting configuration
    pub rate_limiter: RateLimitConfig,

    /// Connection management
    pub connections: ConnectionConfig,

    /// Reputation system
    pub reputation: ReputationConfig,

    /// Ban management
    pub bans: BanConfig,

    /// DoS protection
    pub dos_protection: DosConfig,
}

/// JWT Authentication Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtConfig {
    /// JWT secret (can be reference to file)
    #[serde(default)]
    pub secret: SecretOrFile,

    /// Authentication enabled
    pub auth_enabled: bool,

    /// JWT token expiration (seconds)
    #[validate(range(min = 60, max = 86400))]
    pub token_expiry: u32,

    /// Admin user configuration
    pub admin_user: UserConfig,

    /// Additional users
    #[serde(default)]
    pub users: Vec<UserConfig>,
}

/// Storage Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Database path
    pub db_path: PathBuf,

    /// Cache size in MB
    #[validate(range(min = 32, max = 32768))]
    pub cache_size_mb: u32,

    /// Pruning configuration
    pub pruning: PruningConfig,
}

/// Logging Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level (trace, debug, info, warn, error)
    #[validate(regex = r"^(trace|debug|info|warn|error)$")]
    pub level: String,

    /// Log file path (optional)
    pub file: Option<PathBuf>,

    /// Maximum log file size (MB)
    #[validate(range(min = 1, max = 1024))]
    pub max_size_mb: u32,

    /// Maximum number of log files
    #[validate(range(min = 1, max = 100))]
    pub max_files: u32,

    /// Enable console logging
    pub console: bool,

    /// Enable JSON logging format
    pub json_format: bool,
}

/// Environment-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentConfig {
    /// Environment name (development, testing, production)
    pub env: String,

    /// Enable development-only features
    pub dev_features: bool,

    /// Enable testing-only features
    pub test_features: bool,

    /// Production hardening flags
    pub production: ProductionConfig,
}

/// Production-specific hardening
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductionConfig {
    /// Require HTTPS for all endpoints
    pub force_https: bool,

    /// Enable strict mode checks
    pub strict_mode: bool,

    /// Enable security monitoring
    pub security_monitoring: bool,

    /// Require authentication for all RPC methods
    pub rpc_auth_required: bool,
}

/// Top-level configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BitQuanConfig {
    /// Network configuration
    pub network: NetworkConfig,

    /// Consensus configuration
    pub consensus: ConsensusConfig,

    /// Security configuration
    pub security: SecurityConfig,

    /// RPC configuration
    pub rpc: RpcConfig,

    /// Mining configuration
    pub mining: MiningConfig,

    /// Wallet configuration
    pub wallet: WalletConfig,

    /// Logging configuration
    pub logging: LoggingConfig,

    /// Storage configuration
    pub storage: StorageConfig,

    /// Environment configuration
    pub env: EnvironmentConfig,

    /// Metrics and monitoring
    pub metrics: MetricsConfig,
}

/// RPC Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcConfig {
    /// RPC bind address
    pub bind: String,

    /// Authentication configuration
    pub auth: JwtConfig,

    /// TLS configuration
    pub tls: Option<TlsConfig>,
}

/// TLS Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsConfig {
    /// Enable TLS
    pub enabled: bool,

    /// Certificate file path
    pub cert_path: PathBuf,

    /// Private key file path
    pub key_path: PathBuf,

    /// CA bundle path (optional)
    pub ca_path: Option<PathBuf>,
}

/// Mining Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiningConfig {
    /// Coinbase maturity blocks
    #[validate(range(min = 1, max = 1000))]
    pub coinbase_maturity: u32,

    /// Initial block reward
    #[validate(range(min = 1))]
    pub initial_block_reward: u64,

    /// Halving interval
    #[validate(range(min = 1000, max = 840000))]
    pub halving_interval: u32,

    /// Mining threads
    pub threads: Option<u32>,
}

/// Wallet Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletConfig {
    /// Keystore KDF memory cost (KiB)
    #[validate(range(min = 16384, max = 1048576))]
    pub keystore_kdf_mem_kib: u32,

    /// Keystore KDF time cost
    #[validate(range(min = 1, max = 10))]
    pub keystore_kdf_time_cost: u32,

    /// Keystore KDF parallelism
    #[validate(range(min = 1, max = 16))]
    pub keystore_kdf_parallelism: u32,
}

/// Metrics Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    /// Enable metrics collection
    pub enabled: bool,

    /// Metrics bind address
    pub bind: Option<String>,

    /// Metrics port
    pub port: Option<u16>,

    /// Export to Prometheus
    pub prometheus_enabled: bool,

    /// Export to StatsD
    pub statsd_enabled: bool,

    /// StatsD address
    pub statsd_address: Option<String>,
}

/// Rate Limiting Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Maximum messages per time window
    pub max_messages_per_window: u32,

    /// Time window duration (seconds)
    pub window_seconds: u32,

    /// Violation threshold before ban
    pub violation_threshold: u32,
}

/// Connection Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionConfig {
    /// Maximum total connections
    pub max_total_connections: u32,

    /// Maximum inbound connections
    pub max_inbound_connections: u32,

    /// Maximum outbound connections
    pub max_outbound_connections: u32,

    /// Maximum connections per IP
    pub max_connections_per_ip: u32,
}

/// Reputation Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReputationConfig {
    /// Initial reputation score
    pub initial_score: i32,

    /// Temporary ban threshold
    pub temp_ban_threshold: i32,

    /// Permanent ban threshold
    pub perm_ban_threshold: i32,
}

/// Ban Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BanConfig {
    /// Default ban duration (seconds)
    pub default_duration: u64,

    /// Maximum ban duration (seconds)
    pub max_duration: u64,
}

/// DoS Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DosConfig {
    /// Maximum connections per second
    pub max_connections_per_second: u32,

    /// Connection flood threshold
    pub connection_flood_threshold: u32,
}

/// Pruning Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PruningConfig {
    /// Pruning enabled
    pub enabled: bool,

    /// Prune height (if static)
    pub prune_height: Option<u64>,

    /// Pruning mode (static, adaptive)
    pub mode: String,
}

/// User Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserConfig {
    /// Username
    pub username: String,

    /// Password hash (argon2)
    pub password_hash: String,

    /// User role (admin, miner, readonly)
    pub role: String,
}

/// Secret or file reference
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SecretOrFile {
    /// Direct secret value
    Value(String),
    /// Reference to file containing secret
    File(PathBuf),
}

// Helper functions
fn default_security_level() -> String {
    "standard".to_string()
}
```

### 2. Configuration Loader with Validation

```rust
// crates/config/src/loader.rs
use super::*;
use std::path::Path;
use validator::{Validate, ValidationError};
use crate::env::Environment;

pub struct ConfigLoader {
    base_path: PathBuf,
    env: Environment,
}

impl ConfigLoader {
    pub fn new(base_path: PathBuf, env: Environment) -> Self {
        Self { base_path, env }
    }

    /// Load configuration from files and environment
    pub fn load(&self) -> Result<BitQuanConfig, ConfigError> {
        // 1. Load base configuration
        let mut config = self.load_base_config()?;

        // 2. Merge environment-specific overrides
        self.merge_env_config(&mut config)?;

        // 3. Apply environment variables
        self.apply_env_vars(&mut config)?;

        // 4. Validate configuration
        config.validate()?;

        // 5. Post-load processing
        self.post_process(&mut config)?;

        Ok(config)
    }

    fn load_base_config(&self) -> Result<BitQuanConfig, ConfigError> {
        let config_path = self.base_path.join("config.toml");
        let content = std::fs::read_to_string(&config_path)
            .map_err(|e| ConfigError::FileRead(config_path.clone(), e))?;

        let config: BitQuanConfig = toml::from_str(&content)
            .map_err(|e| ConfigError::Parse(config_path.clone(), e))?;

        Ok(config)
    }

    fn merge_env_config(&self, config: &mut BitQuanConfig) -> Result<(), ConfigError> {
        let env_config_path = match self.env {
            Environment::Development => self.base_path.join("config.development.toml"),
            Environment::Testing => self.base_path.join("config.testing.toml"),
            Environment::Production => self.base_path.join("config.production.toml"),
        };

        if env_config_path.exists() {
            let content = std::fs::read_to_string(&env_config_path)?;
            let env_config: BitQuanConfig = toml::from_str(&content)?;

            // Merge environment-specific settings
            merge_toml_values(config, env_config);
        }

        Ok(())
    }

    fn apply_env_vars(&self, config: &mut BitQuanConfig) -> Result<(), ConfigError> {
        // Network overrides
        if let Some(p2p_port) = std::env::var("BQ_P2P_PORT").ok().and_then(|p| p.parse().ok()) {
            config.network.p2p_port = p2p_port;
        }

        // RPC overrides
        if let Some(bind) = std::env::var("BQ_RPC_BIND").ok() {
            config.rpc.bind = bind;
        }

        // Security overrides
        if let Some(level) = std::env::var("BQ_SECURITY_LEVEL").ok() {
            config.security.security_level = level;
        }

        // JWT secrets from environment
        if let Ok(secret) = std::env::var("BQ_JWT_SECRET") {
            config.rpc.auth.secret = SecretOrFile::Value(secret);
        }

        // Storage paths
        if let Some(data_dir) = std::env::var("BQ_DATA_DIR").ok() {
            config.storage.db_path = data_dir.into();
        }

        // Logging level
        if let Ok(level) = std::env::var("BQ_LOG_LEVEL").ok() {
            config.logging.level = level;
        }

        Ok(())
    }

    fn post_process(&self, config: &mut BitQuanConfig) -> Result<(), ConfigError> {
        // Resolve secret file references
        if let SecretOrFile::File(ref path) = config.rpc.auth.secret {
            let secret = self.read_secret_file(path)?;
            config.rpc.auth.secret = SecretOrFile::Value(secret);
        }

        // Create data directory if it doesn't exist
        let data_dir = &config.storage.db_path;
        if !data_dir.exists() {
            std::fs::create_dir_all(data_dir)
                .map_err(|e| ConfigError::DataDirCreation(data_dir.clone(), e))?;
        }

        // Apply security presets
        self.apply_security_presets(config)?;

        Ok(())
    }

    fn read_secret_file(&self, path: &Path) -> Result<String, ConfigError> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| ConfigError::SecretRead(path.to_path_buf(), e))?;

        // Trim whitespace and newlines
        let secret = content.trim().to_string();

        // Validate secret length
        if secret.len() < 32 {
            return Err(ConfigError::InvalidSecret(path.to_path_buf(), "Secret too short".to_string()));
        }

        Ok(secret)
    }

    fn apply_security_presets(&self, config: &mut BitQuanConfig) -> Result<(), ConfigError> {
        let level = config.security.security_level.to_lowercase();

        match level.as_str() {
            "minimal" => {
                // Apply minimal security preset
                config.security.security_level = "minimal".to_string();
                config.security.rate_limiter = RateLimitConfig {
                    max_messages_per_window: 50,
                    window_seconds: 60,
                    violation_threshold: 5,
                };
            }
            "standard" => {
                // Already standard
            }
            "high" => {
                // Apply high security preset
                config.security.security_level = "high".to_string();
                config.security.rate_limiter = RateLimitConfig {
                    max_messages_per_window: 200,
                    window_seconds: 60,
                    violation_threshold: 2,
                };
            }
            "maximum" => {
                // Apply maximum security preset
                config.security.security_level = "maximum".to_string();
                config.security.rate_limiter = RateLimitConfig {
                    max_messages_per_window: 500,
                    window_seconds: 60,
                    violation_threshold: 1,
                };
            }
            _ => {
                return Err(ConfigError::InvalidSecurityLevel(level));
            }
        }

        Ok(())
    }
}

/// Custom configuration error type
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Failed to read config file: {0:?}")]
    FileRead(PathBuf, #[source] std::io::Error),

    #[error("Failed to parse config: {0:?}")]
    Parse(PathBuf, #[source] toml::de::Error),

    #[error("Failed to read secret file: {0:?}")]
    SecretRead(PathBuf, #[source] std::io::Error),

    #[error("Invalid security level: {0}")]
    InvalidSecurityLevel(String),

    #[error("Invalid secret: {0}")]
    InvalidSecret(PathBuf, String),

    #[error("Failed to create data directory: {0:?}")]
    DataDirCreation(PathBuf, #[source] std::io::Error),

    #[error("Configuration validation failed: {0}")]
    Validation(#[source] ValidationError),
}

impl From<ValidationError> for ConfigError {
    fn from(err: ValidationError) -> Self {
        ConfigError::Validation(err)
    }
}
```

### 3. Environment Variable Handling

```rust
// crates/config/src/env.rs
use std::env;

/// Environment enum
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Environment {
    Development,
    Testing,
    Production,
}

impl Environment {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "dev" | "development" => Some(Environment::Development),
            "test" | "testing" => Some(Environment::Testing),
            "prod" | "production" => Some(Environment::Production),
            _ => None,
        }
    }

    pub fn detect() -> Self {
        if let Ok(env) = env::var("BQ_ENVIRONMENT") {
            Self::from_str(&env).unwrap_or(Environment::Production)
        } else {
            // Default to production if not specified
            Environment::Production
        }
    }

    pub fn is_development(&self) -> bool {
        matches!(self, Environment::Development)
    }

    pub fn is_production(&self) -> bool {
        matches!(self, Environment::Production)
    }
}

/// Environment variable helper
pub struct EnvVar;

impl EnvVar {
    pub fn get<T: std::str::FromStr>(key: &str) -> Option<T> {
        env::var(key)
            .ok()
            .and_then(|v| v.parse().ok())
    }

    pub fn get_string(key: &str) -> Option<String> {
        env::var(key).ok()
    }

    pub fn get_bytes(key: &str) -> Option<Vec<u8>> {
        env::var(key)
            .ok()
            .and_then(|v| hex::decode(v).ok())
    }

    pub fn get_bool(key: &str) -> bool {
        self::get_string(key).map_or(false, |v| {
            matches!(v.to_lowercase().as_str(), "1" | "true" | "yes" | "on")
        })
    }
}
```

### 4. Configuration Provider

```rust
// crates/config/src/lib.rs
pub mod types;
pub mod loader;
pub mod env;
pub mod secrets;

use std::sync::Arc;
use tokio::sync::RwLock;

pub use types::*;
pub use loader::*;
pub use env::*;
use loader::ConfigLoader;

/// Configuration provider with hot-reload capability
pub struct ConfigProvider {
    config: Arc<RwLock<BitQuanConfig>>,
    loader: ConfigLoader,
    watch_handle: Option<tokio::task::JoinHandle<()>>,
}

impl ConfigProvider {
    /// Create new configuration provider
    pub fn new(base_path: PathBuf, env: Environment) -> Result<Self, ConfigError> {
        let loader = ConfigLoader::new(base_path, env);
        let config = Arc::new(RwLock::new(loader.load()?));

        Ok(Self {
            config,
            loader,
            watch_handle: None,
        })
    }

    /// Get current configuration
    pub async fn get(&self) -> BitQuanConfig {
        self.config.read().await.clone()
    }

    /// Reload configuration
    pub async fn reload(&self) -> Result<(), ConfigError> {
        let new_config = self.loader.load()?;
        *self.config.write().await = new_config;
        Ok(())
    }

    /// Watch for configuration changes (hot-reload)
    pub fn start_watching(&mut self, interval: std::time::Duration) {
        let provider = self.config.clone();
        let loader = self.loader.clone();

        let handle = tokio::spawn(async move {
            let mut last_hash = String::new();

            loop {
                tokio::time::sleep(interval).await;

                if let Ok(config) = loader.load() {
                    let current_hash = serde_json::to_string(&config).unwrap_or_default();

                    if current_hash != last_hash {
                        *provider.write().await = config;
                        last_hash = current_hash;
                        log::info!("Configuration hot-reloaded");
                    }
                }
            }
        });

        self.watch_handle = Some(handle);
    }

    /// Stop watching for changes
    pub fn stop_watching(&mut self) {
        if let Some(handle) = self.watch_handle.take() {
            handle.abort();
        }
    }
}

/// Global configuration provider instance
static CONFIG_PROVIDER: once_cell::sync::OnceCell<tokio::sync::OnceCell<ConfigProvider>> =
    once_cell::sync::OnceCell::new();

/// Get global configuration provider
pub async fn get_config() -> Arc<RwLock<BitQuanConfig>> {
    let provider = CONFIG_PROVIDER.get_or_init(|| {
        tokio::sync::OnceCell::new()
    });

    let provider = provider.get_or_init(|| async {
        let base_path = std::env::var("BQ_CONFIG_DIR")
            .ok()
            .map(|p| PathBuf::from(p))
            .unwrap_or_else(|| std::env::current_dir().unwrap());

        let env = Environment::detect();
        ConfigProvider::new(base_path, env)
            .expect("Failed to initialize configuration provider")
    }).await;

    provider.config.clone()
}
```

### 5. Secrets Management System

```rust
// crates/config/src/secrets.rs
use std::path::PathBuf;
use super::types::SecretOrFile;

/// Secrets manager for handling sensitive configuration
pub struct SecretsManager {
    key: Vec<u8>,
}

impl SecretsManager {
    /// Create new secrets manager with encryption key
    pub fn new(key: Vec<u8>) -> Self {
        Self { key }
    }

    /// Encrypt a secret value
    pub fn encrypt(&self, plaintext: &str) -> Result<String, String> {
        use aes_gcm::{
            aead::{Aead, AeadCore},
            Aes256Gcm, Key, Nonce,
        };

        // Use key as AES-256 key
        let key = Key::<Aes256Gcm>::from_slice(&self.key);
        let cipher = Aes256Gcm::new(key);

        // Generate random nonce
        let nonce = Aes256Gcm::generate_nonce(&mut rand::thread_rng());

        // Encrypt
        let ciphertext = cipher.encrypt(&nonce, plaintext.as_bytes())
            .map_err(|e| format!("Encryption failed: {}", e))?;

        // Return nonce + ciphertext in base64
        let mut result = nonce.to_vec();
        result.extend_from_slice(&ciphertext);

        Ok(base64::encode(result))
    }

    /// Decrypt a secret value
    pub fn decrypt(&self, ciphertext: &str) -> Result<String, String> {
        use aes_gcm::{Aead, AeadCore, Key, Nonce};

        let data = base64::decode(ciphertext)
            .map_err(|e| format!("Base64 decode failed: {}", e))?;

        if data.len() < 12 {
            return Err("Invalid ciphertext: too short".to_string());
        }

        // Split nonce and ciphertext
        let (nonce_bytes, ciphertext_bytes) = data.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);

        // Decrypt
        let key = Key::<Aes256Gcm>::from_slice(&self.key);
        let cipher = Aes256Gcm::new(key);

        let plaintext = cipher.decrypt(nonce, ciphertext_bytes)
            .map_err(|e| format!("Decryption failed: {}", e))?;

        String::from_utf8(plaintext)
            .map_err(|e| format!("UTF-8 conversion failed: {}", e))
    }

    /// Create secret from file with encrypted storage
    pub fn create_secret_from_file(
        &self,
        file_path: &PathBuf,
        storage_path: &PathBuf,
    ) -> Result<String, String> {
        let content = std::fs::read_to_string(file_path)
            .map_err(|e| format!("Failed to read secret file: {}", e))?;

        let secret = content.trim().to_string();
        let encrypted = self.encrypt(&secret)?;

        std::fs::write(storage_path, encrypted)
            .map_err(|e| format!("Failed to write encrypted secret: {}", e))?;

        Ok(secret)
    }

    /// Load secret from encrypted file
    pub fn load_secret_from_file(&self, file_path: &PathBuf) -> Result<String, String> {
        let encrypted = std::fs::read_to_string(file_path)
            .map_err(|e| format!("Failed to read encrypted secret: {}", e))?;

        self.decrypt(&encrypted)
    }
}

/// Environment variable-based secrets
pub struct EnvSecrets;

impl EnvSecrets {
    pub fn get_jwt_secret() -> Option<String> {
        std::env::var("BQ_JWT_SECRET").ok()
    }

    pub fn get_rpc_password() -> Option<String> {
        std::env::var("BQ_RPC_PASSWORD").ok()
    }

    pub fn get_api_key() -> Option<String> {
        std::env::var("BQ_API_KEY").ok()
    }
}
```

### 6. Configuration Templates

```toml
# config/templates/base.toml
# Base configuration for all environments

[network]
id = "testnet"
genesis_hash = "0000000000000000000000000000000000000000000000000000000000000000"
p2p_port = 18444
rpc_port = 18332
bootstrap_nodes = []
difficulty_bits = "0x1d00ffff"
block_interval_seconds = 600

[consensus]
asert_half_life = 172800
burst_guard_enabled = true
burst_guard_threshold = 1.5
max_block_weight = 4000000
hybrid_pow_activation_height = 10000
allowed_algos = ["sha256d", "randomx", "ethash"]

[rpc]
bind = "0.0.0.0:18332"
auth_enabled = false

[security.security_level]
value = "standard"

[storage]
db_path = "./data/chainstate"
cache_size_mb = 512

[logging]
level = "info"
console = true
json_format = false
max_size_mb = 100
max_files = 10

[metrics]
enabled = true
prometheus_enabled = true
statsd_enabled = false

[production]
force_https = false
strict_mode = true
security_monitoring = true
rpc_auth_required = true
```

```toml
# config/templates/development.toml
# Development-specific overrides

[env]
env = "development"
dev_features = true
test_features = false

[security.security_level]
value = "minimal"

[logging]
level = "debug"
console = true

[production.force_https]
value = false

[network]
allow_mining_without_peers = true
fast_sync_enabled = false
```

```toml
# config/templates/production.toml
# Production-specific overrides

[env]
env = "production"
dev_features = false
test_features = false

[security.security_level]
value = "high"

[logging]
level = "info"
console = false
file = { value = "/var/log/bitquan/node.log" }

[production.force_https]
value = true

[rpc]
auth_enabled = true

[storage]
cache_size_mb = 2048
```

### 7. Configuration API Usage Example

```rust
// Usage example
use bitquan_config::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize configuration provider
    let mut provider = ConfigProvider::new(
        std::env::current_dir().unwrap(),
        Environment::detect(),
    )?;

    // Start watching for configuration changes
    provider.start_watching(std::time::Duration::from_secs(30));

    // Get configuration
    let config = provider.get().await;

    // Access configuration values
    log::info!("Node running on {}:{}", config.network.p2p_port, config.network.p2p_port);
    log::info!("Security level: {}", config.security.security_level);

    // Configuration validation at startup
    match config.validate() {
        Ok(_) => log::info!("Configuration validated successfully"),
        Err(e) => {
            log::error!("Configuration validation failed: {}", e);
            return Err(e.into());
        }
    }

    // Later, reload configuration on change
    if let Err(e) = provider.reload().await {
        log::error!("Failed to reload configuration: {}", e);
    }

    // Shutdown
    provider.stop_watching();

    Ok(())
}
```

### 8. Multi-Environment Deployment

```bash
# Environment-specific scripts
# scripts/setup-dev.sh
export BQ_ENVIRONMENT=development
export BQ_CONFIG_DIR="./config"
export BQ_DATA_DIR="./data/dev"
export BQ_P2P_PORT=18444
export BQ_RPC_PORT=18332

# scripts/setup-prod.sh
export BQ_ENVIRONMENT=production
export BQ_CONFIG_DIR="/etc/bitquan"
export BQ_DATA_DIR="/var/lib/bitquan"
export BQ_P2P_PORT=8333
export BQ_RPC_PORT=8332
export BQ_FORCE_HTTPS=true
export BQ_RPC_AUTH_REQUIRED=true
```

```yaml
# Docker Compose environment-specific configuration
version: '3.8'

services:
  bitquan-node-dev:
    image: bitquan:latest
    environment:
      - BQ_ENVIRONMENT=development
      - BQ_CONFIG_DIR=/app/config
      - BQ_DATA_DIR=/app/data
      - BQ_LOG_LEVEL=debug
    volumes:
      - ./config/templates:/app/config
      - ./data/dev:/app/data
    ports:
      - "18444:18444"
      - "18332:18332"

  bitquan-node-prod:
    image: bitquan:latest
    environment:
      - BQ_ENVIRONMENT=production
      - BQ_CONFIG_DIR=/etc/bitquan
      - BQ_DATA_DIR=/var/lib/bitquan
      - BQ_LOG_LEVEL=info
      - BQ_RPC_AUTH_REQUIRED=true
    volumes:
      - ./config/prod:/etc/bitquan
      - /var/lib/bitquan:/var/lib/bitquan
    ports:
      - "8333:8333"
      - "8332:8332"
    security_opt:
      - no-new-privileges:true
    read_only: true
```

### 9. Configuration Documentation Generator

```rust
// Generate documentation from configuration schema
pub fn generate_docs() -> String {
    let config = BitQuanConfig::default();

    format!(
        "# BitQuan Configuration Reference\n\n\
        ## Network Configuration\n\
        {}\n\n\
        ## Security Configuration\n\
        {}\n\n\
        ## Consensus Configuration\n\
        {}\n\n\
        ## RPC Configuration\n\
        {}\n\n\
        ## Logging Configuration\n\
        {}",
        generate_network_docs(&config.network),
        generate_security_docs(&config.security),
        generate_consensus_docs(&config.consensus),
        generate_rpc_docs(&config.rpc),
        generate_logging_docs(&config.logging)
    )
}

fn generate_network_docs(network: &NetworkConfig) -> String {
    format!(
        "### Network Settings\n\
        - **ID**: Network identifier (mainnet, testnet, devnet)\n\
        - **Genesis Hash**: 64-character hex string for genesis block\n\
        - **P2P Port**: Port for peer-to-peer networking (1-65535)\n\
        - **RPC Port**: Port for JSON-RPC API (1-65535)\n\
        - **Bootstrap Nodes**: List of bootstrap node addresses\n\
        - **Difficulty Bits**: Initial difficulty in hex format (0xXXXXXXXX)\n\
        - **Block Interval**: Target time between blocks in seconds\n\
        \n\
        ### Environment-Specific Features\n\
        - **Mining Without Peers**: Allow mining without connected peers (dev/test only)\n\
        - **Fast Sync**: Enable fast synchronization mode\n\
        - **Checkpoints**: Enable block checkpoint verification\n"
    )
}
```

### 10. Migration Guide

```bash
# Migration script
#!/bin/bash
# scripts/migrate-config.sh

echo "Migrating BitQuan configuration..."

# Create new config directory
mkdir -p config/templates

# Copy existing config files
cp config/bitquan.toml config/config.toml
cp config/mainnet.toml config/templates/
cp config/testnet.toml config/templates/

# Generate environment-specific files
python3 -c "
import toml

# Load base config
with open('config/config.toml', 'r') as f:
    config = toml.load(f)

# Generate development config
dev_config = config.copy()
dev_config['env'] = {'env': 'development', 'dev_features': True}
dev_config['security']['security_level'] = 'minimal'

# Generate production config
prod_config = config.copy()
prod_config['env'] = {'env': 'production', 'dev_features': False}
prod_config['security']['security_level'] = 'high'
prod_config['rpc']['auth_enabled'] = True

# Write new configs
with open('config/development.toml', 'w') as f:
    toml.dump(dev_config, f)

with open('config/production.toml', 'w') as f:
    toml.dump(prod_config, f)
"

echo "Migration complete!"
echo "New configuration structure:"
echo "- config/config.toml (base configuration)"
echo "- config/development.toml (development overrides)"
echo "- config/production.toml (production overrides)"
echo "- config/templates/ (reference configurations)"
```

## Implementation Plan

1. **Phase 1**: Create configuration crate with basic types and validation
   - Implement core configuration structs
   - Add validation using `validator` crate
   - Create basic configuration loader

2. **Phase 2**: Add environment and secrets management
   - Implement environment variable handling
   - Add secrets management system
   - Create configuration templates

3. **Phase 3**: Add hot-reload capability
   - Implement file watching
   - Add runtime configuration updates
   - Add graceful reload mechanism

4. **Phase 4**: Documentation and migration
   - Generate configuration documentation
   - Create migration scripts
   - Update deployment guides

5. **Phase 5**: Testing and validation
   - Add comprehensive unit tests
   - Add integration tests
   - Performance testing for configuration loading

## Benefits

1. **Type Safety**: Compile-time validation with Rust type system
2. **Environment Support**: Easy switching between dev/test/prod
3. **Hot Reload**: Runtime configuration updates without restart
4. **Secret Management**: Secure handling of sensitive data
5. **Documentation**: Self-documenting configuration
6. **Validation**: Comprehensive configuration validation
7. **Migration**: Smooth upgrade path from existing config
8. **Monitoring**: Built-in metrics and logging support

This enhanced configuration system will provide a robust, secure, and maintainable foundation for BitQuan's configuration management, addressing all current weaknesses while adding modern features needed for production deployment.