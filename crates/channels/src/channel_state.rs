//! Channel State Management

use crate::{ChannelError, ChannelResult};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// State of a channel
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ChannelState {
    /// Channel is open and ready for transactions
    Open {
        balances: HashMap<[u8; 32], u64>,
        state_root: [u8; 32],
        opening_block: u64,
        participants: Vec<Participant>,
    },
    /// Channel has updates pending settlement
    Update {
        pending_updates: Vec<ChannelUpdate>,
        last_state_root: [u8; 32],
        last_sequence: u64,
    },
    /// Channel is being closed
    Close {
        initiator: [u8; 32],
        closing_block: u64,
        reason: CloseReason,
    },
    /// Channel is being settled
    Settle {
        final_balances: HashMap<[u8; 32], u64>,
        settlement_tx_hash: [u8; 32],
        settlement_block: u64,
    },
    /// Channel is finalized
    Finalized {
        final_balances: HashMap<[u8; 32], u64>,
        final_block: u64,
    },
    /// Channel is cancelled due to dispute
    Cancelled {
        reason: DisputeReason,
        cancelled_block: u64,
        final_balances: HashMap<[u8; 32], u64>,
    },
}

/// Reason for closing a channel
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CloseReason {
    /// Mutual agreement to close
    Mutual,
    /// Timeout reached
    Timeout,
    /// Dispute resolution
    Dispute,
    /// Force close by participant
    ForceClose,
}

/// Reason for cancelling a channel
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum DisputeReason {
    /// Fraudulent transaction
    Fraud,
    /// Invalid state transition
    InvalidState,
    /// Counterparty unresponsive
    Unresponsive,
    /// Breach of contract
    ContractBreach,
}

/// Channel update details
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ChannelUpdate {
    pub sequence: u64,
    pub from: [u8; 32],
    pub to: [u8; 32],
    pub amount: u64,
    pub new_balance_from: u64,
    pub new_balance_to: u64,
    pub signature: [u8; 64],
    pub timestamp: u64,
    pub state_root: [u8; 32],
}

/// Channel participant
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Participant {
    pub address: [u8; 32],
    pub public_key: [u8; 32],
    pub is_initiator: bool,
}

/// State transition history
#[derive(Debug, Clone)]
pub struct StateTransition {
    pub from_state: ChannelState,
    pub to_state: ChannelState,
    pub block_height: u64,
    pub tx_hash: Option<[u8; 32]>,
    pub trigger: TransitionTrigger,
}

/// What triggered a state transition
#[derive(Debug, Clone)]
pub enum TransitionTrigger {
    ChannelOpen,
    PaymentUpdate,
    CloseRequest,
    SettlementRequest,
    Dispute,
    Timeout,
    ForceClose,
}

/// State machine for channels
pub struct ChannelStateMachine {
    pub current_state: ChannelState,
    pub state_history: Vec<StateTransition>,
    pub current_block: u64,
}

impl ChannelStateMachine {
    /// Create a new state machine
    pub fn new(initial_state: ChannelState, initial_block: u64) -> Self {
        Self {
            current_state: initial_state,
            state_history: Vec::new(),
            current_block: initial_block,
        }
    }

    /// Transition to a new state
    pub fn transition(
        &mut self,
        new_state: ChannelState,
        trigger: TransitionTrigger,
        tx_hash: Option<[u8; 32]>,
    ) -> ChannelResult<()> {
        // Validate state transition
        self.validate_transition(&self.current_state, &new_state)?;

        // Create transition record
        let transition = StateTransition {
            from_state: self.current_state.clone(),
            to_state: new_state.clone(),
            block_height: self.current_block,
            tx_hash,
            trigger,
        };

        // Update state
        self.current_state = new_state;
        self.state_history.push(transition);

        Ok(())
    }

    /// Validate state transition
    fn validate_transition(&self, from: &ChannelState, to: &ChannelState) -> ChannelResult<()> {
        match (from, to) {
            // Valid transitions
            (ChannelState::Open { .. }, ChannelState::Update { .. }) => Ok(()),
            (ChannelState::Open { .. }, ChannelState::Close { .. }) => Ok(()),
            (ChannelState::Update { .. }, ChannelState::Update { .. }) => Ok(()),
            (ChannelState::Update { .. }, ChannelState::Close { .. }) => Ok(()),
            (ChannelState::Update { .. }, ChannelState::Settle { .. }) => Ok(()),
            (ChannelState::Close { .. }, ChannelState::Settle { .. }) => Ok(()),
            (ChannelState::Settle { .. }, ChannelState::Finalized { .. }) => Ok(()),

            // Dispute transitions
            (_, ChannelState::Cancelled { .. }) => Ok(()),

            // Invalid transitions
            _ => Err(ChannelError::InvalidStateTransition),
        }
    }

    /// Get current state
    pub fn current_state(&self) -> &ChannelState {
        &self.current_state
    }

    /// Update current block height
    pub fn set_block_height(&mut self, height: u64) {
        self.current_block = height;
    }

    /// Get state history
    pub fn history(&self) -> &[StateTransition] {
        &self.state_history
    }

    /// Check if channel is open for payments
    pub fn is_open_for_payments(&self) -> bool {
        matches!(&self.current_state, ChannelState::Open { .. } | ChannelState::Update { .. })
    }

    /// Check if channel can be closed
    pub fn can_be_closed(&self) -> bool {
        matches!(&self.current_state, ChannelState::Open { .. } | ChannelState::Update { .. } | ChannelState::Close { .. })
    }

    /// Check if channel can be settled
    pub fn can_be_settled(&self) -> bool {
        matches!(&self.current_state, ChannelState::Update { .. } | ChannelState::Close { .. })
    }

