//! # BitQuan Wallet Library
//! 
//! A secure, high-performance wallet implementation for BitQuan cryptocurrency.
//! 
//! ## Features
//! 
//! - **Post-Quantum Security**: Uses Dilithium signatures for future-proof security
//! - **Adaptive Performance**: Automatically optimizes KDF parameters based on hardware
//! - **Secure Key Caching**: Fast decryption with memory-safe caching
//! - **Production Ready**: Comprehensive configuration and monitoring
//! 
//! ## Quick Start
//! 
//! ```rust
//! use bitquan_wallet::keystore::{encrypt_keystore_adaptive, decrypt_keystore};
//! 
//! // Encrypt with adaptive parameters (recommended for most users)
//! let keystore = encrypt_keystore_adaptive(
//!     b"my secret data",
//!     "my strong password",
//!     None, // no metadata
//! );
//! 
//! // Decrypt (automatically uses cache for performance)
//! let decrypted = decrypt_keystore(&keystore, "my strong password")?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//! 
//! ## Performance Optimization
//! 
//! The wallet automatically optimizes performance based on your hardware:
//! - **Low-end devices**: Reduced KDF parameters for faster encryption
//! - **High-end servers**: Maximum security parameters
//! - **Hot cache paths**: ~1.85µs decryption (5,400x faster than cold)
//! 
//! ## Advanced Configuration
//! 
//! For server or mobile deployments, use `WalletConfig`:
//! 
//! ```rust
//! use bitquan_wallet::keystore::{WalletConfig, encrypt_keystore_with_config};
//! use std::time::Duration;
//! 
//! // Server configuration (high security, no caching)
//! let server_config = WalletConfig::server();
//! 
//! // Mobile configuration (balanced security/performance)
//! let mobile_config = WalletConfig::mobile()
//!     .with_cache_timeout(Duration::from_secs(60)); // Short cache
//! 
//! let keystore = encrypt_keystore_with_config(
//!     b"sensitive data",
//!     "password",
//!     None,
//!     &mobile_config,
//! )?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//! 
//! ## Security Features
//! 
//! - **Memory Safety**: All secrets are zeroized when dropped
//! - **Cache Isolation**: Each password/salt combination has isolated cache
//! - **Timeout Enforcement**: Cache entries expire after 5 minutes by default
//! - **Post-Quantum**: Uses Dilithium for signature security
//! 
//! ## Monitoring
//! 
//! Monitor cache usage in production:
//! 
//! ```rust
//! use bitquan_wallet::keystore::{get_cache_stats, get_cache_memory_usage};
//! 
//! let stats = get_cache_stats();
//! let memory_bytes = get_cache_memory_usage();
//! 
//! println!("Cache entries: {}", stats.active_entries);
//! println!("Memory usage: {} bytes", memory_bytes);
//! ```
//! 
//! ## Choosing the Right API
//! 
//! ### For Most Users
//! - `encrypt_keystore_adaptive()` - Automatically optimizes for your hardware
//! - `decrypt_keystore()` - Fast, secure decryption with caching
//! 
//! ### For Server/Infrastructure
//! - `WalletConfig::server()` - Maximum security, no caching
//! - `encrypt_keystore_with_config()` - Full control over parameters
//! 
//! ### For Mobile Applications
//! - `WalletConfig::mobile()` - Balanced for battery life
//! - Short cache timeouts to preserve memory
//! 
//! ### For High-Security Applications
//! - `WalletConfig::conservative()` - Maximum KDF parameters
//! - Consider disabling caching for sensitive operations

pub mod backup;
pub mod keystore;
pub mod multisig;