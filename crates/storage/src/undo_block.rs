use bitquan_types::{Transaction, TxOut};
use serde::{Deserialize, Serialize};

/// Represents the data needed to "undo" the effects of a single transaction
/// when a block is disconnected.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpentOutput {
    pub output: TxOut,
    pub prev_txid: [u8; 32],
    pub prev_vout: u32,
}

/// Represents all the data needed to "undo" the effects of an entire block
/// when it is disconnected from the main chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UndoBlock {
    pub spent_outputs: Vec<SpentOutput>,
}

impl UndoBlock {
    pub fn new() -> Self {
        Self {
            spent_outputs: Vec::new(),
        }
    }

    pub fn add_spent_output(&mut self, output: TxOut, prev_txid: [u8; 32], prev_vout: u32) {
        self.spent_outputs.push(SpentOutput {
            output,
            prev_txid,
            prev_vout,
        });
    }
}
