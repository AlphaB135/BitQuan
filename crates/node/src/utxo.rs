//! UTXO set management and validation.

use anyhow::{bail, Result};
use bitquan_types::{Transaction, TxOut};
use std::collections::HashMap;

/// A single unspent transaction output.
#[derive(Debug, Clone)]
pub struct Utxo {
    /// Transaction ID
    pub txid: [u8; 32],
    /// Output index
    pub vout: u32,
    /// Output data
    pub output: TxOut,
    /// Block height where this UTXO was created
    pub height: u64,
    /// Whether this is a coinbase output
    pub is_coinbase: bool,
}

impl Utxo {
    /// Creates a new UTXO.
    pub fn new(txid: [u8; 32], vout: u32, output: TxOut, height: u64, is_coinbase: bool) -> Self {
        Self {
            txid,
            vout,
            output,
            height,
            is_coinbase,
        }
    }

    /// Returns the value of this UTXO.
    pub fn value(&self) -> u64 {
        self.output.value
    }

    /// Returns the script pubkey.
    pub fn script_pubkey(&self) -> &[u8] {
        &self.output.script_pubkey
    }
}

/// In-memory UTXO set for validation.
pub struct UtxoSet {
    /// Map of (txid, vout) -> UTXO
    utxos: HashMap<([u8; 32], u32), Utxo>,
}

impl UtxoSet {
    /// Creates a new empty UTXO set.
    pub fn new() -> Self {
        Self {
            utxos: HashMap::new(),
        }
    }

    /// Adds a UTXO to the set.
    pub fn add(&mut self, utxo: Utxo) {
        self.utxos.insert((utxo.txid, utxo.vout), utxo);
    }

    /// Removes a UTXO from the set.
    pub fn remove(&mut self, txid: &[u8; 32], vout: u32) -> Option<Utxo> {
        self.utxos.remove(&(*txid, vout))
    }

    /// Gets a UTXO if it exists.
    pub fn get(&self, txid: &[u8; 32], vout: u32) -> Option<&Utxo> {
        self.utxos.get(&(*txid, vout))
    }

    /// Checks if a UTXO exists.
    pub fn contains(&self, txid: &[u8; 32], vout: u32) -> bool {
        self.utxos.contains_key(&(*txid, vout))
    }

    /// Returns the number of UTXOs.
    pub fn len(&self) -> usize {
        self.utxos.len()
    }

    /// Checks if the set is empty.
    pub fn is_empty(&self) -> bool {
        self.utxos.is_empty()
    }

    /// Gets all UTXOs for a given script pubkey.
    pub fn get_by_script(&self, script_pubkey: &[u8]) -> Vec<&Utxo> {
        self.utxos
            .values()
            .filter(|utxo| utxo.script_pubkey() == script_pubkey)
            .collect()
    }

    /// Calculates the total value for a script pubkey.
    pub fn balance(&self, script_pubkey: &[u8]) -> u64 {
        self.get_by_script(script_pubkey)
            .iter()
            .map(|utxo| utxo.value())
            .sum()
    }

    /// Applies a transaction to the UTXO set.
    pub fn apply_transaction(&mut self, tx: &Transaction, height: u64) -> Result<()> {
        // Remove spent outputs
        for input in &tx.inputs {
            if input.prev_txid == [0u8; 32] {
                // Coinbase input, skip
                continue;
            }

            if !self.contains(&input.prev_txid, input.prev_vout) {
                bail!(
                    "Input references non-existent UTXO: {}:{}",
                    hex::encode(&input.prev_txid),
                    input.prev_vout
                );
            }

            self.remove(&input.prev_txid, input.prev_vout);
        }

        // Add new outputs
        let txid = tx.txid();
        let is_coinbase = !tx.inputs.is_empty() && tx.inputs[0].prev_txid == [0u8; 32];

        for (vout, output) in tx.outputs.iter().enumerate() {
            let utxo = Utxo::new(txid, vout as u32, output.clone(), height, is_coinbase);
            self.add(utxo);
        }

        Ok(())
    }

    /// Validates that all inputs in a transaction are spendable.
    pub fn validate_transaction(&self, tx: &Transaction, height: u64) -> Result<()> {
        let mut total_input_value = 0u64;
        let mut total_output_value = 0u64;

        // Check inputs
        for input in &tx.inputs {
            // Skip coinbase inputs
            if input.prev_txid == [0u8; 32] {
                continue;
            }

            // Get the UTXO
            let utxo = self.get(&input.prev_txid, input.prev_vout).ok_or_else(|| {
                anyhow::anyhow!(
                    "Input references non-existent UTXO: {}:{}",
                    hex::encode(&input.prev_txid),
                    input.prev_vout
                )
            })?;

            // Check coinbase maturity (100 blocks)
            if utxo.is_coinbase {
                if height < utxo.height + 100 {
                    bail!(
                        "Coinbase output not mature: created at {}, current {}",
                        utxo.height,
                        height
                    );
                }
            }

            total_input_value = total_input_value
                .checked_add(utxo.value())
                .ok_or_else(|| anyhow::anyhow!("Input value overflow"))?;
        }

        // Check outputs
        for output in &tx.outputs {
            total_output_value = total_output_value
                .checked_add(output.value)
                .ok_or_else(|| anyhow::anyhow!("Output value overflow"))?;
        }

        // Validate fees (inputs >= outputs)
        if total_input_value < total_output_value {
            bail!(
                "Outputs exceed inputs: {} < {}",
                total_input_value,
                total_output_value
            );
        }

        Ok(())
    }

