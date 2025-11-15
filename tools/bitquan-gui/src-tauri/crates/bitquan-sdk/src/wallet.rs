use crate::error::{BitQuanError, Result};
use crate::types::{Account, Network};
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};
use sha2::Digest;
use std::collections::HashMap;

/// Wallet manager for handling multiple accounts
#[derive(Debug)]
pub struct WalletManager {
    accounts: HashMap<String, Wallet>,
    default_account: Option<String>,
    network: Network,
}

impl WalletManager {
    /// Create a new wallet manager
    pub fn new(network: Network) -> Self {
        Self {
            accounts: HashMap::new(),
            default_account: None,
            network,
        }
    }

    /// Create a new wallet
    pub fn create_wallet(&mut self, name: &str) -> Result<String> {
        if self.accounts.contains_key(name) {
            return Err(BitQuanError::Wallet(format!(
                "Wallet '{}' already exists",
                name
            )));
        }

        let wallet = Wallet::new(self.network)?;
        let address = wallet.address().to_string();
        self.accounts.insert(name.to_string(), wallet);

        if self.default_account.is_none() {
            self.default_account = Some(name.to_string());
        }

        Ok(address)
    }

    /// Import wallet from private key
    pub fn import_wallet(&mut self, name: &str, private_key: &str) -> Result<String> {
        if self.accounts.contains_key(name) {
            return Err(BitQuanError::Wallet(format!(
                "Wallet '{}' already exists",
                name
            )));
        }

        let wallet = Wallet::from_private_key(private_key, self.network)?;
        let address = wallet.address().to_string();
        self.accounts.insert(name.to_string(), wallet);

        if self.default_account.is_none() {
            self.default_account = Some(name.to_string());
        }

        Ok(address)
    }

    /// Get wallet by name
    pub fn get_wallet(&self, name: &str) -> Option<&Wallet> {
        self.accounts.get(name)
    }

    /// Get mutable wallet by name
    pub fn get_wallet_mut(&mut self, name: &str) -> Option<&mut Wallet> {
        self.accounts.get_mut(name)
    }

    /// Get default wallet
    pub fn get_default_wallet(&self) -> Option<&Wallet> {
        self.default_account.as_ref().and_then(|name| self.accounts.get(name))
    }

    /// Get mutable default wallet
    pub fn get_default_wallet_mut(&mut self) -> Option<&mut Wallet> {
        self.default_account.as_ref().and_then(|name| self.accounts.get_mut(name))
    }

    /// Set default wallet
    pub fn set_default_wallet(&mut self, name: &str) -> Result<()> {
        if !self.accounts.contains_key(name) {
            return Err(BitQuanError::Wallet(format!("Wallet '{}' not found", name)));
        }
        self.default_account = Some(name.to_string());
        Ok(())
    }

    /// List all wallet names
    pub fn list_wallets(&self) -> Vec<String> {
        self.accounts.keys().cloned().collect()
    }

    /// Remove wallet
    pub fn remove_wallet(&mut self, name: &str) -> Result<()> {
        if !self.accounts.contains_key(name) {
            return Err(BitQuanError::Wallet(format!("Wallet '{}' not found", name)));
        }

        self.accounts.remove(name);
        
        if self.default_account.as_ref().map_or(false, |default| default == name) {
            self.default_account = self.accounts.keys().next().cloned();
        }

        Ok(())
    }

    /// Get wallet count
    pub fn wallet_count(&self) -> usize {
        self.accounts.len()
    }

    /// Update account information for all wallets
    pub async fn update_accounts(&mut self, client: &crate::client::BitQuanClient) -> Result<()> {
        for wallet in self.accounts.values_mut() {
            wallet.update_account(client).await?;
        }
        Ok(())
    }
}

/// Individual wallet
#[derive(Debug, Clone)]
pub struct Wallet {
    private_key: String,
    public_key: String,
    address: String,
    network: Network,
    account: Option<Account>,
}

impl Wallet {
    /// Create a new wallet with generated key pair
    pub fn new(network: Network) -> Result<Self> {
        let private_key = generate_private_key();
        let public_key = derive_public_key(&private_key)?;
        let address = derive_address(&public_key, network)?;

        Ok(Self {
            private_key,
            public_key,
            address,
            network,
            account: None,
        })
    }

    /// Create wallet from private key
    pub fn from_private_key(private_key: &str, network: Network) -> Result<Self> {
        let public_key = derive_public_key(private_key)?;
        let address = derive_address(&public_key, network)?;

        Ok(Self {
            private_key: private_key.to_string(),
            public_key,
            address,
            network,
            account: None,
        })
    }

    /// Get wallet address
    pub fn address(&self) -> &str {
        &self.address
    }

    /// Get private key
    pub fn private_key(&self) -> &str {
        &self.private_key
    }

    /// Get public key
    pub fn public_key(&self) -> &str {
        &self.public_key
    }

    /// Get network
    pub fn network(&self) -> Network {
        self.network
    }

    /// Get account information
    pub fn account(&self) -> Option<&Account> {
        self.account.as_ref()
    }

    /// Update account information from blockchain
    pub async fn update_account(&mut self, client: &crate::client::BitQuanClient) -> Result<()> {
        let account = client.get_account(&self.address).await?;
        self.account = Some(account);
        Ok(())
    }

