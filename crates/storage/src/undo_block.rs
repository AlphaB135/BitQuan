use bitquan_types::TxOut;
use serde::{Deserialize, Serialize};

/// Represents the data needed to "undo" the effects of a single transaction
/// when a block is disconnected.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpentOutput {
    /// The output that was spent
    pub output: TxOut,
    /// The transaction ID that contained this output
    pub prev_txid: [u8; 32],
    /// The output index in the previous transaction
    pub prev_vout: u32,
    /// Block height where this output was created (for coinbase maturity)
    pub height: u64,
    /// Whether this is a coinbase output (for maturity enforcement)
    pub is_coinbase: bool,
}

/// Represents all the data needed to "undo" the effects of an entire block
/// when it is disconnected from the main chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UndoBlock {
    /// List of all outputs that were spent in this block
    pub spent_outputs: Vec<SpentOutput>,
}

impl Default for UndoBlock {
    fn default() -> Self {
        Self::new()
    }
}

impl UndoBlock {
    /// Create a new empty undo block
    pub fn new() -> Self {
        Self {
            spent_outputs: Vec::new(),
        }
    }

    /// Add a spent output to this undo block
    pub fn add_spent_output(
        &mut self,
        output: TxOut,
        prev_txid: [u8; 32],
        prev_vout: u32,
        height: u64,
        is_coinbase: bool,
    ) {
        self.spent_outputs.push(SpentOutput {
            output,
            prev_txid,
            prev_vout,
            height,
            is_coinbase,
        });
    }
}
