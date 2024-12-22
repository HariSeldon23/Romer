use commonware_consensus::{Automaton, Committer, Digest};
use std::sync::Arc;
use tokio::sync::{Mutex, oneshot};
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

use crate::{
    storage::{Block, BlockStorage, BlockError},
    consensus::relay::{ConsensusRelay, ConsensusMessage},
};

/// Proposer handles the creation and validation of blocks, working with the relay
/// system for network communication. It maintains focus on block lifecycle while
/// delegating all network operations to the relay.
pub struct Proposer {
    /// Storage interface for blocks
    storage: Arc<Mutex<BlockStorage>>,
    /// Relay for network communication
    relay: Arc<Mutex<ConsensusRelay>>,
    /// Hash of the most recently created block
    latest_hash: Arc<Mutex<[u8; 32]>>,
}

impl Proposer {
    /// Creates a new Proposer instance with the given storage and relay
    pub fn new(storage: BlockStorage, relay: ConsensusRelay) -> Self {
        Self {
            storage: Arc::new(Mutex::new(storage)),
            relay: Arc::new(Mutex::new(relay)),
            latest_hash: Arc::new(Mutex::new([1; 32])), // Genesis block hash
        }
    }

    /// Creates a new block building on top of the given parent.
    /// This is an internal operation that doesn't involve network communication.
    async fn create_block(&self, parent_hash: [u8; 32]) -> Result<Block, ProposerError> {
        let mut storage = self.storage.lock().await;
        
        // Determine the parent block's number
        let parent_number = if parent_hash == [1; 32] {
            // Special case for genesis block
            0
        } else {
            match storage.get_block_by_hash(&parent_hash).await? {
                Some(parent) => parent.number,
                None => {
                    // If we don't have the parent, request it from the network
                    let mut relay = self.relay.lock().await;
                    relay.request_block(&parent_hash).await?;
                    return Err(ProposerError::MissingParent);
                }
            }
        };

        // Create the new block
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Ok(Block::new(
            parent_number + 1,
            parent_hash,
            timestamp,
        ))
    }

    /// Validates a block matches our consensus rules
    async fn validate_block(&self, block: &Block, expected_parent: [u8; 32]) -> Result<(), ProposerError> {
        // First check the parent hash matches what we expect
        if block.parent_hash != expected_parent {
            return Err(ProposerError::InvalidParentHash);
        }

        let storage = self.storage.lock().await;

        // Get the parent block, requesting it if we don't have it
        let parent = if expected_parent == [1; 32] {
            None // Genesis block has no parent
        } else {
            match storage.get_block_by_hash(&expected_parent).await? {
                Some(parent) => Some(parent),
                None => {
                    // Request the missing parent block
                    let mut relay = self.relay.lock().await;
                    relay.request_block(&expected_parent).await?;
                    return Err(ProposerError::MissingParent);
                }
            }
        };

        // Validate block against its parent
        block.validate(parent.as_ref())?;
        Ok(())
    }
}

impl Automaton for Proposer {
    type Context = (u64, [u8; 32]); // (view number, parent hash)

    async fn genesis(&mut self) -> Digest {
        // Return the genesis block hash
        [1; 32]
    }

    async fn propose(&mut self, context: Self::Context) -> oneshot::Receiver<Digest> {
        let (tx, rx) = oneshot::channel();
        let (_view, parent_hash) = context;
        
        // Clone Arc references for the async block
        let this = self.clone();
        let latest_hash = self.latest_hash.clone();
        
        tokio::spawn(async move {
            match this.create_block(parent_hash).await {
                Ok(block) => {
                    // Store the block locally
                    let mut storage = this.storage.lock().await;
                    if let Ok(()) = storage.put_block(block.clone()).await {
                        // Broadcast the new block through the relay
                        if let Ok(()) = this.relay.lock().await
                            .broadcast_block(block.clone()).await 
                        {
                            *latest_hash.lock().await = block.hash;
                            let _ = tx.send(block.hash);
                        }
                    }
                },
                Err(_) => {
                    let _ = tx.send(parent_hash); // Fall back to parent hash on error
                }
            }
        });

        rx
    }

