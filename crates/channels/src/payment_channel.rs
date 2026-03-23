//! Payment Channels - Fast off-chain transactions with on-chain settlement

use crate::{ChannelConfig, ChannelError, ChannelResult, ChannelId, Participant};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use std::time::{Duration, Instant};
use bitquan_types::{Transaction, TransactionOutput, TransactionInput};

/// State of a payment channel
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChannelState {
    /// Channel is open and ready for transactions
    Open {
        balances: HashMap<[u8; 32], u64>,
        state_root: [u8; 32],
        opening_block: u64,
    },
    /// Channel has updates pending settlement
    Update {
        pending_updates: Vec<ChannelUpdate>,
        last_state_root: [u8; 32],
    },
    /// Channel is being closed
    Close {
        initiator: Participant,
        closing_block: u64,
    },
    /// Channel is being settled
    Settle {
        final_balances: HashMap<[u8; 32], u64>,
        settlement_tx: Option<Transaction>,
    },
    /// Channel is finalized
    Finalized {
        final_balances: HashMap<[u8; 32], u64>,
    },
}

/// Channel update
#[derive(Debug, Clone)]
pub struct ChannelUpdate {
    pub sequence: u64,
    pub from: [u8; 32],
    pub to: [u8; 32],
    pub amount: u64,
    pub new_balance_from: u64,
    pub new_balance_to: u64,
    pub signature: [u8; 64],
    pub timestamp: Instant,
}

/// Payment channel implementation
pub struct PaymentChannel {
    pub id: ChannelId,
    pub participants: Vec<Participant>,
    pub state: ChannelState,
    pub config: ChannelConfig,
    pub state_machine: ChannelStateMachine,
    pub timeout_block: u64,
    pub updates: Vec<ChannelUpdate>,
}

/// State machine for channel transitions
pub struct ChannelStateMachine {
    pub current_state: ChannelState,
    pub state_history: Vec<(u64, ChannelState)>,
    pub dispute_window: Duration,
}

impl PaymentChannel {
    /// Create a new payment channel
    pub fn new(
        participants: Vec<Participant>,
        initial_balances: HashMap<[u8; 32], u64>,
        config: ChannelConfig,
        current_block: u64,
    ) -> ChannelResult<Self> {
        if participants.is_empty() {
            return Err(ChannelError::InvalidParticipants);
        }

        if initial_balances.is_empty() {
            return Err(ChannelError::InsufficientBalance);
        }

        // Validate initial amounts
        for balance in initial_balances.values() {
            if *balance < config.min_open_amount {
                return Err(ChannelError::AmountTooSmall);
            }
        }

        let channel_id = Self::generate_channel_id(&participants, &initial_balances);
        let state_root = Self::compute_state_root(&initial_balances);

        Ok(Self {
            id: channel_id,
            participants,
            state: ChannelState::Open {
                balances: initial_balances,
                state_root,
                opening_block: current_block,
            },
            config,
            state_machine: ChannelStateMachine {
                current_state: ChannelState::Open {
                    balances: initial_balances.clone(),
                    state_root,
                    opening_block: current_block,
                },
                state_history: vec![(current_block, ChannelState::Open {
                    balances: initial_balances,
                    state_root,
                    opening_block: current_block,
                })],
                dispute_window: Duration::from_secs(3600), // 1 hour
            },
            timeout_block: current_block + config.max_duration,
            updates: Vec::new(),
        })
    }

    /// Process a payment through the channel
    pub fn process_payment(
        &mut self,
        from: [u8; 32],
        to: [u8; 32],
        amount: u64,
        signature: [u8; 64],
        current_block: u64,
    ) -> ChannelResult<ChannelUpdate> {
        self.validate_payment(&from, &to, amount)?;

        let new_update = ChannelUpdate {
            sequence: self.updates.len() as u64 + 1,
            from,
            to,
            amount,
            signature,
            timestamp: Instant::now(),
            // These will be calculated after getting current balances
            new_balance_from: 0,
            new_balance_to: 0,
        };

        // Apply update to current state
        let new_state = self.apply_update(new_update.clone(), current_block)?;

        // Store update
        self.updates.push(new_update.clone());
        self.state_machine.current_state = new_state;
        self.state_machine.state_history.push((current_block, new_state.clone()));

        Ok(new_update)
    }

    /// Close the channel
    pub fn close(&mut self, initiator: Participant, current_block: u64) -> ChannelResult<()> {
        // Check if channel can be closed
        if matches!(self.state, ChannelState::Finalized { .. }) {
            return Err(ChannelError::ChannelClosed);
        }

        // Set closing state
        self.state = ChannelState::Close {
            initiator: initiator.clone(),
            closing_block: current_block,
        };

        self.state_machine.current_state = self.state.clone();
        self.state_machine.state_history.push((current_block, self.state.clone()));

        Ok(())
    }

