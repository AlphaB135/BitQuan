//! Transaction builder for wallet operations.

#![allow(dead_code)]

use anyhow::{bail, Result};
use bitquan_types::{SigAlgorithm, Transaction, TxIn, TxOut, Witness};

/// Builder for constructing transactions.
#[allow(dead_code)]
pub struct TransactionBuilder {
    version: i32,
    inputs: Vec<TxIn>,
    outputs: Vec<TxOut>,
    lock_time: u32,
}

impl TransactionBuilder {
    /// Creates a new transaction builder.
    pub fn new() -> Self {
        Self {
            version: 2,
            inputs: Vec::new(),
            outputs: Vec::new(),
            lock_time: 0,
        }
    }

    /// Sets the transaction version.
    pub fn version(mut self, version: i32) -> Self {
        self.version = version;
        self
    }

    /// Adds an input to the transaction.
    pub fn add_input(mut self, prev_txid: [u8; 32], prev_vout: u32, _value: u64) -> Self {
        let input = TxIn {
            prev_txid,
            prev_vout,
            script_sig: Vec::new(), // Will be filled during signing
            sequence: 0xffffffff,
        };
        self.inputs.push(input);
        self
    }

    /// Adds an output to the transaction.
    pub fn add_output(mut self, script_pubkey: Vec<u8>, value: u64) -> Self {
        let output = TxOut {
            value,
            script_pubkey,
        };
        self.outputs.push(output);
        self
    }

    /// Sets the lock time.
    pub fn lock_time(mut self, lock_time: u32) -> Self {
        self.lock_time = lock_time;
        self
    }

    /// Builds the unsigned transaction.
    pub fn build_unsigned(self) -> Result<Transaction> {
        if self.inputs.is_empty() {
            bail!("Transaction must have at least one input");
        }
        if self.outputs.is_empty() {
            bail!("Transaction must have at least one output");
        }

        Ok(Transaction {
            version: self.version,
            lock_time: self.lock_time,
            inputs: self.inputs,
            outputs: self.outputs,
            sig_algo: SigAlgorithm::Dilithium3,
            witnesses: Vec::new(),
        })
    }

    /// Builds and signs the transaction.
    pub fn build_and_sign(self, sign_fn: impl Fn(&[u8]) -> Result<Vec<u8>>) -> Result<Transaction> {
        let mut tx = self.build_unsigned()?;

        // Create witness for each input
        for i in 0..tx.inputs.len() {
            // Compute sighash for this input
            let sighash = compute_sighash(&tx, i)?;

            // Sign the sighash
            let signature = sign_fn(&sighash)?;

            // Create witness
            use bitquan_types::SignaturePayload;

            let witness = Witness {
                signatures: vec![SignaturePayload {
                    signer_index: i as u16,
                    signature: signature.clone(),
                    public_key: Vec::new(), // Filled by wallet
                    aux: None,
                }],
            };

            tx.witnesses.push(witness);
        }

        Ok(tx)
    }
}

impl Default for TransactionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Computes the signature hash for a transaction input.
#[allow(dead_code)]
pub fn compute_sighash(tx: &Transaction, input_index: usize) -> Result<[u8; 32]> {
    use sha2::Digest;

    if input_index >= tx.inputs.len() {
        bail!("Input index out of bounds");
    }

    // Simplified sighash (Bitcoin-style)
    // Hash: version || inputs || outputs || locktime || input_index

    let mut hasher = sha2::Sha256::new();

    // Version
    hasher.update(&tx.version.to_le_bytes());

    // Inputs (without script_sig)
    hasher.update(&(tx.inputs.len() as u32).to_le_bytes());
    for input in &tx.inputs {
        hasher.update(&input.prev_txid);
        hasher.update(&input.prev_vout.to_le_bytes());
        hasher.update(&input.sequence.to_le_bytes());
    }

    // Outputs
    hasher.update(&(tx.outputs.len() as u32).to_le_bytes());
    for output in &tx.outputs {
        hasher.update(&output.value.to_le_bytes());
        hasher.update(&(output.script_pubkey.len() as u32).to_le_bytes());
        hasher.update(&output.script_pubkey);
    }

    // Lock time
    hasher.update(&tx.lock_time.to_le_bytes());

    // Input index being signed
    hasher.update(&(input_index as u32).to_le_bytes());

    let result = hasher.finalize();
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&result);

    Ok(hash)
}

