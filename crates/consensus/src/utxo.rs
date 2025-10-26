//! UTXO (Unspent Transaction Output) set management and validation.
//!
//! This module provides the core UTXO database that tracks all unspent outputs
//! in the blockchain, enabling double-spend detection and transaction validation.

use std::collections::HashMap;
use bitquan_types::{Transaction, TxOut};
use thiserror::Error;
use serde::{Deserialize, Serialize};

/// Errors that can occur during UTXO operations.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum UtxoError {
    /// Attempted to spend an output that doesn't exist.
    #[error("output not found: txid={}, vout={}", hex::encode(.0), .1)]
    OutputNotFound([u8; 32], u32),
    
    /// Attempted to spend an output that was already spent.
    #[error("output already spent: txid={}, vout={}", hex::encode(.0), .1)]
    DoubleSpend([u8; 32], u32),
    
    /// Transaction creates outputs with total value greater than inputs.
    #[error("outputs exceed inputs: inputs={0}, outputs={1}")]
    OutputsExceedInputs(u64, u64),
    
    /// Coinbase transaction is not the first transaction in block.
    #[error("coinbase must be first transaction")]
    CoinbaseNotFirst,
    
    /// Non-coinbase transaction has coinbase-style inputs.
    #[error("non-coinbase transaction has null inputs")]
    InvalidCoinbase,
    
    /// Output value overflow.
    #[error("output value overflow")]
    Overflow,
}

/// Unique identifier for a transaction output (outpoint).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OutPoint {
    /// Transaction ID (hash).
    pub txid: [u8; 32],
    /// Output index within the transaction.
    pub vout: u32,
}

impl OutPoint {
    /// Creates a new outpoint.
    pub fn new(txid: [u8; 32], vout: u32) -> Self {
        Self { txid, vout }
    }
    
    /// Creates a coinbase outpoint (null hash, max vout).
    pub fn coinbase() -> Self {
        Self {
            txid: [0u8; 32],
            vout: u32::MAX,
        }
    }
    
    /// Checks if this is a coinbase outpoint.
    pub fn is_coinbase(&self) -> bool {
        self.txid == [0u8; 32] && self.vout == u32::MAX
    }
}

/// A UTXO entry in the database.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UtxoEntry {
    /// The outpoint this entry represents.
    pub outpoint: OutPoint,
    /// The transaction output data.
    pub output: TxOut,
    /// Block height where this output was created.
    pub height: u64,
    /// Whether this is a coinbase output (special maturity rules).
    pub is_coinbase: bool,
}

impl UtxoEntry {
    /// Creates a new UTXO entry.
    pub fn new(outpoint: OutPoint, output: TxOut, height: u64, is_coinbase: bool) -> Self {
        Self {
            outpoint,
            output,
            height,
            is_coinbase,
        }
    }
    
    /// Checks if this UTXO is mature (spendable).
    /// Coinbase outputs require 100 confirmations.
    pub fn is_mature(&self, current_height: u64) -> bool {
        if !self.is_coinbase {
            return true;
        }
        // Coinbase maturity: 100 blocks
        current_height >= self.height.saturating_add(100)
    }
}

/// UTXO set database tracking all unspent outputs.
pub struct UtxoSet {
    /// Map of outpoint -> UTXO entry.
    utxos: HashMap<OutPoint, UtxoEntry>,
    /// Total number of UTXOs.
    count: usize,
    /// Total value of all UTXOs (in base units).
    total_value: u64,
}

impl UtxoSet {
    /// Creates a new empty UTXO set.
    pub fn new() -> Self {
        Self {
            utxos: HashMap::new(),
            count: 0,
            total_value: 0,
        }
    }
    
    /// Returns the number of UTXOs in the set.
    pub fn len(&self) -> usize {
        self.count
    }
    
    /// Checks if the UTXO set is empty.
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
    
    /// Returns the total value of all UTXOs.
    pub fn total_value(&self) -> u64 {
        self.total_value
    }
    
    /// Adds a UTXO to the set.
    pub fn add_utxo(&mut self, entry: UtxoEntry) {
        let value = entry.output.value;
        self.utxos.insert(entry.outpoint, entry);
        self.count += 1;
        self.total_value = self.total_value.saturating_add(value);
    }
    
    /// Removes a UTXO from the set (when spent).
    pub fn remove_utxo(&mut self, outpoint: &OutPoint) -> Option<UtxoEntry> {
        if let Some(entry) = self.utxos.remove(outpoint) {
            self.count -= 1;
            self.total_value = self.total_value.saturating_sub(entry.output.value);
            Some(entry)
        } else {
            None
        }
    }
    
    /// Gets a UTXO entry by outpoint.
    pub fn get_utxo(&self, outpoint: &OutPoint) -> Option<&UtxoEntry> {
        self.utxos.get(outpoint)
    }
    