    /// Get total amount in channel
    pub fn get_total_amount(&self) -> u64 {
        match &self.current_state {
            ChannelState::Open { balances, .. } => balances.values().sum(),
            ChannelState::Update { pending_updates, .. } => {
                // Calculate from last known state
                0 // Would calculate based on updates
            }
            ChannelState::Settle { final_balances, .. } => final_balances.values().sum(),
            ChannelState::Finalized { final_balances, .. } => final_balances.values().sum(),
            ChannelState::Cancelled { final_balances, .. } => final_balances.values().sum(),
            ChannelState::Close { .. } => 0, // Would get from current state
        }
    }

    /// Get participant balance
    pub fn get_balance(&self, participant: &[u8; 32]) -> Option<u64> {
        match &self.current_state {
            ChannelState::Open { balances, .. } => balances.get(participant).cloned(),
            ChannelState::Update { .. } => {
                // Would calculate from current state
                None
            }
            ChannelState::Settle { final_balances, .. } => final_balances.get(participant).cloned(),
            ChannelState::Finalized { final_balances, .. } => final_balances.get(participant).cloned(),
            ChannelState::Cancelled { final_balances, .. } => final_balances.get(participant).cloned(),
            ChannelState::Close { .. } => None,
        }
    }

    /// Get channel participants
    pub fn get_participants(&self) -> Vec<&[u8; 32]> {
        match &self.current_state {
            ChannelState::Open { participants, .. } => {
                participants.iter().map(|p| &p.address).collect()
            }
            ChannelState::Update { .. } => {
                // Would infer from updates
                vec![]
            }
            _ => vec![],
        }
    }
}

/// State machine persistence
pub struct StateMachineStore {
    store: Arc<RwLock<HashMap<[u8; 32], ChannelStateMachine>>>,
}

impl StateMachineStore {
    pub fn new() -> Self {
        Self {
            store: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Store a state machine
    pub async fn store(&self, channel_id: [u8; 32], state_machine: ChannelStateMachine) {
        let mut store = self.store.write().await;
        store.insert(channel_id, state_machine);
    }

    /// Get a state machine
    pub async fn get(&self, channel_id: &[u8; 32]) -> Option<ChannelStateMachine> {
        let store = self.store.read().await;
        store.get(channel_id).cloned()
    }

    /// Delete a state machine
    pub async fn delete(&self, channel_id: &[u8; 32]) {
        let mut store = self.store.write().await;
        store.remove(channel_id);
    }

    /// Get all stored state machines
    pub async fn get_all(&self) -> HashMap<[u8; 32], ChannelStateMachine> {
        let store = self.store.read().await;
        store.clone()
    }

    /// Get count of stored state machines
    pub async fn count(&self) -> usize {
        let store = self.store.read().await;
        store.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_transitions() {
        let initial = ChannelState::Open {
            balances: HashMap::new(),
            state_root: [0u8; 32],
            opening_block: 1000,
            participants: vec![],
        };

        let mut machine = ChannelStateMachine::new(initial, 1000);

        // Valid transition: Open -> Update
        let new_state = ChannelState::Update {
            pending_updates: vec![],
            last_state_root: [1u8; 32],
            last_sequence: 0,
        };

        assert!(machine.transition(new_state, TransitionTrigger::PaymentUpdate, None).is_ok());

        // Invalid transition: Update -> Open (should fail)
        let invalid_state = ChannelState::Open {
            balances: HashMap::new(),
            state_root: [2u8; 32],
            opening_block: 1000,
            participants: vec![],
        };

        assert!(machine.transition(invalid_state, TransitionTrigger::PaymentUpdate, None).is_err());
    }

    #[test]
    fn test_channel_status_checks() {
        let initial = ChannelState::Open {
            balances: HashMap::new(),
            state_root: [0u8; 32],
            opening_block: 1000,
            participants: vec![],
        };

        let mut machine = ChannelStateMachine::new(initial, 1000);

        assert!(machine.is_open_for_payments());
        assert!(machine.can_be_closed());
        assert!(!machine.can_be_settled());

        // Transition to update state
        let update_state = ChannelState::Update {
            pending_updates: vec![],
            last_state_root: [1u8; 32],
            last_sequence: 0,
        };

        machine.transition(update_state, TransitionTrigger::PaymentUpdate, None).unwrap();

        assert!(machine.is_open_for_payments());
        assert!(machine.can_be_closed());
        assert!(machine.can_be_settled());
    }

    #[test]
    fn test_balance_calculation() {
        let mut balances = HashMap::new();
        balances.insert([1u8; 32], 1000);
        balances.insert([2u8; 32], 2000);

        let initial = ChannelState::Open {
            balances: balances.clone(),
            state_root: [0u8; 32],
            opening_block: 1000,
            participants: vec![],
        };

        let machine = ChannelStateMachine::new(initial, 1000);

        assert_eq!(machine.get_total_amount(), 3000);
        assert_eq!(machine.get_balance(&[1u8; 32]), Some(1000));
        assert_eq!(machine.get_balance(&[2u8; 32]), Some(2000));
        assert_eq!(machine.get_balance(&[3u8; 32]), None);
    }

    #[tokio::test]
    async fn test_state_machine_store() {
        let store = StateMachineStore::new();

        let initial = ChannelState::Open {
            balances: HashMap::new(),
            state_root: [0u8; 32],
            opening_block: 1000,
            participants: vec![],
        };

        let mut machine = ChannelStateMachine::new(initial, 1000);

        let channel_id = [1u8; 32];
        store.store(channel_id, machine).await;

        let retrieved = store.get(&channel_id).await.unwrap();
        assert_eq!(retrieved.current_block(), 1000);

        store.delete(&channel_id).await;
        assert!(store.get(&channel_id).await.is_none());
    }
}