    /// Clears all UTXOs.
    pub fn clear(&mut self) {
        self.utxos.clear();
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
        prev_txid: [u8; 32],
        prev_vout: u32,
        outputs: Vec<(u64, Vec<u8>)>,
    ) -> Transaction {
        let inputs = vec![TxIn {
            prev_txid,
            prev_vout,
            script_sig: vec![],
            sequence: 0xffffffff,
        }];

        let outputs = outputs
            .into_iter()
            .map(|(value, script)| TxOut {
                value,
                script_pubkey: script,
            })
            .collect();

        Transaction {
            version: 2,
            lock_time: 0,
            inputs,
            outputs,
            sig_algo: SigAlgorithm::Dilithium3,
            witnesses: vec![],
        }
    }

    #[test]
    fn test_utxo_set_basic() {
        let mut set = UtxoSet::new();

        let utxo = Utxo::new(
            [0x01; 32],
            0,
            TxOut {
                value: 100,
                script_pubkey: vec![0x76, 0xa9],
            },
            1,
            false,
        );

        set.add(utxo);
        assert_eq!(set.len(), 1);
        assert!(set.contains(&[0x01; 32], 0));
    }

    #[test]
    fn test_utxo_set_remove() {
        let mut set = UtxoSet::new();

        let utxo = Utxo::new(
            [0x01; 32],
            0,
            TxOut {
                value: 100,
                script_pubkey: vec![],
            },
            1,
            false,
        );
        set.add(utxo);

        let removed = set.remove(&[0x01; 32], 0);
        assert!(removed.is_some());
        assert_eq!(set.len(), 0);
    }

    #[test]
    fn test_apply_transaction() {
        let mut set = UtxoSet::new();

        // Add initial UTXO
        let utxo = Utxo::new(
            [0x01; 32],
            0,
            TxOut {
                value: 100,
                script_pubkey: vec![],
            },
            1,
            false,
        );
        set.add(utxo);

        // Create transaction spending it
        let tx = create_test_tx([0x01; 32], 0, vec![(50, vec![0x00]), (40, vec![0x01])]);

        set.apply_transaction(&tx, 2).unwrap();

        // Original UTXO should be removed
        assert!(!set.contains(&[0x01; 32], 0));

        // New UTXOs should be added
        let txid = tx.txid();
        assert!(set.contains(&txid, 0));
        assert!(set.contains(&txid, 1));
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_validate_transaction() {
        let mut set = UtxoSet::new();

        // Add UTXO
        let utxo = Utxo::new(
            [0x01; 32],
            0,
            TxOut {
                value: 100,
                script_pubkey: vec![],
            },
            1,
            false,
        );
        set.add(utxo);

        // Valid transaction
        let tx = create_test_tx([0x01; 32], 0, vec![(90, vec![0x00])]);
        assert!(set.validate_transaction(&tx, 10).is_ok());

        // Invalid: outputs exceed inputs
        let bad_tx = create_test_tx([0x01; 32], 0, vec![(150, vec![0x00])]);
        assert!(set.validate_transaction(&bad_tx, 10).is_err());
    }

    #[test]
    fn test_coinbase_maturity() {
        let mut set = UtxoSet::new();

        // Add coinbase UTXO at height 1
        let utxo = Utxo::new(
            [0x01; 32],
            0,
            TxOut {
                value: 5000000000,
                script_pubkey: vec![],
            },
            1,
            true,
        );
        set.add(utxo);

        let tx = create_test_tx([0x01; 32], 0, vec![(4000000000, vec![0x00])]);

        // Too early (height 50 < 101)
        assert!(set.validate_transaction(&tx, 50).is_err());

        // Mature enough (height 101 >= 101)
        assert!(set.validate_transaction(&tx, 101).is_ok());
    }

    #[test]
    fn test_balance_calculation() {
        let mut set = UtxoSet::new();
        let script = vec![0x76, 0xa9];

        let utxo1 = Utxo::new(
            [0x01; 32],
            0,
            TxOut {
                value: 100,
                script_pubkey: script.clone(),
            },
            1,
            false,
        );
        let utxo2 = Utxo::new(
            [0x02; 32],
            0,
            TxOut {
                value: 200,
                script_pubkey: script.clone(),
            },
            2,
            false,
        );
        let utxo3 = Utxo::new(
            [0x03; 32],
            0,
            TxOut {
                value: 50,
                script_pubkey: vec![0x00],
            },
            3,
            false,
        );

        set.add(utxo1);
        set.add(utxo2);
        set.add(utxo3);

        assert_eq!(set.balance(&script), 300);
    }
}