    /// Checks if an outpoint exists in the UTXO set.
    pub fn contains(&self, outpoint: &OutPoint) -> bool {
        self.utxos.contains_key(outpoint)
    }
    
    /// Validates and applies a transaction to the UTXO set.
    ///
    /// Returns (inputs_value, outputs_value, fee)
    pub fn apply_transaction(
        &mut self,
        tx: &Transaction,
        height: u64,
        is_coinbase: bool,
    ) -> Result<(u64, u64, u64), UtxoError> {
        // Handle coinbase separately
        if is_coinbase {
            return self.apply_coinbase(tx, height);
        }
        
        // Check for coinbase-style inputs in non-coinbase tx
        for input in &tx.inputs {
            let outpoint = OutPoint::new(input.prev_txid, input.prev_vout);
            if outpoint.is_coinbase() {
                return Err(UtxoError::InvalidCoinbase);
            }
        }
        
        // Collect and validate inputs
        let mut inputs_value = 0u64;
        let mut spent_outpoints = Vec::new();
        
        for input in &tx.inputs {
            let outpoint = OutPoint::new(input.prev_txid, input.prev_vout);
            
            // Check if UTXO exists
            let utxo = self.get_utxo(&outpoint)
                .ok_or(UtxoError::OutputNotFound(input.prev_txid, input.prev_vout))?;
            
            // Check maturity (coinbase outputs)
            if !utxo.is_mature(height) {
                return Err(UtxoError::OutputNotFound(input.prev_txid, input.prev_vout));
            }
            
            inputs_value = inputs_value.checked_add(utxo.output.value)
                .ok_or(UtxoError::Overflow)?;
            
            spent_outpoints.push(outpoint);
        }
        
        // Calculate outputs value
        let mut outputs_value = 0u64;
        for output in &tx.outputs {
            outputs_value = outputs_value.checked_add(output.value)
                .ok_or(UtxoError::Overflow)?;
        }
        
        // Check outputs don't exceed inputs
        if outputs_value > inputs_value {
            return Err(UtxoError::OutputsExceedInputs(inputs_value, outputs_value));
        }
        
        // Calculate fee
        let fee = inputs_value - outputs_value;
        
        // Remove spent UTXOs
        for outpoint in spent_outpoints {
            self.remove_utxo(&outpoint);
        }
        
        // Add new UTXOs
        let txid = tx.txid();
        for (vout, output) in tx.outputs.iter().enumerate() {
            let outpoint = OutPoint::new(txid, vout as u32);
            let entry = UtxoEntry::new(outpoint, output.clone(), height, false);
            self.add_utxo(entry);
        }
        
        Ok((inputs_value, outputs_value, fee))
    }
    
    /// Applies a coinbase transaction.
    fn apply_coinbase(
        &mut self,
        tx: &Transaction,
        height: u64,
    ) -> Result<(u64, u64, u64), UtxoError> {
        // Coinbase has no inputs (or one null input)
        let inputs_value = 0u64;
        
        // Calculate outputs
        let mut outputs_value = 0u64;
        for output in &tx.outputs {
            outputs_value = outputs_value.checked_add(output.value)
                .ok_or(UtxoError::Overflow)?;
        }
        
        // Add coinbase outputs to UTXO set
        let txid = tx.txid();
        for (vout, output) in tx.outputs.iter().enumerate() {
            let outpoint = OutPoint::new(txid, vout as u32);
            let entry = UtxoEntry::new(outpoint, output.clone(), height, true);
            self.add_utxo(entry);
        }
        
        // Coinbase has no fee (subsidy is validated separately)
        Ok((inputs_value, outputs_value, 0))
    }
    
    /// Validates a transaction without applying it (dry run).
    pub fn validate_transaction(
        &self,
        tx: &Transaction,
        height: u64,
        is_coinbase: bool,
    ) -> Result<(u64, u64, u64), UtxoError> {
        if is_coinbase {
            // Coinbase validation
            let mut outputs_value = 0u64;
            for output in &tx.outputs {
                outputs_value = outputs_value.checked_add(output.value)
                    .ok_or(UtxoError::Overflow)?;
            }
            return Ok((0, outputs_value, 0));
        }
        
        // Regular transaction validation
        let mut inputs_value = 0u64;
        
        for input in &tx.inputs {
            let outpoint = OutPoint::new(input.prev_txid, input.prev_vout);
            
            if outpoint.is_coinbase() {
                return Err(UtxoError::InvalidCoinbase);
            }
            
            let utxo = self.get_utxo(&outpoint)
                .ok_or(UtxoError::OutputNotFound(input.prev_txid, input.prev_vout))?;
            
            if !utxo.is_mature(height) {
                return Err(UtxoError::OutputNotFound(input.prev_txid, input.prev_vout));
            }
            
            inputs_value = inputs_value.checked_add(utxo.output.value)
                .ok_or(UtxoError::Overflow)?;
        }
        
        let mut outputs_value = 0u64;
        for output in &tx.outputs {
            outputs_value = outputs_value.checked_add(output.value)
                .ok_or(UtxoError::Overflow)?;
        }
        
        if outputs_value > inputs_value {
            return Err(UtxoError::OutputsExceedInputs(inputs_value, outputs_value));
        }
        
        let fee = inputs_value - outputs_value;
        Ok((inputs_value, outputs_value, fee))
    }
}