    /// Settle the channel on-chain
    pub fn settle(&mut self, settlement_tx: Transaction, current_block: u64) -> ChannelResult<()> {
        // Final balances from last state
        let final_balances = match &self.state_machine.current_state {
            ChannelState::Update { last_state_root, .. } => {
                // Recalculate from final update
                self.recalculate_balances(last_state_root)?
            }
            ChannelState::Open { balances, .. } => balances.clone(),
            _ => return Err(ChannelError::InvalidStateTransition),
        };

        self.state = ChannelState::Settle {
            final_balances: final_balances.clone(),
            settlement_tx: Some(settlement_tx),
        };

        self.state_machine.current_state = self.state.clone();
        self.state_machine.state_history.push((current_block, self.state.clone()));

        Ok(())
    }

    /// Finalize the channel
    pub fn finalize(&mut self, current_block: u64) -> ChannelResult<()> {
        let final_balances = match &self.state {
            ChannelState::Settle { final_balances, .. } => final_balances.clone(),
            _ => return Err(ChannelError::InvalidStateTransition),
        };

        self.state = ChannelState::Finalized {
            final_balances,
        };

        self.state_machine.current_state = self.state.clone();
        self.state_machine.state_history.push((current_block, self.state.clone()));

        Ok(())
    }

    /// Apply an update to the channel state
    fn apply_update(&self, update: ChannelUpdate, current_block: u64) -> ChannelResult<ChannelState> {
        // Get current balances
        let current_balances = match &self.state_machine.current_state {
            ChannelState::Open { balances, state_root, .. } => {
                (balances.clone(), *state_root)
            }
            ChannelState::Update { last_state_root, .. } => {
                self.recalculate_balances(last_state_root)?
            }
            _ => return Err(ChannelError::InvalidStateTransition),
        };

        let (mut balances, _) = current_balances;

        // Verify sufficient balance
        let from_balance = balances.get(&update.from).cloned().unwrap_or(0);
        if from_balance < update.amount {
            return Err(ChannelError::InsufficientBalance);
        }

        // Apply payment
        balances.insert(update.from, from_balance - update.amount);
        let to_balance = balances.get(&update.to).cloned().unwrap_or(0);
        balances.insert(update.to, to_balance + update.amount);

        // Create new state
        let state_root = Self::compute_state_root(&balances);

        Ok(ChannelState::Update {
            pending_updates: vec![update],
            last_state_root: state_root,
        })
    }

    /// Validate a payment
    fn validate_payment(&self, from: &[u8; 32], to: &[u8; 32], amount: u64) -> ChannelResult<()> {
        // Check if participants are valid
        if !self.participants.iter().any(|p| p.address == *from) {
            return Err(ChannelError::InvalidParticipants);
        }

        if !self.participants.iter().any(|p| p.address == *to) {
            return Err(ChannelError::InvalidParticipants);
        }

        // Check amount
        if amount < self.config.min_open_amount {
            return Err(ChannelError::AmountTooSmall);
        }

        // Check channel state
        match self.state {
            ChannelState::Open { .. } => Ok(()),
            ChannelState::Update { .. } => Ok(()),
            _ => Err(ChannelError::InvalidStateTransition),
        }
    }

    /// Recalculate balances from state root
    fn recalculate_balances(&self, _state_root: &[u8; 32]) -> ChannelResult<HashMap<[u8; 32], u64>> {
        // In a real implementation, this would:
        // 1. Fetch all pending updates
        // 2. Apply them in sequence
        // 3. Return the final balances
        // For now, return empty map
        Ok(HashMap::new())
    }

    /// Generate channel ID
    fn generate_channel_id(participants: &[Participant], balances: &HashMap<[u8; 32], u64>) -> ChannelId {
        use sha2::{Digest, Sha256};

        let mut hasher = Sha256::new();

        // Include participants
        for participant in participants {
            hasher.update(participant.address);
            hasher.update(participant.public_key);
        }

        // Include balances
        let mut sorted_addresses: Vec<&[u8; 32]> = balances.keys().collect();
        sorted_addresses.sort();

        for address in sorted_addresses {
            hasher.update(address);
            let amount = balances[address];
            hasher.update(&amount.to_le_bytes());
        }

        let result = hasher.finalize();
        let mut id = [0u8; 32];
        id.copy_from_slice(&result);
        id
    }