    /// Get balance
    pub fn balance(&self) -> u64 {
        self.account.as_ref().map_or(0, |acc| acc.balance)
    }

    /// Get nonce
    pub fn nonce(&self) -> u64 {
        self.account.as_ref().map_or(0, |acc| acc.nonce)
    }

    /// Sign message
    pub fn sign_message(&self, message: &str) -> Result<String> {
        // Simplified signing
        let signature = format!("signed_{}_{}", message, &self.private_key[..8]);
        Ok(signature)
    }

    /// Verify message signature
    pub fn verify_message(&self, message: &str, signature: &str) -> bool {
        signature.contains(message) && signature.contains(&self.private_key[..8])
    }

    /// Export wallet to JSON
    pub fn export(&self) -> Result<WalletExport> {
        Ok(WalletExport {
            address: self.address.clone(),
            private_key: self.private_key.clone(),
            public_key: self.public_key.clone(),
            network: self.network,
        })
    }

    /// Import wallet from JSON
    pub fn import(export: WalletExport) -> Result<Self> {
        Ok(Self {
            private_key: export.private_key,
            public_key: export.public_key,
            address: export.address,
            network: export.network,
            account: None,
        })
    }
}

/// Wallet export format
#[derive(Debug, Serialize, Deserialize)]
pub struct WalletExport {
    pub address: String,
    pub private_key: String,
    pub public_key: String,
    pub network: Network,
}

/// Generate a new private key
fn generate_private_key() -> String {
    let mut rng = thread_rng();
    let mut bytes = [0u8; 32];
    rng.fill(&mut bytes);
    hex::encode(bytes)
}

/// Derive public key from private key
fn derive_public_key(private_key: &str) -> Result<String> {
    // Simplified public key derivation
    let public_key = format!("pub_{}", &private_key[..16]);
    Ok(public_key)
}

/// Derive address from public key
fn derive_address(public_key: &str, network: Network) -> Result<String> {
    // Simplified address derivation
    let prefix = match network {
        Network::Mainnet => "bq",
        Network::Testnet => "tbq",
        Network::Devnet => "dbq",
    };
    
    let hash = hex::encode(sha2::Sha256::new().chain_update(public_key.as_bytes()).finalize());
    let address = format!("{}1{}", prefix, &hash[..32]);
    Ok(address)
}

/// Address validation
pub fn validate_address(address: &str) -> bool {
    if address.len() != 35 {
        return false;
    }

    let prefix = &address[..3];
    matches!(prefix, "bq1" | "tbq1" | "dbq1")
}

/// Address utilities
pub mod address {
    use super::*;

    /// Extract network from address
    pub fn extract_network(address: &str) -> Option<Network> {
        if address.starts_with("bq1") {
            Some(Network::Mainnet)
        } else if address.starts_with("tbq1") {
            Some(Network::Testnet)
        } else if address.starts_with("dbq1") {
            Some(Network::Devnet)
        } else {
            None
        }
    }

    /// Convert address to different network
    pub fn convert_network(address: &str, target_network: Network) -> Result<String> {
        if !validate_address(address) {
            return Err(BitQuanError::InvalidAddress(address.to_string()));
        }

        let current_network = extract_network(address).unwrap();
        if current_network == target_network {
            return Ok(address.to_string());
        }

        let suffix = &address[3..];
        let prefix = match target_network {
            Network::Mainnet => "bq1",
            Network::Testnet => "tbq1",
            Network::Devnet => "dbq1",
        };

        Ok(format!("{}{}", prefix, suffix))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wallet_creation() {
        let wallet = Wallet::new(Network::Testnet).unwrap();
        assert!(validate_address(wallet.address()));
        assert_eq!(wallet.network(), Network::Testnet);
        assert!(!wallet.private_key().is_empty());
        assert!(!wallet.public_key().is_empty());
    }

    #[test]
    fn test_wallet_manager() {
        let mut manager = WalletManager::new(Network::Testnet);
        
        let address1 = manager.create_wallet("wallet1").unwrap();
        let address2 = manager.create_wallet("wallet2").unwrap();
        
        assert_ne!(address1, address2);
        assert_eq!(manager.wallet_count(), 2);
        assert!(manager.get_wallet("wallet1").is_some());
        assert!(manager.get_wallet("nonexistent").is_none());
    }

    #[test]
    fn test_address_validation() {
        assert!(validate_address("bq1abcdef1234567890abcdef1234567890ab"));
        assert!(validate_address("tbq1abcdef1234567890abcdef1234567890ab"));
        assert!(validate_address("dbq1abcdef1234567890abcdef1234567890ab"));
        assert!(!validate_address("invalid"));
        assert!(!validate_address("bq1short"));
    }

    #[test]
    fn test_wallet_export_import() {
        let wallet = Wallet::new(Network::Mainnet).unwrap();
        let export = wallet.export().unwrap();
        let imported_wallet = Wallet::import(export).unwrap();
        
        assert_eq!(wallet.address(), imported_wallet.address());
        assert_eq!(wallet.private_key(), imported_wallet.private_key());
    }

    #[test]
    fn test_message_signing() {
        let wallet = Wallet::new(Network::Mainnet).unwrap();
        let message = "Hello, BitQuan!";
        let signature = wallet.sign_message(message).unwrap();
        assert!(wallet.verify_message(message, &signature));
        assert!(!wallet.verify_message("Different message", &signature));
    }
}