impl Default for UtxoSet {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitquan_types::{SigAlgorithm, TxIn};

    fn create_test_tx(
        inputs: Vec<([u8; 32], u32)>,
        outputs: Vec<u64>,
    ) -> Transaction {
        Transaction {
            version: 1,
            lock_time: 0,
            inputs: inputs
                .into_iter()
                .map(|(txid, vout)| TxIn {
                    prev_txid: txid,
                    prev_vout: vout,
                    sequence: 0xffffffff,
                    script_sig: vec![],
                })
                .collect(),
            outputs: outputs
                .into_iter()
                .map(|value| TxOut {
                    value,
                    script_pubkey: vec![0x51],
                })
                .collect(),
            sig_algo: SigAlgorithm::Dilithium3,
            witnesses: vec![],
        }
    }

    #[test]
    fn utxo_set_basic_operations() {
        let mut utxo_set = UtxoSet::new();
        assert_eq!(utxo_set.len(), 0);
        assert!(utxo_set.is_empty());

        let outpoint = OutPoint::new([1u8; 32], 0);
        let output = TxOut {
            value: 1000,
            script_pubkey: vec![0x51],
        };
        let entry = UtxoEntry::new(outpoint, output, 100, false);

        utxo_set.add_utxo(entry);
        assert_eq!(utxo_set.len(), 1);
        assert_eq!(utxo_set.total_value(), 1000);
        assert!(utxo_set.contains(&outpoint));
    }

    #[test]
    fn detect_double_spend() {
        let mut utxo_set = UtxoSet::new();

        // Create initial UTXO
        let txid1 = [1u8; 32];
        let outpoint = OutPoint::new(txid1, 0);
        let output = TxOut {
            value: 1000,
            script_pubkey: vec![0x51],
        };
        utxo_set.add_utxo(UtxoEntry::new(outpoint, output, 100, false));

        // First spend - should succeed
        let tx1 = create_test_tx(vec![(txid1, 0)], vec![900]);
        assert!(utxo_set.apply_transaction(&tx1, 101, false).is_ok());

        // Second spend of same output - should fail
        let tx2 = create_test_tx(vec![(txid1, 0)], vec![800]);
        assert!(matches!(
            utxo_set.apply_transaction(&tx2, 102, false),
            Err(UtxoError::OutputNotFound(_, _))
        ));
    }

    #[test]
    fn reject_outputs_exceeding_inputs() {
        let mut utxo_set = UtxoSet::new();

        let txid = [1u8; 32];
        let outpoint = OutPoint::new(txid, 0);
        let output = TxOut {
            value: 1000,
            script_pubkey: vec![0x51],
        };
        utxo_set.add_utxo(UtxoEntry::new(outpoint, output, 100, false));

        // Try to spend more than available
        let tx = create_test_tx(vec![(txid, 0)], vec![1500]);
        assert!(matches!(
            utxo_set.apply_transaction(&tx, 101, false),
            Err(UtxoError::OutputsExceedInputs(1000, 1500))
        ));
    }

    #[test]
    fn coinbase_maturity() {
        let mut utxo_set = UtxoSet::new();

        // Create coinbase UTXO at height 100
        let txid = [1u8; 32];
        let outpoint = OutPoint::new(txid, 0);
        let output = TxOut {
            value: 5000,
            script_pubkey: vec![0x51],
        };
        utxo_set.add_utxo(UtxoEntry::new(outpoint, output, 100, true));

        // Try to spend before maturity (100 blocks)
        let tx = create_test_tx(vec![(txid, 0)], vec![4900]);
        assert!(matches!(
            utxo_set.apply_transaction(&tx, 150, false),
            Err(UtxoError::OutputNotFound(_, _))
        ));

        // Spend after maturity
        assert!(utxo_set.apply_transaction(&tx, 200, false).is_ok());
    }

    #[test]
    fn calculate_fee_correctly() {
        let mut utxo_set = UtxoSet::new();

        let txid = [1u8; 32];
        let outpoint = OutPoint::new(txid, 0);
        let output = TxOut {
            value: 1000,
            script_pubkey: vec![0x51],
        };
        utxo_set.add_utxo(UtxoEntry::new(outpoint, output, 100, false));

        let tx = create_test_tx(vec![(txid, 0)], vec![900]);
        let result = utxo_set.apply_transaction(&tx, 101, false).unwrap();

        assert_eq!(result.0, 1000); // inputs
        assert_eq!(result.1, 900);  // outputs
        assert_eq!(result.2, 100);  // fee
    }
}