    /// Compute state root from balances
    fn compute_state_root(balances: &HashMap<[u8; 32], u64>) -> [u8; 32] {
        use sha2::{Digest, Sha256};

        let mut hasher = Sha256::new();

        let mut sorted: Vec<_> = balances.iter().collect();
        sorted.sort_by(|a, b| a.0.cmp(b.0));

        for (address, balance) in sorted {
            hasher.update(address);
            hasher.update(&balance.to_le_bytes());
        }

        let result = hasher.finalize();
        let mut root = [0u8; 32];
        root.copy_from_slice(&result);
        root
    }

    /// Get current channel state
    pub fn get_current_state(&self) -> &ChannelState {
        &self.state_machine.current_state
    }

    /// Get channel status
    pub fn get_status(&self) -> ChannelStatus {
        match &self.state {
            ChannelState::Open { .. } => ChannelStatus::Open,
            ChannelState::Update { .. } => ChannelStatus::Active,
            ChannelState::Close { .. } => ChannelStatus::Closing,
            ChannelState::Settle { .. } => ChannelStatus::Settling,
            ChannelState::Finalized { .. } => ChannelStatus::Finalized,
        }
    }

    /// Check if channel is ready for settlement
    pub fn is_ready_for_settlement(&self) -> bool {
        matches!(&self.state, ChannelState::Update { .. } | ChannelState::Close { .. })
    }

    /// Check if channel has timed out
    pub fn is_timed_out(&self, current_block: u64) -> bool {
        current_block >= self.timeout_block
    }

    /// Get total amount in channel
    pub fn get_total_amount(&self) -> u64 {
        match &self.state {
            ChannelState::Open { balances, .. } => {
                balances.values().sum()
            }
            ChannelState::Update { .. } => {
                // Recalculate from pending updates
                0 // Would calculate from history
            }
            _ => 0,
        }
    }
}

/// Channel status
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChannelStatus {
    Open,
    Active,
    Closing,
    Settling,
    Finalized,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_channel_creation() {
        let participants = vec![
            Participant::new([1u8; 32], [2u8; 32]).as_initiator(),
            Participant::new([3u8; 32], [4u8; 32]),
        ];

        let mut balances = HashMap::new();
        balances.insert([1u8; 32], 1000000);
        balances.insert([3u8; 32], 1000000);

        let config = ChannelConfig::default();
        let channel = PaymentChannel::new(participants, balances, config, 1000).unwrap();

        assert_eq!(channel.participants.len(), 2);
        assert_eq!(channel.get_status(), ChannelStatus::Open);
        assert_eq!(channel.get_total_amount(), 2000000);
    }

    #[test]
    fn test_payment_processing() {
        let participants = vec![
            Participant::new([1u8; 32], [2u8; 32]).as_initiator(),
            Participant::new([3u8; 32], [4u8; 32]),
        ];

        let mut balances = HashMap::new();
        balances.insert([1u8; 32], 1000000);
        balances.insert([3u8; 32], 1000000);

        let config = ChannelConfig::default();
        let mut channel = PaymentChannel::new(participants, balances, config, 1000).unwrap();

        // Process payment
        let update = channel.process_payment(
            [1u8; 32],
            [3u8; 32],
            500000,
            [5u8; 64],
            1001,
        ).unwrap();

        assert_eq!(update.sequence, 1);
        assert_eq!(update.amount, 500000);
        assert_eq!(channel.updates.len(), 1);
    }

    #[test]
    fn test_insufficient_funds() {
        let participants = vec![
            Participant::new([1u8; 32], [2u8; 32]).as_initiator(),
            Participant::new([3u8; 32], [4u8; 32]),
        ];

        let mut balances = HashMap::new();
        balances.insert([1u8; 32], 1000000);
        balances.insert([3u8; 32], 1000000);

        let config = ChannelConfig::default();
        let mut channel = PaymentChannel::new(participants, balances, config, 1000).unwrap();

        // Try to send more than balance
        let result = channel.process_payment(
            [1u8; 32],
            [3u8; 32],
            2000000,
            [5u8; 64],
            1001,
        );

        assert!(matches!(result, Err(ChannelError::InsufficientBalance)));
    }

    #[test]
    fn test_channel_closing() {
        let participants = vec![
            Participant::new([1u8; 32], [2u8; 32]).as_initiator(),
        ];

        let mut balances = HashMap::new();
        balances.insert([1u8; 32], 1000000);

        let config = ChannelConfig::default();
        let mut channel = PaymentChannel::new(participants, balances, config, 1000).unwrap();

        let initiator = Participant::new([1u8; 32], [2u8; 32]);
        channel.close(initiator, 1001).unwrap();
        assert_eq!(channel.get_status(), ChannelStatus::Closing);
    }
}