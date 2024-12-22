use commonware_consensus::Relay;
use commonware_cryptography::Ed25519;
use commonware_p2p::{authenticated::Sender, Recipients};
use std::sync::Arc;
use tokio::sync::Mutex;
use bytes::Bytes;
use serde::{Serialize, Deserialize};
use thiserror::Error;

use crate::storage::{Block, BlockStorage};

/// Types of messages that can be sent between nodes
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ConsensusMessage {
    // Block-related messages
    BlockRequest(Vec<u8>),
    BlockResponse(Block),
    NewBlock(Block),

    // Leader election messages
    ViewChange(u64),                    // Notify peers of view change
    LeaderProposal(u64, Vec<u8>),      // (view, leader_pubkey)
    LeaderVote(u64, Vec<u8>),          // (view, vote_for_pubkey)
    LeaderAnnouncement(u64, Vec<u8>),  // (view, chosen_leader)

    // Region/validator messages
    ValidatorAnnounce {
        public_key: Vec<u8>,
        region: String,
    },
    ValidatorLeave {
        public_key: Vec<u8>,
        region: String,
    },
}

/// ConsensusRelay handles all network communication between nodes
pub struct ConsensusRelay {
    /// Network interface for sending messages
    network: Arc<Mutex<Sender>>,
    /// Storage interface for blocks
    storage: Arc<Mutex<BlockStorage>>,
}

impl ConsensusRelay {
    /// Creates a new ConsensusRelay instance
    pub fn new(network: Sender, storage: BlockStorage) -> Self {
        Self {
            network: Arc::new(Mutex::new(network)),
            storage: Arc::new(Mutex::new(storage)),
        }
    }

    /// Sends a message to a specific recipient
    pub async fn send_to(&self, recipient: Recipients, message: ConsensusMessage) -> Result<(), RelayError> {
        let encoded = bincode::serialize(&message)
            .map_err(|_| RelayError::SerializationError)?;

        let mut network = self.network.lock().await;
        network.send(recipient, Bytes::from(encoded), false)
            .await
            .map_err(|_| RelayError::NetworkError)?;

        Ok(())
    }

    /// Handles an incoming consensus message
    pub async fn handle_message(
        &self,
        message: ConsensusMessage,
        sender: Vec<u8>,
        beacon: &mut crate::beacon::BeaconConsensus,
    ) -> Result<(), RelayError> {
        match message {
            // Block-related message handling
            ConsensusMessage::BlockRequest(hash) => {
                let storage = self.storage.lock().await;
                if let Ok(Some(block)) = storage.get_block_by_hash(&hash[..32].try_into().unwrap()).await {
                    self.send_to(
                        Recipients::Single(Bytes::from(sender)),
                        ConsensusMessage::BlockResponse(block),
                    ).await?;
                }
            },
            ConsensusMessage::BlockResponse(block) => {
                let mut storage = self.storage.lock().await;
                storage.put_block(block).await
                    .map_err(|_| RelayError::StorageError)?;
            },
            ConsensusMessage::NewBlock(block) => {
                let mut storage = self.storage.lock().await;
                storage.put_block(block).await
                    .map_err(|_| RelayError::StorageError)?;
            },

            // Leader election message handling
            ConsensusMessage::ViewChange(view) => {
                // Handle view change notification
                self.broadcast_leader_proposal(view).await?;
            },
            ConsensusMessage::LeaderProposal(view, proposed_leader) => {
                // Process leader proposal and vote if valid
                if self.verify_leader_proposal(view, &proposed_leader).await? {
                    self.send_leader_vote(view, proposed_leader).await?;
                }
            },
            ConsensusMessage::LeaderVote(view, vote) => {
                // Collect votes and potentially trigger leader announcement
                self.process_leader_vote(view, vote).await?;
            },
            ConsensusMessage::LeaderAnnouncement(view, leader) => {
                // Update local leader state
                self.handle_leader_announcement(view, leader).await?;
            },

            // Region/validator message handling
            ConsensusMessage::ValidatorAnnounce { public_key, region } => {
                // Register new validator with beacon
                let validator = Ed25519::from_public_key(&public_key)
                    .map_err(|_| RelayError::InvalidMessage)?;
                beacon.register_validator(region, validator);
            },
            ConsensusMessage::ValidatorLeave { public_key, region } => {
                // Remove validator from beacon
                beacon.remove_validator(&region, &public_key);
            },
        }
        Ok(())
    }

