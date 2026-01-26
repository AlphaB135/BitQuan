//! Transaction builder for wallet operations.

#![allow(dead_code)]

use bitquan_types::error::{Error, Result};
use bitquan_types::{
    genesis::GENESIS_HASH_BYTES, NetworkId, SigAlgorithm, Transaction, TxContext, TxIn, TxOut,
    Witness,
};

/// Builder for constructing transactions.
pub struct TransactionBuilder {
    version: i32,
    inputs: Vec<TxIn>,
    outputs: Vec<TxOut>,
    lock_time: u32,
    network: NetworkId,
    genesis_hash: [u8; 32],
    ctx: TxContext,
}

impl TransactionBuilder {
    /// Creates a new transaction builder.
    pub fn new() -> Self {
        let network = NetworkId::Devnet;
        let genesis_hash = GENESIS_HASH_BYTES;
        Self {
            version: 2,
            inputs: Vec::new(),
            outputs: Vec::new(),
            lock_time: 0,
            network,
            genesis_hash,
            ctx: TxContext::new(network, genesis_hash),
        }
    }

    /// Sets the transaction version.
    pub fn version(mut self, version: i32) -> Self {
        self.version = version;
        self
    }

    /// Adds an input to the transaction.
    pub fn add_input(mut self, prev_txid: [u8; 32], prev_vout: u32, _value: u128) -> Self {
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
    pub fn add_output(mut self, script_pubkey: Vec<u8>, value: u128) -> Self {
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

    /// Sets the target network identifier.
    pub fn network(mut self, network: NetworkId) -> Self {
        self.network = network;
        self.ctx = TxContext::new(network, self.genesis_hash);
        self
    }

    /// Sets the genesis hash used for replay protection.
    pub fn genesis_hash(mut self, genesis_hash: [u8; 32]) -> Self {
        self.genesis_hash = genesis_hash;
        self.ctx = TxContext::new(self.network, genesis_hash);
        self
    }

    /// Sets the transaction context directly.
    pub fn with_context(mut self, ctx: TxContext) -> Self {
        self.network = ctx.network_id;
        self.genesis_hash = ctx.genesis_hash;
        self.ctx = ctx;
        self
    }

    /// Builds the unsigned transaction.
    pub fn build_unsigned(self) -> Result<Transaction> {
        if self.inputs.is_empty() {
            return Err(Error::Invalid(
                "Transaction must have at least one input".to_string(),
            ));
        }
        if self.outputs.is_empty() {
            return Err(Error::Invalid(
                "Transaction must have at least one output".to_string(),
            ));
        }

        Ok(Transaction {
            version: self.version,
            network: self.network,
            genesis_hash: self.genesis_hash,
            lock_time: self.lock_time,
            inputs: self.inputs,
            outputs: self.outputs,
            sig_algo: SigAlgorithm::Dilithium5,
            witnesses: Vec::new(),
        })
    }

    /// Builds and signs the transaction.
    pub fn build_and_sign(self, sign_fn: impl Fn(&[u8]) -> Result<Vec<u8>>) -> Result<Transaction> {
        let ctx = self.ctx.clone();
        let mut tx = self.build_unsigned()?;

        // Create witness for each input
        for i in 0..tx.inputs.len() {
            // Compute sighash using transaction_sighash from consensus
            let sighash = compute_sighash_with_context(&tx, &ctx, i)?;

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

/// Convenience helper for building a simple unsigned transaction.
pub fn build_simple_tx(
    ctx: &TxContext,
    inputs: &[([u8; 32], u32, u128)],
    outputs: &[(Vec<u8>, u128)],
) -> Result<Transaction> {
    if inputs.is_empty() {
        return Err(Error::Invalid("no inputs".to_string()));
    }
    if outputs.is_empty() {
        return Err(Error::Invalid("no outputs".to_string()));
    }

    let mut builder = TransactionBuilder::new().with_context(ctx.clone());

    let mut total_in: u128 = 0;
    for &(prev_txid, prev_vout, value) in inputs {
        builder = builder.add_input(prev_txid, prev_vout, value);
        total_in = total_in
            .checked_add(value)
            .ok_or(Error::Overflow("input sum overflow"))?;
    }

    let mut total_out: u128 = 0;
    for (script, value) in outputs {
        builder = builder.add_output(script.clone(), *value);
        total_out = total_out
            .checked_add(*value)
            .ok_or(Error::Overflow("output sum overflow"))?;
    }

    if total_out > total_in {
        return Err(Error::Invalid("insufficient input amount".to_string()));
    }

    builder
        .build_unsigned()
        .map_err(|e| Error::Invalid(e.to_string()))
}

/// Computes the signature hash for a transaction input using TxContext.
///
/// This function wraps the consensus transaction_sighash and adds per-input
/// differentiation by hashing the input index into the result.
pub fn compute_sighash_with_context(
    tx: &Transaction,
    ctx: &TxContext,
    input_index: usize,
) -> Result<[u8; 32]> {
    use sha2::Digest;

    if input_index >= tx.inputs.len() {
        return Err(Error::Invalid("Input index out of bounds".to_string()));
    }

    // Use consensus transaction_sighash
    let base_hash = bitquan_consensus::transaction_sighash(tx, ctx)
        .map_err(|e| Error::Invalid(format!("Sighash error: {}", e)))?;

    // For per-input signing, hash the base sighash with the input index
    // This allows each input to have a unique signature
    let mut hasher = sha2::Sha256::new();
    hasher.update(base_hash);
    hasher.update((input_index as u32).to_le_bytes());

    let result = hasher.finalize();
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&result);

    Ok(hash)
}

/// Computes the signature hash for a transaction input (legacy version).
///
/// DEPRECATED: Use compute_sighash_with_context instead.
/// This function is kept for backward compatibility.
#[deprecated(note = "Use compute_sighash_with_context with TxContext")]
pub fn compute_sighash(tx: &Transaction, input_index: usize) -> Result<[u8; 32]> {
    let ctx = TxContext::new(tx.network, tx.genesis_hash);
    compute_sighash_with_context(tx, &ctx, input_index)
}

/// UTXO information for transaction building.
#[derive(Debug, Clone)]
pub struct Utxo {
    /// Transaction ID
    pub txid: [u8; 32],
    /// Output index
    pub vout: u32,
    /// Value in qbits
    pub value: u128,
    /// Script pubkey
    pub script_pubkey: Vec<u8>,
}

impl Utxo {
    /// Creates a new UTXO.
    pub fn new(txid: [u8; 32], vout: u32, value: u128, script_pubkey: Vec<u8>) -> Self {
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
    target_amount: u128,
    fee_rate: u64,
    strategy: CoinSelection,
) -> Result<Vec<Utxo>> {
    if utxos.is_empty() {
        return Err(Error::Invalid("No UTXOs available".to_string()));
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
    let mut total = 0u128;

    // Estimate fee (simplified)
    // Each input ~100 bytes + Dilithium sig ~3000 bytes
    // Each output ~50 bytes
    let base_fee = fee_rate.saturating_mul(100); // Base tx size

    for utxo in available {
        selected.push(utxo.clone());
        total = total.saturating_add(utxo.value);

        // Calculate current fee estimate using saturating arithmetic
        let input_fee =
            fee_rate.saturating_mul((selected.len() as u64).saturating_mul(3100)) as u128;
        let total_needed = target_amount
            .saturating_add(base_fee as u128)
            .saturating_add(input_fee);

        if total >= total_needed {
            return Ok(selected);
        }
    }

    Err(Error::Invalid(format!(
        "Insufficient funds: need {} qbits, have {} qbits",
        target_amount, total
    )))
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use bitquan_types::error::Error;

    #[test]
    fn test_transaction_builder() {
        let tx = TransactionBuilder::new()
            .add_input([0x01; 32], 0, 50_000_000)
            .add_output(vec![0x76, 0xa9, 0x14], 40_000_000)
            .build_unsigned()
            .expect("Failed to build unsigned transaction");

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
            .expect("Failed to build unsigned transaction");

        let ctx = TxContext::new(tx.network, tx.genesis_hash);
        let hash = compute_sighash_with_context(&tx, &ctx, 0).expect("Failed to compute sighash");
        assert_ne!(hash, [0u8; 32]);
    }

    #[test]
    fn test_builder_with_context() {
        let ctx = TxContext::mainnet(GENESIS_HASH_BYTES);
        let tx = TransactionBuilder::new()
            .with_context(ctx.clone())
            .add_input([0x42; 32], 0, 100)
            .add_output(vec![0x00], 50)
            .build_unsigned()
            .expect("Failed to build unsigned transaction with context");

        assert_eq!(tx.network, NetworkId::Mainnet);
        assert_eq!(tx.genesis_hash, GENESIS_HASH_BYTES);
    }

    #[test]
    fn test_context_mismatch_detection() {
        // Create transaction with devnet context
        let tx = TransactionBuilder::new()
            .network(NetworkId::Devnet)
            .add_input([0x42; 32], 0, 100)
            .add_output(vec![0x00], 50)
            .build_unsigned()
            .expect("Failed to build unsigned transaction for devnet");

        // Try to compute sighash with mainnet context
        let wrong_ctx = TxContext::mainnet(GENESIS_HASH_BYTES);
        let result = compute_sighash_with_context(&tx, &wrong_ctx, 0);

        // Should fail due to network mismatch
        assert!(result.is_err());
    }

    #[test]
    fn test_different_networks_different_sighash() {
        let input = [0x42; 32];
        let output = vec![0x00];

        // Build transaction for devnet
        let tx_devnet = TransactionBuilder::new()
            .network(NetworkId::Devnet)
            .add_input(input, 0, 100)
            .add_output(output.clone(), 50)
            .build_unsigned()
            .expect("Failed to build unsigned transaction for devnet");

        // Build transaction for mainnet (same data, different network)
        let tx_mainnet = TransactionBuilder::new()
            .network(NetworkId::Mainnet)
            .add_input(input, 0, 100)
            .add_output(output, 50)
            .build_unsigned()
            .expect("Failed to build unsigned transaction for mainnet");

        let ctx_devnet = TxContext::new(NetworkId::Devnet, GENESIS_HASH_BYTES);
        let ctx_mainnet = TxContext::new(NetworkId::Mainnet, GENESIS_HASH_BYTES);

        let hash_devnet = compute_sighash_with_context(&tx_devnet, &ctx_devnet, 0)
            .expect("Failed to compute devnet sighash");
        let hash_mainnet = compute_sighash_with_context(&tx_mainnet, &ctx_mainnet, 0)
            .expect("Failed to compute mainnet sighash");

        // Same transaction data but different networks should produce different hashes
        assert_ne!(hash_devnet, hash_mainnet);
    }

    #[test]
    fn test_different_genesis_different_sighash() {
        let input = [0x42; 32];
        let output = vec![0x00];
        let genesis1 = [0xAA; 32];
        let genesis2 = [0xBB; 32];

        // Build transaction with genesis1
        let tx1 = TransactionBuilder::new()
            .genesis_hash(genesis1)
            .add_input(input, 0, 100)
            .add_output(output.clone(), 50)
            .build_unsigned()
            .expect("Failed to build unsigned transaction with genesis1");

        // Build transaction with genesis2
        let tx2 = TransactionBuilder::new()
            .genesis_hash(genesis2)
            .add_input(input, 0, 100)
            .add_output(output, 50)
            .build_unsigned()
            .expect("Failed to build unsigned transaction with genesis2");

        let ctx1 = TxContext::new(NetworkId::Devnet, genesis1);
        let ctx2 = TxContext::new(NetworkId::Devnet, genesis2);

        let hash1 = compute_sighash_with_context(&tx1, &ctx1, 0)
            .expect("Failed to compute sighash with genesis1");
        let hash2 = compute_sighash_with_context(&tx2, &ctx2, 0)
            .expect("Failed to compute sighash with genesis2");

        // Same transaction data but different genesis should produce different hashes
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn build_simple_tx_no_inputs_errs() {
        let ctx = TxContext::devnet(GENESIS_HASH_BYTES);
        let result = build_simple_tx(&ctx, &[], &[(vec![0x51], 1)]);
        assert!(matches!(result, Err(Error::Invalid(msg)) if msg == "no inputs"));
    }

    #[test]
    fn build_simple_tx_insufficient_amount_errs() {
        let ctx = TxContext::devnet(GENESIS_HASH_BYTES);
        let inputs = [([0xAA; 32], 0, 1)];
        let outputs = [(vec![0x51], 2)];
        let result = build_simple_tx(&ctx, &inputs, &outputs);
        assert!(matches!(result, Err(Error::Invalid(msg)) if msg == "insufficient input amount"));
    }

    #[test]
    fn build_simple_tx_success() {
        let ctx = TxContext::devnet(GENESIS_HASH_BYTES);
        let inputs = [([0xAA; 32], 0, 5)];
        let outputs = [(vec![0x51], 3)];
        let tx = build_simple_tx(&ctx, &inputs, &outputs).expect("tx");
        assert_eq!(tx.inputs.len(), 1);
        assert_eq!(tx.outputs.len(), 1);
        assert_eq!(tx.outputs[0].value, 3);
    }

    #[test]
    fn test_coin_selection() {
        let utxos = vec![
            Utxo::new([0x01; 32], 0, 10_000_000, vec![]),
            Utxo::new([0x02; 32], 0, 20_000_000, vec![]),
            Utxo::new([0x03; 32], 0, 30_000_000, vec![]),
        ];

        let selected = select_coins(&utxos, 25_000_000, 1, CoinSelection::SmallestSufficient)
            .expect("Failed to select coins");
        assert!(!selected.is_empty());

        // Use saturating_add to prevent overflow when summing coin values
        let total: u128 = selected
            .iter()
            .fold(0u128, |acc, u| acc.saturating_add(u.value));
        assert!(total >= 25_000_000);
    }

    #[test]
    fn test_coin_selection_insufficient() {
        let utxos = vec![Utxo::new([0x01; 32], 0, 10_000_000, vec![])];

        let result = select_coins(&utxos, 50_000_000, 1, CoinSelection::LargestFirst);
        assert!(result.is_err());
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod overflow_tests {
    use super::*;

    #[test]
    fn test_extreme_value_utxos() {
        // Coins with very large values should not cause overflow
        let utxos = vec![
            Utxo::new([0xFF; 32], 0, (u64::MAX - 1000) as u128, vec![]),
            Utxo::new([0xFE; 32], 0, (u64::MAX - 2000) as u128, vec![]),
        ];

        let result = select_coins(&utxos, 1_000_000, 1, CoinSelection::LargestFirst);
        assert!(result.is_ok());

        // Should select one coin (sufficient)
        let selected = result.expect("Failed to get selected coins");
        assert!(!selected.is_empty());
    }

    #[test]
    fn test_extreme_fee_rate() {
        let utxos = vec![Utxo::new(
            [0x01; 32],
            0,
            1_000_000_000_000_000_000_000,
            vec![],
        )]; // 1000 BQ (18 decimals)

        // Very high fee rate should saturate, not overflow
        let result = select_coins(
            &utxos,
            10_000,
            u64::MAX / 1000,
            CoinSelection::SmallestSufficient,
        );

        // Key: should not panic; may succeed or fail gracefully
        drop(result);
    }

    #[test]
    fn test_many_small_utxos() {
        // Many UTXOs to test loop saturation
        let utxos: Vec<_> = (0..5000)
            .map(|i| {
                let mut txid = [0u8; 32];
                txid[0] = (i % 256) as u8;
                txid[1] = (i / 256) as u8;
                Utxo::new(txid, 0, 100_000, vec![])
            })
            .collect();

        // Should handle gracefully - either succeed or fail due to overflow protection
        let result = select_coins(&utxos, 50_000_000, 100, CoinSelection::LargestFirst);
        // Accept both Ok and Err as valid outcomes (overflow protection may kick in)
        drop(result);
    }
}
