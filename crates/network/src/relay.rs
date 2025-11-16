//! Transaction and block relay logic for P2P network.

use crate::{
    protocol::{InvType, InvVector, Message},
    NetworkError, Result,
};
use bitquan_types::Transaction;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Relay manager for tracking announced and requested items
pub struct RelayManager {
    /// Recently announced inventory (txid/block_hash -> timestamp)
    announced: Arc<Mutex<HashMap<[u8; 32], Instant>>>,
    /// Pending requests (hash -> requesting peer IDs)
    pending_requests: Arc<Mutex<HashMap<[u8; 32], HashSet<String>>>>,
    /// Recently relayed items (to prevent loops)
    relayed: Arc<Mutex<HashSet<[u8; 32]>>>,
    /// Maximum items to track
    max_items: usize,
}

impl RelayManager {
    /// Creates a new relay manager
    pub fn new(max_items: usize) -> Self {
        Self {
            announced: Arc::new(Mutex::new(HashMap::new())),
            pending_requests: Arc::new(Mutex::new(HashMap::new())),
            relayed: Arc::new(Mutex::new(HashSet::new())),
            max_items,
        }
    }

    /// Records an inventory announcement
    pub fn announce(&self, inv: &InvVector) -> Result<()> {
        let mut announced = self
            .announced
            .lock()
            .map_err(|e| NetworkError::LockPoisoned(format!("announced: {}", e)))?;

        // Cleanup old entries if needed
        if announced.len() >= self.max_items {
            let cutoff = Instant::now() - Duration::from_secs(600); // 10 minutes
            announced.retain(|_, time| *time > cutoff);
        }

        announced.insert(inv.hash, Instant::now());
        Ok(())
    }

    /// Checks if we've recently announced this item
    pub fn has_announced(&self, hash: &[u8; 32]) -> Result<bool> {
        let announced = self
            .announced
            .lock()
            .map_err(|e| NetworkError::LockPoisoned(format!("announced: {}", e)))?;
        Ok(announced.contains_key(hash))
    }

    /// Adds a pending request
    pub fn add_request(&self, hash: [u8; 32], peer_id: String) -> Result<()> {
        let mut requests = self
            .pending_requests
            .lock()
            .map_err(|e| NetworkError::LockPoisoned(format!("pending_requests: {}", e)))?;
        requests.entry(hash).or_default().insert(peer_id);
        Ok(())
    }

    /// Removes a pending request
    pub fn remove_request(&self, hash: &[u8; 32]) -> Result<()> {
        let mut requests = self
            .pending_requests
            .lock()
            .map_err(|e| NetworkError::LockPoisoned(format!("pending_requests: {}", e)))?;
        requests.remove(hash);
        Ok(())
    }

    /// Gets peers waiting for this item
    pub fn get_requesters(&self, hash: &[u8; 32]) -> Result<Vec<String>> {
        let requests = self
            .pending_requests
            .lock()
            .map_err(|e| NetworkError::LockPoisoned(format!("pending_requests: {}", e)))?;
        Ok(requests
            .get(hash)
            .map(|s| s.iter().cloned().collect())
            .unwrap_or_default())
    }

    /// Marks an item as relayed
    pub fn mark_relayed(&self, hash: [u8; 32]) -> Result<()> {
        let mut relayed = self
            .relayed
            .lock()
            .map_err(|e| NetworkError::LockPoisoned(format!("relayed: {}", e)))?;

        // Limit size
        if relayed.len() >= self.max_items {
            relayed.clear(); // Simple cleanup
        }

        relayed.insert(hash);
        Ok(())
    }

    /// Checks if we've already relayed this
    pub fn was_relayed(&self, hash: &[u8; 32]) -> Result<bool> {
        let relayed = self
            .relayed
            .lock()
            .map_err(|e| NetworkError::LockPoisoned(format!("relayed: {}", e)))?;
        Ok(relayed.contains(hash))
    }

    /// Cleans up old data
    pub fn cleanup(&self) -> Result<()> {
        let cutoff = Instant::now() - Duration::from_secs(600);

        let mut announced = self
            .announced
            .lock()
            .map_err(|e| NetworkError::LockPoisoned(format!("announced: {}", e)))?;
        announced.retain(|_, time| *time > cutoff);
        Ok(())
    }
}

/// Transaction relay policy
pub struct RelayPolicy {
    /// Minimum fee per weight unit (in qbits)
    pub min_fee_rate: u64,
    /// Maximum transaction size (bytes)
    pub max_tx_size: usize,
    /// Maximum signature count per transaction
    pub max_signatures: usize,
}