    async fn verify(&mut self, context: Self::Context, payload: Digest) -> oneshot::Receiver<bool> {
        let (tx, rx) = oneshot::channel();
        let (_view, parent_hash) = context;
        
        let this = self.clone();
        
        tokio::spawn(async move {
            let storage = this.storage.lock().await;
            
            let is_valid = if let Ok(Some(block)) = storage.get_block_by_hash(&payload).await {
                this.validate_block(&block, parent_hash).await.is_ok()
            } else {
                // If we don't have the block, request it and return false for now
                let mut relay = this.relay.lock().await;
                let _ = relay.request_block(&payload).await;
                false
            };
            
            let _ = tx.send(is_valid);
        });

        rx
    }
}

impl Committer for Proposer {
    async fn prepared(&mut self, _proposal: &[u8], _proof: &[u8]) {
        // We don't need special handling for the prepared phase
    }
    
    async fn finalized(&mut self, _proposal: &[u8], _proof: &[u8]) {
        // We don't need special handling for the finalized phase
    }
    
    async fn committed(&mut self, payload: &[u8]) {
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&payload[..32]);
        
        // Update our latest hash and notify the relay of commitment
        *self.latest_hash.lock().await = hash;
        
        if let Ok(Some(block)) = self.storage.lock().await.get_block_by_hash(&hash).await {
            let _ = self.relay.lock().await.send_to(
                crate::consensus::relay::Recipients::All,
                ConsensusMessage::NewBlock(block),
            ).await;
        }
    }
}

impl Clone for Proposer {
    fn clone(&self) -> Self {
        Self {
            storage: self.storage.clone(),
            relay: self.relay.clone(),
            latest_hash: self.latest_hash.clone(),
        }
    }
}

/// Errors that can occur during proposer operations
#[derive(Debug, Error)]
pub enum ProposerError {
    #[error("Storage error: {0}")]
    Storage(#[from] BlockError),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Parent block not found")]
    MissingParent,

    #[error("Invalid parent hash")]
    InvalidParentHash,

    #[error("Invalid block")]
    InvalidBlock,

    #[error("Relay error: {0}")]
    Relay(#[from] crate::consensus::relay::RelayError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use prometheus_client::registry::Registry;
    use commonware_p2p::authenticated::Sender;

    async fn setup_test_environment() -> (Proposer, BlockStorage) {
        let storage = BlockStorage::new(
            runtime.clone(),
            Arc::new(std::sync::Mutex::new(Registry::default())),
        ).await.unwrap();
        
        let network = Sender::default();
        let relay = ConsensusRelay::new(network, storage.clone());
        let proposer = Proposer::new(storage.clone(), relay);
        
        (proposer, storage)
    }

    #[tokio::test]
    async fn test_block_creation() {
        let (proposer, _) = setup_test_environment().await;
        
        let parent_hash = [1; 32]; // Genesis hash
        let block = proposer.create_block(parent_hash).await.unwrap();
        
        assert_eq!(block.number, 1);
        assert_eq!(block.parent_hash, parent_hash);
        assert!(block.timestamp > 0);
    }

    #[tokio::test]
    async fn test_block_validation() {
        let (proposer, _) = setup_test_environment().await;
        
        // Create a valid block
        let parent_hash = [1; 32];
        let valid_block = proposer.create_block(parent_hash).await.unwrap();
        
        // Should validate successfully
        assert!(proposer.validate_block(&valid_block, parent_hash).await.is_ok());
        
        // Create an invalid block (wrong parent)
        let wrong_parent = [2; 32];
        assert!(proposer.validate_block(&valid_block, wrong_parent).await.is_err());
    }

    #[tokio::test]
    async fn test_block_proposal_cycle() {
        let (mut proposer, _) = setup_test_environment().await;
        
        // Test genesis
        let genesis_hash = proposer.genesis().await;
        assert_eq!(genesis_hash, [1; 32]);
        
        // Test propose
        let (view, parent_hash) = (0u64, genesis_hash);
        let propose_rx = proposer.propose((view, parent_hash)).await;
        let proposed_hash = propose_rx.await.unwrap();
        
        // Test verify
        let verify_rx = proposer.verify((view, parent_hash), proposed_hash).await;
        assert!(verify_rx.await.unwrap());
    }
}