    /// Broadcasts a new view change to all peers
    pub async fn broadcast_view_change(&self, view: u64) -> Result<(), RelayError> {
        self.send_to(
            Recipients::All,
            ConsensusMessage::ViewChange(view),
        ).await
    }

    /// Broadcasts a leader proposal for a view
    pub async fn broadcast_leader_proposal(&self, view: u64) -> Result<(), RelayError> {
        // Logic to select and propose a leader based on region
        // This would typically come from the beacon
        Ok(())
    }

    /// Verifies a leader proposal is valid
    async fn verify_leader_proposal(&self, view: u64, proposed_leader: &[u8]) -> Result<bool, RelayError> {
        // Verify the proposed leader is valid for this view
        // This would typically check against the beacon's rules
        Ok(true)
    }

    /// Sends a vote for a proposed leader
    async fn send_leader_vote(&self, view: u64, leader: Vec<u8>) -> Result<(), RelayError> {
        self.send_to(
            Recipients::All,
            ConsensusMessage::LeaderVote(view, leader),
        ).await
    }

    /// Processes a received leader vote
    async fn process_leader_vote(&self, view: u64, vote: Vec<u8>) -> Result<(), RelayError> {
        // Process vote and if we have enough votes, announce the leader
        Ok(())
    }

    /// Handles a leader announcement
    async fn handle_leader_announcement(&self, view: u64, leader: Vec<u8>) -> Result<(), RelayError> {
        // Update local state with the new leader
        Ok(())
    }

    /// Announces a validator's presence in a region
    pub async fn announce_validator(&self, public_key: Vec<u8>, region: String) -> Result<(), RelayError> {
        self.send_to(
            Recipients::All,
            ConsensusMessage::ValidatorAnnounce { public_key, region },
        ).await
    }

    /// Announces a validator's departure from a region
    pub async fn leave_region(&self, public_key: Vec<u8>, region: String) -> Result<(), RelayError> {
        self.send_to(
            Recipients::All,
            ConsensusMessage::ValidatorLeave { public_key, region },
        ).await
    }
}

impl Relay for ConsensusRelay {
    async fn broadcast(&mut self, payload: &[u8]) {
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&payload[..32]);

        // Attempt to get and broadcast the block
        if let Ok(Some(block)) = self.storage.lock().await.get_block_by_hash(&hash).await {
            let _ = self.broadcast_block(block).await;
        }
    }
}

/// Errors that can occur during relay operations
#[derive(Debug, Error)]
pub enum RelayError {
    #[error("Network error")]
    NetworkError,

    #[error("Storage error")]
    StorageError,

    #[error("Serialization error")]
    SerializationError,

    #[error("Invalid message format")]
    InvalidMessage,

    #[error("Invalid view change")]
    InvalidViewChange,

    #[error("Leader election error")]
    LeaderElectionError,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use prometheus_client::registry::Registry;

    async fn setup_test_environment() -> (ConsensusRelay, BlockStorage) {
        let storage = BlockStorage::new(
            runtime.clone(),
            Arc::new(std::sync::Mutex::new(Registry::default())),
        ).await.unwrap();
        let network = Sender::default();
        
        let relay = ConsensusRelay::new(network, storage.clone());
        (relay, storage)
    }

    #[tokio::test]
    async fn test_view_change_cycle() {
        let (relay, _) = setup_test_environment().await;
        
        // Test view change broadcast
        relay.broadcast_view_change(1).await.unwrap();
        
        // Test leader proposal
        relay.broadcast_leader_proposal(1).await.unwrap();
        
        // Test leader voting
        let test_leader = vec![1; 32];
        relay.send_leader_vote(1, test_leader.clone()).await.unwrap();
    }

    #[tokio::test]
    async fn test_validator_announcements() {
        let (relay, _) = setup_test_environment().await;
        
        let test_key = vec![1; 32];
        let test_region = "Frankfurt".to_string();
        
        // Test validator announce
        relay.announce_validator(test_key.clone(), test_region.clone()).await.unwrap();
        
        // Test validator leave
        relay.leave_region(test_key, test_region).await.unwrap();
    }
}