impl Default for RelayPolicy {
    fn default() -> Self {
        Self {
            min_fee_rate: 1,      // 1 qbit per WU
            max_tx_size: 400_000, // 400 KB
            max_signatures: 100,  // Reasonable limit
        }
    }
}

impl RelayPolicy {
    /// Checks if a transaction should be relayed
    pub fn should_relay(&self, tx: &Transaction) -> bool {
        // Check size
        let tx_bytes = self.estimate_tx_size(tx);
        if tx_bytes > self.max_tx_size {
            return false;
        }

        // Check signature count
        let sig_count = match tx.signature_count() {
            Ok(count) => count,
            Err(_) => return false, // Overflow in signature count
        };
        if sig_count > self.max_signatures {
            return false;
        }

        // Check fee rate (simplified - would need UTXO lookup in reality)
        // For now, just accept all transactions
        true
    }

    /// Estimates transaction size in bytes
    fn estimate_tx_size(&self, tx: &Transaction) -> usize {
        // Rough estimate: base + inputs + outputs + witnesses
        let base = 10; // version, locktime
        let inputs = tx.inputs.len() * 100; // ~100 bytes per input
        let outputs = tx.outputs.len() * 50; // ~50 bytes per output
        let witnesses = tx.witnesses.len() * 3000; // ~3KB per Dilithium sig

        base + inputs + outputs + witnesses
    }
}

/// Creates an inventory message for a transaction
pub fn create_tx_inv(txid: [u8; 32]) -> Message {
    Message::Inv {
        inventory: vec![InvVector {
            inv_type: InvType::Tx,
            hash: txid,
        }],
    }
}

/// Creates an inventory message for a block
pub fn create_block_inv(block_hash: [u8; 32]) -> Message {
    Message::Inv {
        inventory: vec![InvVector {
            inv_type: InvType::Block,
            hash: block_hash,
        }],
    }
}

/// Creates a getdata request for transactions
pub fn create_tx_getdata(txids: Vec<[u8; 32]>) -> Message {
    Message::GetData {
        inventory: txids
            .into_iter()
            .map(|hash| InvVector {
                inv_type: InvType::Tx,
                hash,
            })
            .collect(),
    }
}

/// Creates a getdata request for blocks
pub fn create_block_getdata(block_hashes: Vec<[u8; 32]>) -> Message {
    Message::GetData {
        inventory: block_hashes
            .into_iter()
            .map(|hash| InvVector {
                inv_type: InvType::Block,
                hash,
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitquan_types::{genesis::GENESIS_HASH_BYTES, NetworkId};

    #[test]
    fn test_relay_manager() {
        let manager = RelayManager::new(100);
        let hash = [0x42u8; 32];

        let inv = InvVector {
            inv_type: InvType::Tx,
            hash,
        };

        manager
            .announce(&inv)
            .expect("Failed to announce inventory");
        assert!(manager
            .has_announced(&hash)
            .expect("Failed to check if announced"));

        manager
            .mark_relayed(hash)
            .expect("Failed to mark as relayed");
        assert!(manager
            .was_relayed(&hash)
            .expect("Failed to check if relayed"));
    }

    #[test]
    fn test_relay_policy() {
        let policy = RelayPolicy::default();

        // Create test transaction
        let tx = Transaction {
            version: 2,
            network: NetworkId::Devnet,
            genesis_hash: GENESIS_HASH_BYTES,
            lock_time: 0,
            inputs: vec![],
            outputs: vec![],
            sig_algo: bitquan_types::SigAlgorithm::Dilithium3,
            witnesses: vec![],
        };

        assert!(policy.should_relay(&tx));
    }

    #[test]
    fn test_create_inv_messages() {
        let hash = [0x42u8; 32];

        let tx_inv = create_tx_inv(hash);
        match tx_inv {
            Message::Inv { inventory } => {
                assert_eq!(inventory.len(), 1);
                assert_eq!(inventory[0].inv_type, InvType::Tx);
            }
            _ => {
                // Test code: unreachable is OK
                unreachable!("Expected Inv message for TX");
            }
        }

        let block_inv = create_block_inv(hash);
        match block_inv {
            Message::Inv { inventory } => {
                assert_eq!(inventory.len(), 1);
                assert_eq!(inventory[0].inv_type, InvType::Block);
            }
            _ => {
                // Test code: unreachable is OK
                unreachable!("Expected Inv message for Block");
            }
        }
    }
}
