//! Compact unsigned integer encoding used across BitQuan wire formats.

use serde::{Deserialize, Serialize};

/// Represents a CompactSize-encoded unsigned integer (Bitcoin style).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct CompactUint(u64);

impl CompactUint {
    /// Creates a new compact integer.
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Returns the underlying numeric value.
    pub const fn value(self) -> u64 {
        self.0
    }

    /// Returns the encoded length when serialized according to CompactSize rules.
    pub const fn encoded_length(self) -> usize {
        match self.0 {
            0..=0xFC => 1,
            0xFD..=0xFFFF => 3,
            0x1_0000..=0xFFFF_FFFF => 5,
            _ => 9,
        }
    }

    /// Constructs from a `usize`.
    pub fn from_usize(value: usize) -> Self {
        Self(value as u64)
    }
}

impl From<u64> for CompactUint {
    fn from(value: u64) -> Self {
        Self::new(value)
    }
}

impl From<CompactUint> for u64 {
    fn from(value: CompactUint) -> Self {
        value.value()
    }
}
