//! BitQuan State Channels Module
//!
//! This module implements state channel technology for off-chain transaction processing,
//! enabling fast, low-cost transactions with periodic on-chain settlement.

pub mod payment_channel;
pub mod multi_party;
pub mod channel_state;
pub mod dispute_resolution;
pub mod settlement;

pub use payment_channel::{PaymentChannel, ChannelState, ChannelStateMachine};
pub use multi_party::{MultiPartyChannel, MultiPartyState};
pub use channel_state::{ChannelStatus, ChannelUpdate};
pub use dispute_resolution::{DisputeResolution, DisputeType};
pub use settlement::{ChannelSettlement, SettlementTransaction};

/// Channel configuration
#[derive(Debug, Clone)]
pub struct ChannelConfig {
    /// Minimum channel duration (blocks)
    pub min_duration: u64,
    /// Maximum channel duration (blocks)
    pub max_duration: u64,
    /// Minimum opening amount
    pub min_open_amount: u64,
    /// Maximum channel size
    pub max_channel_size: u64,
    /// Dispute resolution timeout
    pub dispute_timeout: std::time::Duration,
    /// Settlement fee
    pub settlement_fee: u64,
}

impl Default for ChannelConfig {
    fn default() -> Self {
        Self {
            min_duration: 100,
            max_duration: 100000,
            min_open_amount: 1000000, // 0.001 BQN
            max_channel_size: 1000000000, // 1000 BQN
            dispute_timeout: std::time::Duration::from_secs(3600),
            settlement_fee: 1000, // 0.001 BQN
        }
    }
}

/// Channel ID type
pub type ChannelId = [u8; 32];

/// Participant in a channel
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Participant {
    pub address: [u8; 32],
    pub public_key: [u8; 32],
    pub is_initiator: bool,
}

impl Participant {
    pub fn new(address: [u8; 32], public_key: [u8; 32]) -> Self {
        Self {
            address,
            public_key,
            is_initiator: false,
        }
    }

    pub fn as_initiator(mut self) -> Self {
        self.is_initiator = true;
        self
    }
}

/// Channel errors
#[derive(Debug, thiserror::Error)]
pub enum ChannelError {
    #[error("Insufficient balance")]
    InsufficientBalance,
    #[error("Invalid channel state transition")]
    InvalidStateTransition,
    #[error("Channel timeout")]
    ChannelTimeout,
    #[error("Invalid signature")]
    InvalidSignature,
    #[error("Dispute in progress")]
    DisputeInProgress,
    #[error("Channel closed")]
    ChannelClosed,
    #[error("Amount too small")]
    AmountTooSmall,
    #[error("Invalid participants")]
    InvalidParticipants,
}

/// Result type for channel operations
pub type ChannelResult<T> = Result<T, ChannelError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_channel_config() {
        let config = ChannelConfig::default();
        assert_eq!(config.min_duration, 100);
        assert_eq!(config.max_duration, 100000);
        assert_eq!(config.min_open_amount, 1000000);
        assert_eq!(config.max_channel_size, 1000000000);
    }

    #[test]
    fn test_participant_creation() {
        let addr = [1u8; 32];
        let pk = [2u8; 32];
        let participant = Participant::new(addr, pk);
        assert_eq!(participant.address, addr);
        assert_eq!(participant.public_key, pk);
        assert!(!participant.is_initiator);
    }
}