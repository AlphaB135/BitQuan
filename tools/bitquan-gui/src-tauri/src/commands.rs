// Import wallet functionality from local crates
use wallet::keystore::{encrypt_keystore_adaptive, decrypt_keystore_with_config, 
    WalletConfig, get_cache_stats, clear_key_cache};
use bq_crypto::wallet::{SecurePrivateKey, SecureString};
use pqc_dilithium_seeded::Keypair;
// use bitquan_types::{Transaction, Sighash}; // Not used yet
use serde::{Deserialize, Serialize};
use tauri::{command, State};
use std::sync::Mutex;

// Thread-safe wallet state
pub struct WalletState {
    unlocked_key: Option<SecurePrivateKey>,
    keystore_data: Option<EncryptedKeystoreData>,
}

impl Default for WalletState {
    fn default() -> Self {
        Self {
            unlocked_key: None,
            keystore_data: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedKeystoreData {
    pub address: String,
    pub encrypted_data: String,
    pub created_at: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WalletCreateRequest {
    pub password: String,
    pub address_hint: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WalletCreateResponse {
    pub success: bool,
    pub keystore_data: Option<EncryptedKeystoreData>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WalletUnlockRequest {
    pub keystore_data: EncryptedKeystoreData,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WalletUnlockResponse {
    pub success: bool,
    pub address: String,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TransactionSignRequest {
    pub sighash_hex: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TransactionSignResponse {
    pub success: bool,
    pub signature_hex: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WalletStatusResponse {
    pub is_locked: bool,
    pub address: Option<String>,
    pub cache_stats: CacheStatsResponse,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CacheStatsResponse {
    pub active_entries: usize,
    pub total_entries: usize,
    pub memory_usage_bytes: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RawTransactionRequest {
    pub tx_hex: String,
    pub max_fee_rate: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RawTransactionResponse {
    pub success: bool,
    pub txid: Option<String>,
    pub error: Option<String>,
}

/// Create new encrypted wallet with adaptive KDF
#[command]
pub async fn create_wallet(
    request: WalletCreateRequest,
    state: State<'_, Mutex<WalletState>>,
) -> Result<WalletCreateResponse, String> {
    // Generate new PQC keypair
    let keypair = Keypair::generate();
    
    // Convert to secure format
    let private_key = SecurePrivateKey::new(keypair.expose_secret().to_vec());
    let password = SecureString::new(request.password.clone());
    
    // Create address from public key (simplified for demo)
    let address = request.address_hint.clone().unwrap_or_else(|| {
        format!("bq1{}", hex::encode(&keypair.public[..20]))
    });
    
    // Encrypt with adaptive parameters
    let keystore = encrypt_keystore_adaptive(
        private_key.as_slice(),
        &request.password,
        Some(serde_json::json!({
            "algorithm": "dilithium3",
            "created_by": "bitquan-wallet"
        }))
    );
    
    // Store encrypted data
    let keystore_data = EncryptedKeystoreData {
        address: address.clone(),
        encrypted_data: serde_json::to_string(&keystore)
            .map_err(|e| format!("Failed to serialize keystore: {}", e))?,
        created_at: keystore.created,
    };
    
    // Update state
    {
        let mut state_guard = state.lock().map_err(|e| format!("State lock error: {}", e))?;
        state_guard.keystore_data = Some(keystore_data.clone());
        state_guard.unlocked_key = None;
    }
    
    Ok(WalletCreateResponse {
        success: true,
        keystore_data: Some(keystore_data),
        error: None,
    })
}

/// Unlock wallet (decrypt private key into secure memory)
#[command]
pub async fn unlock_wallet(
    request: WalletUnlockRequest,
    state: State<'_, Mutex<WalletState>>,
) -> Result<WalletUnlockResponse, String> {
    // Parse keystore
    let keystore: wallet::keystore::KeystoreFile = serde_json::from_str(&request.keystore_data.encrypted_data)
        .map_err(|e| format!("Invalid keystore format: {}", e))?;
    
    // Decrypt with secure caching
    let config = WalletConfig::performance();
    
    let decrypted = decrypt_keystore_with_config(&keystore, &request.password, &config)
        .map_err(|e| format!("Failed to decrypt keystore: {}", e))?;
    
    // Store in secure memory
    let private_key = SecurePrivateKey::new(decrypted);
    
    let address = request.keystore_data.address.clone();
    
    // Update state
    {
        let mut state_guard = state.lock().map_err(|e| format!("State lock error: {}", e))?;
        state_guard.unlocked_key = Some(private_key);
        state_guard.keystore_data = Some(request.keystore_data);
    }
    
    Ok(WalletUnlockResponse {
        success: true,
        address,
        error: None,
    })
}

/// Sign transaction with PQC (private key stays in Rust!)
#[command]
pub async fn sign_transaction(
    request: TransactionSignRequest,
    state: State<'_, Mutex<WalletState>>,
) -> Result<TransactionSignResponse, String> {
    // Get unlocked key from state
    let private_key = {
        let state_guard = state.lock().map_err(|e| format!("State lock error: {}", e))?;
        match &state_guard.unlocked_key {
            Some(key) => key.clone(),
            None => return Ok(TransactionSignResponse {
                success: false,
                signature_hex: None,
                error: Some("Wallet is locked. Please unlock first.".to_string()),
            }),
        }
    };
    
    // Parse sighash
    let sighash_bytes = hex::decode(&request.sighash_hex)
        .map_err(|e| format!("Invalid sighash hex: {}", e))?;
    
    if sighash_bytes.len() != 32 {
        return Ok(TransactionSignResponse {
            success: false,
            signature_hex: None,
            error: Some("Sighash must be 32 bytes".to_string()),
        });
    }
    
    // Create Dilithium3 signer from private key
    let key_bytes = private_key.as_slice();
    if key_bytes.len() != 32 {
        return Ok(TransactionSignResponse {
            success: false,
            signature_hex: None,
            error: Some("Invalid private key length".to_string()),
        });
    }
    
    let mut secret_bytes = [0u8; 32];
    secret_bytes.copy_from_slice(&key_bytes[..32]);
    
    // Reconstruct keypair from secret key (simplified - in production this would store the full keypair)
    let keypair = Keypair::generate(); // This should be improved to use the actual secret
    
    // Sign sighash
    let signature = keypair.sign(&sighash_bytes);
    
    // Return hex-encoded signature
    Ok(TransactionSignResponse {
        success: true,
        signature_hex: Some(hex::encode(signature)),
        error: None,
    })
}

/// Get wallet status
#[command]
pub async fn get_wallet_status(
    state: State<'_, Mutex<WalletState>>,
) -> Result<WalletStatusResponse, String> {
    let state_guard = state.lock().map_err(|e| format!("State lock error: {}", e))?;
    
    let cache_stats = get_cache_stats();
    
    Ok(WalletStatusResponse {
        is_locked: state_guard.unlocked_key.is_none(),
        address: state_guard.keystore_data.as_ref().map(|k| k.address.clone()),
        cache_stats: CacheStatsResponse {
            active_entries: cache_stats.active_entries,
            total_entries: cache_stats.total_entries,
            memory_usage_bytes: get_cache_memory_usage(),
        },
    })
}

/// Lock wallet (clear private key from memory)
#[command]
pub async fn lock_wallet(
    state: State<'_, Mutex<WalletState>>,
) -> Result<bool, String> {
    {
        let mut state_guard = state.lock().map_err(|e| format!("State lock error: {}", e))?;
        state_guard.unlocked_key = None;
    }
    
    Ok(true)
}

/// Clear key cache (security operation)
#[command]
pub async fn clear_cache() -> Result<bool, String> {
    clear_key_cache();
    Ok(true)
}

/// Get cache memory usage
fn get_cache_memory_usage() -> usize {
    wallet::keystore::get_cache_memory_usage()
}

/// Broadcast signed transaction to BitQuan network
#[command]
pub async fn send_raw_transaction(
    _request: RawTransactionRequest,
) -> Result<RawTransactionResponse, String> {
    // For now, return a mock response
    // In a real implementation, this would connect to the RPC server
    // TODO: Implement actual RPC client integration
    Ok(RawTransactionResponse {
        success: true,
        txid: Some("mock_txid_1234567890abcdef".to_string()),
        error: None,
    })
}