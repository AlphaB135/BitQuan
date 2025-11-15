// Import wallet functionality from local crates
use wallet::keystore::{encrypt_keystore_adaptive, decrypt_keystore_with_config, 
    WalletConfig, get_cache_stats, clear_key_cache};
use bq_crypto::wallet::{SecurePrivateKey, SecureString};
use pqc_dilithium_seeded::Keypair;
// use bitquan_types::{Transaction, Sighash}; // Not used yet
use serde::{Deserialize, Serialize};
use tauri::{command, State};
use std::sync::Mutex;
use crate::types::{Miner, Balance, Rig, Transaction, Alert};

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
    let _password = SecureString::new(request.password.clone());
    
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
    // In a real implementation, this would connect to RPC server
    // TODO: Implement actual RPC client integration
    Ok(RawTransactionResponse {
        success: true,
        txid: Some("mock_txid_1234567890abcdef".to_string()),
        error: None,
    })
}

// Mock data commands for GUI functionality

/// Get miners data for dashboard
#[command]
pub async fn get_miners() -> Result<Vec<Miner>, String> {
        Ok(vec![
        Miner {
            id: 1,
            name: "BitQuan Node #1".to_string(),
            pool: "BitQuan Mainnet".to_string(),
            devices: "1x NVIDIA RTX 4090".to_string(),
            profit: 12.5,
            algo: "RandomX".to_string(),
            speed: "145.2 MH/s".to_string(),
        },
        Miner {
            id: 2,
            name: "BitQuan Node #2".to_string(),
            pool: "BitQuan Testnet".to_string(),
            devices: "1x AMD RX 6800".to_string(),
            profit: 8.3,
            algo: "SHA-256d".to_string(),
            speed: "78.5 MH/s".to_string(),
        },
    ])
}

/// Get balances for dashboard
#[command]
pub async fn get_balances() -> Result<Vec<Balance>, String> {
    Ok(vec![
        Balance {
            pool: "BitQuan Mainnet".to_string(),
            bq: 1250.75,
            btc: 0.0234,
            usd: 1250.75 * 27.50,
        },
        Balance {
            pool: "BitQuan Testnet".to_string(),
            bq: 450.25,
            btc: 0.0089,
            usd: 450.25 * 27.50,
        },
    ])
}

/// Get rigs data for rigs page
#[command]
pub async fn get_rigs() -> Result<Vec<Rig>, String> {
    Ok(vec![
        Rig {
            id: 1,
            name: "BitQuan Node #1".to_string(),
            is_active: true,
            device_type: "NVIDIA RTX 4090".to_string(),
            temp: 72.5,
            power: 350.0,
            hashrate: 145.2,
            hashrate_unit: "MH/s".to_string(),
            algorithm: "RandomX".to_string(),
            miner_version: "v2.1.0".to_string(),
            earnings: 12.5,
        },
        Rig {
            id: 2,
            name: "BitQuan Node #2".to_string(),
            is_active: false,
            device_type: "AMD RX 6800".to_string(),
            temp: 0.0,
            power: 0.0,
            hashrate: 0.0,
            hashrate_unit: "MH/s".to_string(),
            algorithm: "SHA-256d".to_string(),
            miner_version: "v2.0.8".to_string(),
            earnings: 0.0,
        },
        Rig {
            id: 3,
            name: "BitQuan Node #3".to_string(),
            is_active: true,
            device_type: "Intel i7-12700K".to_string(),
            temp: 65.0,
            power: 125.0,
            hashrate: 8.7,
            hashrate_unit: "MH/s".to_string(),
            algorithm: "RandomX".to_string(),
            miner_version: "v2.1.0".to_string(),
            earnings: 3.2,
        },
    ])
}

/// Get transactions for wallet page
#[command]
pub async fn get_transactions() -> Result<Vec<Transaction>, String> {
    Ok(vec![
        Transaction {
            id: "tx_001".to_string(),
            transaction_type: "received".to_string(),
            date: "2024-01-15 14:32:10".to_string(),
            address: "bq1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wl".to_string(),
            amount: 125.50,
        },
        Transaction {
            id: "tx_002".to_string(),
            transaction_type: "sent".to_string(),
            date: "2024-01-14 09:15:22".to_string(),
            address: "bq1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4".to_string(),
            amount: 45.25,
        },
        Transaction {
            id: "tx_003".to_string(),
            transaction_type: "received".to_string(),
            date: "2024-01-13 16:45:33".to_string(),
            address: "bq1qrp33g0q5c5txsp9arysrx4k6zdkfs4nce4xj0gdcccefvpysxf3q".to_string(),
            amount: 89.75,
        },
    ])
}

/// Get alerts for alerts page
#[command]
pub async fn get_alerts() -> Result<Vec<Alert>, String> {
    Ok(vec![
        Alert {
            id: "alert_001".to_string(),
            alert_type: "info".to_string(),
            message: "BitQuan Node #1 has successfully connected to the network".to_string(),
            timestamp: "2024-01-15 14:32:10".to_string(),
        },
        Alert {
            id: "alert_002".to_string(),
            alert_type: "warning".to_string(),
            message: "BitQuan Node #2 is running hot (85°C)".to_string(),
            timestamp: "2024-01-15 13:15:22".to_string(),
        },
        Alert {
            id: "alert_003".to_string(),
            alert_type: "error".to_string(),
            message: "Connection lost to BitQuan Testnet pool".to_string(),
            timestamp: "2024-01-15 12:45:33".to_string(),
        },
        Alert {
            id: "alert_004".to_string(),
            alert_type: "info".to_string(),
            message: "New block mined by your pool! Reward: 12.5 BQ".to_string(),
            timestamp: "2024-01-15 11:20:15".to_string(),
        },
    ])
}