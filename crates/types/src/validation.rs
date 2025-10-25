//! Transaction and block validation utilities.

use crate::{Block, Transaction};
use std::collections::HashSet;
use thiserror::Error;

/// Maximum transaction size in bytes (1 MB)
const MAX_TX_SIZE: usize = 1_000_000;

/// Maximum block size in bytes (4 MB)
const MAX_BLOCK_SIZE: usize = 4_000_000;

/// Maximum script size in bytes
const MAX_SCRIPT_SIZE: usize = 10_000;

/// Maximum number of signature operations per transaction
const MAX_SIG_OPS_PER_TX: usize = 20;

/// Maximum number of inputs per transaction
const MAX_TX_INPUTS: usize = 10_000;

/// Maximum number of outputs per transaction
const MAX_TX_OUTPUTS: usize = 10_000;

/// Validation errors
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ValidationError {
    /// Transaction exceeds maximum size
    #[error("transaction size {0} exceeds maximum {1}")]
    TransactionTooLarge(usize, usize),
    
    /// Block exceeds maximum size
    #[error("block size {0} exceeds maximum {1}")]
    BlockTooLarge(usize, usize),
    
    /// Transaction has no inputs
    #[error("transaction has no inputs")]
    NoInputs,
    
    /// Transaction has no outputs
    #[error("transaction has no outputs")]
    NoOutputs,
    
    /// Transaction has too many inputs
    #[error("transaction has {0} inputs, max {1}")]
    TooManyInputs(usize, usize),
    
    /// Transaction has too many outputs
    #[error("transaction has {0} outputs, max {1}")]
    TooManyOutputs(usize, usize),
    
    /// Script size exceeds maximum
    #[error("script size {0} exceeds maximum {1}")]
    ScriptTooLarge(usize, usize),
    
    /// Duplicate input detected
    #[error("duplicate input detected")]
    DuplicateInput,
    
    /// Output value overflow
    #[error("output value overflow")]
    OutputValueOverflow,
    
    /// Negative or zero output value
    #[error("invalid output value")]
    InvalidOutputValue,
    
    /// Too many signature operations
    #[error("too many signature operations: {0} > {1}")]
    TooManySigOps(usize, usize),
    
    /// Block has no transactions
    #[error("block has no transactions")]
    EmptyBlock,
    
    /// Coinbase transaction invalid
    #[error("invalid coinbase transaction")]
    InvalidCoinbase,
    
    /// Duplicate transactions in block
    #[error("duplicate transactions in block")]
    DuplicateTransaction,
    
    /// Invalid timestamp (too far in future)
    #[error("timestamp {0} too far in future (max {1})")]
    TimestampTooFarInFuture(u32, u32),
    
    /// Invalid timestamp (before minimum)
    #[error("timestamp {0} before minimum {1}")]
    TimestampTooOld(u32, u32),
}

/// Validates a transaction for structural correctness
pub fn validate_transaction(tx: &Transaction) -> Result<(), ValidationError> {
    // Check size
    let size = tx.serialized_size_hint();
    if size > MAX_TX_SIZE {
        return Err(ValidationError::TransactionTooLarge(size, MAX_TX_SIZE));
    }
    
    // Check inputs
    if tx.inputs.is_empty() {
        return Err(ValidationError::NoInputs);
    }
    if tx.inputs.len() > MAX_TX_INPUTS {
        return Err(ValidationError::TooManyInputs(tx.inputs.len(), MAX_TX_INPUTS));
    }
    
    // Check outputs
    if tx.outputs.is_empty() {
        return Err(ValidationError::NoOutputs);
    }
    if tx.outputs.len() > MAX_TX_OUTPUTS {
        return Err(ValidationError::TooManyOutputs(tx.outputs.len(), MAX_TX_OUTPUTS));
    }
    
    // Check for duplicate inputs
    let mut seen_inputs = HashSet::new();
    for input in &tx.inputs {
        let outpoint = (&input.prev_txid, input.prev_vout);
        if !seen_inputs.insert(outpoint) {
            return Err(ValidationError::DuplicateInput);
        }
    }
    
    // Validate scripts and outputs
    let mut total_output: u64 = 0;
    for output in &tx.outputs {
        // Check script size
        if output.script_pubkey.len() > MAX_SCRIPT_SIZE {
            return Err(ValidationError::ScriptTooLarge(
                output.script_pubkey.len(),
                MAX_SCRIPT_SIZE,
            ));
        }
        
        // Check output value
        if output.value == 0 {
            return Err(ValidationError::InvalidOutputValue);
        }
        
        // Check for overflow
        total_output = total_output
            .checked_add(output.value)
            .ok_or(ValidationError::OutputValueOverflow)?;
    }
    
    // Check input scripts
    for input in &tx.inputs {
        if input.script_sig.len() > MAX_SCRIPT_SIZE {
            return Err(ValidationError::ScriptTooLarge(
                input.script_sig.len(),
                MAX_SCRIPT_SIZE,
            ));
        }
    }
    
    // Check signature operations count
    let sig_ops = tx.signature_count();
    if sig_ops > MAX_SIG_OPS_PER_TX {
        return Err(ValidationError::TooManySigOps(sig_ops, MAX_SIG_OPS_PER_TX));
    }
    
    Ok(())
}

