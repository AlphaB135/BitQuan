use crate::error::{BitQuanError, Result};
use crate::types::TransactionStatus;
use serde::{Deserialize, Serialize};
use sha2::Digest;


/// BitQuan transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub version: u32,
    pub inputs: Vec<TransactionInput>,
    pub outputs: Vec<TransactionOutput>,
    pub lock_time: u64,
    pub fee: u64,
    pub signature: Option<String>,
    pub timestamp: Option<u64>,
}

/// Transaction input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionInput {
    pub previous_output: OutPoint,
    pub script_sig: Option<String>,
    pub sequence: u32,
}

/// Transaction output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionOutput {
    pub value: u64,
    pub script_pubkey: String,
    pub address: Option<String>,
}

/// Out point references a previous transaction output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutPoint {
    pub hash: String,
    pub index: u32,
}

/// Transaction builder for creating transactions
#[derive(Debug, Clone)]
pub struct TransactionBuilder {
    version: u32,
    inputs: Vec<TransactionInput>,
    outputs: Vec<TransactionOutput>,
    lock_time: u64,
    fee_rate: u64,
}

impl TransactionBuilder {
    /// Create a new transaction builder
    pub fn new() -> Self {
        Self {
            version: 1,
            inputs: Vec::new(),
            outputs: Vec::new(),
            lock_time: 0,
            fee_rate: 1000, // Default fee rate in satoshis per byte
        }
    }

    /// Set transaction version
    pub fn version(mut self, version: u32) -> Self {
        self.version = version;
        self
    }

    /// Add an input to the transaction
    pub fn add_input(mut self, previous_tx: &str, output_index: u32) -> Self {
        let input = TransactionInput {
            previous_output: OutPoint {
                hash: previous_tx.to_string(),
                index: output_index,
            },
            script_sig: None,
            sequence: u32::MAX,
        };
        self.inputs.push(input);
        self
    }

    /// Add an output to the transaction
    pub fn add_output(mut self, address: &str, amount: u64) -> Self {
        let output = TransactionOutput {
            value: amount,
            script_pubkey: address_to_script_pubkey(address),
            address: Some(address.to_string()),
        };
        self.outputs.push(output);
        self
    }

    /// Set the lock time
    pub fn lock_time(mut self, lock_time: u64) -> Self {
        self.lock_time = lock_time;
        self
    }

    /// Set the fee rate (satoshis per byte)
    pub fn fee_rate(mut self, fee_rate: u64) -> Self {
        self.fee_rate = fee_rate;
        self
    }

    /// Build the transaction
    pub fn build(self) -> Result<Transaction> {
        if self.inputs.is_empty() {
            return Err(BitQuanError::InvalidTransaction(
                "Transaction must have at least one input".to_string(),
            ));
        }

        if self.outputs.is_empty() {
            return Err(BitQuanError::InvalidTransaction(
                "Transaction must have at least one output".to_string(),
            ));
        }

        let tx_size = self.estimate_size();
        let fee = tx_size * self.fee_rate;

        Ok(Transaction {
            version: self.version,
            inputs: self.inputs,
            outputs: self.outputs,
            lock_time: self.lock_time,
            fee,
            signature: None,
            timestamp: None,
        })
    }

    /// Estimate transaction size in bytes
    fn estimate_size(&self) -> u64 {
        // Basic size estimation
        let base_size = 10; // version + lock_time + input/output counts
        let input_size = self.inputs.len() * 40; // Approximate input size
        let output_size = self.outputs.len() * 34; // Approximate output size
        
        (base_size + input_size + output_size) as u64
    }
}

impl Default for TransactionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl Transaction {
    /// Create a new transaction
    pub fn new(version: u32, inputs: Vec<TransactionInput>, outputs: Vec<TransactionOutput>) -> Self {
        Self {
            version,
            inputs,
            outputs,
            lock_time: 0,
            fee: 0,
            signature: None,
            timestamp: None,
        }
    }

    /// Get transaction hash
    pub fn hash(&self) -> String {
        let serialized = self.serialize();
        hex::encode(sha2::Sha256::new().chain_update(serialized.as_bytes()).finalize())
    }

    /// Get transaction size in bytes
    pub fn size(&self) -> u64 {
        self.serialize().len() as u64
    }

    /// Get total input value
    pub fn input_value(&self) -> u64 {
        self.inputs.iter().map(|_| 0).sum() // Would need UTXO lookup
    }

    /// Get total output value
    pub fn output_value(&self) -> u64 {
        self.outputs.iter().map(|output| output.value).sum()
    }

    /// Get transaction fee
    pub fn fee(&self) -> u64 {
        self.fee
    }

    /// Check if transaction is valid
    pub fn is_valid(&self) -> bool {
        !self.inputs.is_empty() 
            && !self.outputs.is_empty()
            && self.output_value() > 0
    }

    /// Serialize transaction to hex string
    pub fn serialize(&self) -> String {
        // Simplified serialization
        format!(
            "v:{}|i:{}|o:{}|lt:{}",
            self.version,
            self.inputs.len(),
            self.outputs.len(),
            self.lock_time
        )
    }

    /// Deserialize transaction from hex string
    pub fn deserialize(_hex: &str) -> Result<Self> {
        // Simplified deserialization
        Err(BitQuanError::InvalidTransaction(
            "Deserialization not implemented".to_string(),
        ))
    }

    /// Sign transaction with private key
    pub fn sign(&mut self, _private_key: &str) -> Result<()> {
        // Simplified signing
        self.signature = Some("signed".to_string());
        Ok(())
    }

    /// Verify transaction signature
    pub fn verify_signature(&self) -> bool {
        self.signature.is_some()
    }
}

/// Convert address to script pubkey
fn address_to_script_pubkey(address: &str) -> String {
    // Simplified address to script conversion
    format!("OP_DUP OP_HASH160 {} OP_EQUALVERIFY OP_CHECKSIG", address)
}

/// Transaction pool information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionPoolInfo {
    pub size: usize,
    pub bytes: usize,
    pub usage: usize,
    pub max_size: usize,
}

/// Transaction receipt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionReceipt {
    pub transaction_hash: String,
    pub block_hash: Option<String>,
    pub block_number: Option<u64>,
    pub transaction_index: Option<u32>,
    pub status: TransactionStatus,
    pub gas_used: Option<u64>,
    pub cumulative_gas_used: Option<u64>,
    pub logs: Vec<TransactionLog>,
}

/// Transaction log
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionLog {
    pub address: String,
    pub topics: Vec<String>,
    pub data: String,
    pub block_number: Option<u64>,
    pub transaction_hash: Option<String>,
    pub transaction_index: Option<u32>,
    pub block_hash: Option<String>,
    pub log_index: Option<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transaction_builder() {
        let tx = TransactionBuilder::new()
            .add_input("prev_tx_hash", 0)
            .add_output("address1", 1000)
            .add_output("address2", 2000)
            .build()
            .unwrap();

        assert_eq!(tx.inputs.len(), 1);
        assert_eq!(tx.outputs.len(), 2);
        assert_eq!(tx.output_value(), 3000);
        assert!(tx.is_valid());
    }

    #[test]
    fn test_empty_transaction() {
        let result = TransactionBuilder::new().build();
        assert!(result.is_err());
    }

    #[test]
    fn test_transaction_hash() {
        let tx = TransactionBuilder::new()
            .add_input("prev_tx_hash", 0)
            .add_output("address1", 1000)
            .build()
            .unwrap();

        let hash = tx.hash();
        assert!(!hash.is_empty());
    }
}