/// UTXO information for transaction building.
#[derive(Debug, Clone)]
pub struct Utxo {
    /// Transaction ID
    pub txid: [u8; 32],
    /// Output index
    pub vout: u32,
    /// Value in qbits
    pub value: u64,
    /// Script pubkey
    pub script_pubkey: Vec<u8>,
}

impl Utxo {
    /// Creates a new UTXO.
    pub fn new(txid: [u8; 32], vout: u32, value: u64, script_pubkey: Vec<u8>) -> Self {
        Self {
            txid,
            vout,
            value,
            script_pubkey,
        }
    }
}

/// Coin selection strategy.
pub enum CoinSelection {
    /// Select oldest coins first
    OldestFirst,
    /// Select largest coins first
    LargestFirst,
    /// Select smallest coins that cover the amount
    SmallestSufficient,
}

/// Selects UTXOs to spend for a transaction.
pub fn select_coins(
    utxos: &[Utxo],
    target_amount: u64,
    fee_rate: u64,
    strategy: CoinSelection,
) -> Result<Vec<Utxo>> {
    if utxos.is_empty() {
        bail!("No UTXOs available");
    }

    let mut available = utxos.to_vec();

    // Sort based on strategy
    match strategy {
        CoinSelection::OldestFirst => {
            // Already in order (assumed)
        }
        CoinSelection::LargestFirst => {
            available.sort_by(|a, b| b.value.cmp(&a.value));
        }
        CoinSelection::SmallestSufficient => {
            available.sort_by(|a, b| a.value.cmp(&b.value));
        }
    }

    let mut selected = Vec::new();
    let mut total = 0u64;

    // Estimate fee (simplified)
    // Each input ~100 bytes + Dilithium sig ~3000 bytes
    // Each output ~50 bytes
    let base_fee = fee_rate * 100; // Base tx size

    for utxo in available {
        selected.push(utxo.clone());
        total += utxo.value;

        // Calculate current fee estimate
        let input_fee = fee_rate * (selected.len() as u64 * 3100); // 100 + 3000 per input
        let total_needed = target_amount + base_fee + input_fee;

        if total >= total_needed {
            return Ok(selected);
        }
    }

    bail!(
        "Insufficient funds: need {} qbits, have {} qbits",
        target_amount,
        total
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transaction_builder() {
        let tx = TransactionBuilder::new()
            .add_input([0x01; 32], 0, 50_000_000)
            .add_output(vec![0x76, 0xa9, 0x14], 40_000_000)
            .build_unsigned()
            .unwrap();

        assert_eq!(tx.inputs.len(), 1);
        assert_eq!(tx.outputs.len(), 1);
        assert_eq!(tx.version, 2);
    }

    #[test]
    fn test_compute_sighash() {
        let tx = TransactionBuilder::new()
            .add_input([0x42; 32], 0, 100)
            .add_output(vec![0x00], 50)
            .build_unsigned()
            .unwrap();

        let hash = compute_sighash(&tx, 0).unwrap();
        assert_ne!(hash, [0u8; 32]);
    }

    #[test]
    fn test_coin_selection() {
        let utxos = vec![
            Utxo::new([0x01; 32], 0, 10_000_000, vec![]),
            Utxo::new([0x02; 32], 0, 20_000_000, vec![]),
            Utxo::new([0x03; 32], 0, 30_000_000, vec![]),
        ];

        let selected =
            select_coins(&utxos, 25_000_000, 1, CoinSelection::SmallestSufficient).unwrap();
        assert!(selected.len() >= 1);

        let total: u64 = selected.iter().map(|u| u.value).sum();
        assert!(total >= 25_000_000);
    }

    #[test]
    fn test_coin_selection_insufficient() {
        let utxos = vec![Utxo::new([0x01; 32], 0, 10_000_000, vec![])];

        let result = select_coins(&utxos, 50_000_000, 1, CoinSelection::LargestFirst);
        assert!(result.is_err());
    }
}