/// Validates a block for structural correctness
pub fn validate_block_structure(block: &Block, current_time: u32) -> Result<(), ValidationError> {
    // Check block size
    let size = block.serialized_size_hint();
    if size > MAX_BLOCK_SIZE {
        return Err(ValidationError::BlockTooLarge(size, MAX_BLOCK_SIZE));
    }
    
    // Check block has transactions
    if block.transactions.is_empty() {
        return Err(ValidationError::EmptyBlock);
    }
    
    // Check timestamp not too far in future (2 hours)
    let max_future_time = current_time.saturating_add(7200);
    if block.header.time > max_future_time {
        return Err(ValidationError::TimestampTooFarInFuture(
            block.header.time,
            max_future_time,
        ));
    }
    
    // First transaction must be coinbase
    if !is_coinbase(&block.transactions[0]) {
        return Err(ValidationError::InvalidCoinbase);
    }
    
    // Only first transaction can be coinbase
    for tx in &block.transactions[1..] {
        if is_coinbase(tx) {
            return Err(ValidationError::InvalidCoinbase);
        }
    }
    
    // Check for duplicate transactions
    let mut seen_txids = HashSet::new();
    for tx in &block.transactions {
        let txid = tx.txid();
        if !seen_txids.insert(txid) {
            return Err(ValidationError::DuplicateTransaction);
        }
    }
    
    // Validate each transaction
    for tx in &block.transactions {
        validate_transaction(tx)?;
    }
    
    Ok(())
}

/// Checks if a transaction is a coinbase transaction
fn is_coinbase(tx: &Transaction) -> bool {
    if tx.inputs.len() != 1 {
        return false;
    }
    let input = &tx.inputs[0];
    input.prev_txid == [0u8; 32] && input.prev_vout == u32::MAX
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SigAlgorithm, TxIn, TxOut};

    #[test]
    fn rejects_empty_inputs() {
        let tx = Transaction {
            version: 1,
            lock_time: 0,
            inputs: vec![],
            outputs: vec![TxOut {
                value: 100,
                script_pubkey: vec![0x51],
            }],
            sig_algo: SigAlgorithm::Dilithium3,
            witnesses: vec![],
        };
        assert!(matches!(
            validate_transaction(&tx),
            Err(ValidationError::NoInputs)
        ));
    }

    #[test]
    fn rejects_empty_outputs() {
        let tx = Transaction {
            version: 1,
            lock_time: 0,
            inputs: vec![TxIn {
                prev_txid: [1u8; 32],
                prev_vout: 0,
                sequence: 0xffffffff,
                script_sig: vec![],
            }],
            outputs: vec![],
            sig_algo: SigAlgorithm::Dilithium3,
            witnesses: vec![],
        };
        assert!(matches!(
            validate_transaction(&tx),
            Err(ValidationError::NoOutputs)
        ));
    }

    #[test]
    fn rejects_duplicate_inputs() {
        let tx = Transaction {
            version: 1,
            lock_time: 0,
            inputs: vec![
                TxIn {
                    prev_txid: [1u8; 32],
                    prev_vout: 0,
                    sequence: 0xffffffff,
                    script_sig: vec![],
                },
                TxIn {
                    prev_txid: [1u8; 32],
                    prev_vout: 0,
                    sequence: 0xffffffff,
                    script_sig: vec![],
                },
            ],
            outputs: vec![TxOut {
                value: 100,
                script_pubkey: vec![0x51],
            }],
            sig_algo: SigAlgorithm::Dilithium3,
            witnesses: vec![],
        };
        assert!(matches!(
            validate_transaction(&tx),
            Err(ValidationError::DuplicateInput)
        ));
    }
}
