use commonware_storage::{
    journal::{Journal, Config as JournalConfig},
    archive::{Archive, Config as ArchiveConfig, Identifier, translator::FourCap},
};
use commonware_runtime::tokio::{Runtime, Blob};
use commonware_utils::hash;
use prometheus_client::registry::Registry;
use std::sync::{Arc, Mutex};
use bytes::Bytes;
use serde::{Serialize, Deserialize};
use thiserror::Error;

/// Represents a block in the blockchain. Each block contains a number indicating its height,
/// the hash of its parent block, its own hash (calculated from its contents), and a timestamp
/// of when it was created.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Block {
    /// Block height in the chain
    pub number: u64,
    /// Hash of the parent block
    pub parent_hash: [u8; 32],
    /// Hash of this block
    pub hash: [u8; 32],
    /// Unix timestamp when block was created
    pub timestamp: u64,
}

impl Block {
    /// Creates a new block with the given parameters. The block's own hash is automatically
    /// calculated from its contents to ensure integrity.
    pub fn new(number: u64, parent_hash: [u8; 32], timestamp: u64) -> Self {
        let mut block = Self {
            number,
            parent_hash,
            hash: [0; 32],
            timestamp,
        };
        block.hash = block.calculate_hash();
        block
    }

    /// Calculates the block's hash using its number, parent hash, and timestamp.
    /// This hash uniquely identifies the block and protects its integrity.
    pub fn calculate_hash(&self) -> [u8; 32] {
        // Create a buffer for serializing block data
        let mut buffer = Vec::new();
        
        // Add all fields that contribute to block identity
        buffer.extend_from_slice(&self.number.to_le_bytes());
        buffer.extend_from_slice(&self.parent_hash);
        buffer.extend_from_slice(&self.timestamp.to_le_bytes());
        
        // Hash using SHA-256
        let hash_result = hash(&buffer);
        
        // Convert to fixed-size array
        let mut fixed_hash = [0u8; 32];
        fixed_hash.copy_from_slice(&hash_result);
        fixed_hash
    }

    /// Validates the block's relationship with its parent block and its internal consistency.
    pub fn validate(&self, parent: Option<&Block>) -> Result<(), BlockError> {
        // Verify that the block's hash matches its contents
        if self.hash != self.calculate_hash() {
            return Err(BlockError::InvalidHash);
        }

        // If we have a parent block, validate the relationship
        if let Some(parent) = parent {
            // Verify block builds on parent
            if self.parent_hash != parent.hash {
                return Err(BlockError::InvalidParentHash);
            }

            // Verify block number follows parent
            if self.number != parent.number + 1 {
                return Err(BlockError::InvalidBlockNumber);
            }

            // Verify timestamp is after parent
            if self.timestamp <= parent.timestamp {
                return Err(BlockError::InvalidTimestamp);
            }
        } else if self.number != 0 {
            // If no parent provided, only genesis block (number 0) is valid
            return Err(BlockError::MissingParent);
        }

        Ok(())
    }
}

/// Manages persistent storage of blockchain data using Archive for efficient
/// key-value storage with dual indexing capabilities.
pub struct BlockStorage {
    archive: Archive<FourCap, Blob, Runtime>,
}

impl BlockStorage {
    /// Creates a new BlockStorage instance with the given runtime and metrics registry.
    pub async fn new(runtime: Runtime, registry: Arc<Mutex<Registry>>) -> Result<Self, BlockError> {
        // Initialize the journal for persistent storage
        let journal = Journal::init(
            runtime.clone(),
            JournalConfig {
                registry: registry.clone(),
                partition: "blocks".into(),
            },
        ).await.map_err(BlockError::Archive)?;

        // Configure and initialize the archive
        let archive = Archive::init(
            journal,
            ArchiveConfig {
                registry,
                key_len: 32,  // SHA-256 hashes are 32 bytes
                translator: FourCap,  // Use first 4 bytes of hash for indexing
                section_mask: 0xffff_ffff_ffff_0000u64,  // 65536 blocks per section
                pending_writes: 10,
                replay_concurrency: 4,
                compression: None,
            },
        ).await.map_err(BlockError::Archive)?;

        Ok(Self { archive })
    }

    /// Stores a block in the archive, indexed by both its number and hash.
    pub async fn put_block(&mut self, block: Block) -> Result<(), BlockError> {
        let data = bincode::serialize(&block).map_err(BlockError::Serialization)?;
        self.archive
            .put(block.number, &block.hash, Bytes::from(data))
            .await
            .map_err(BlockError::Archive)?;
        Ok(())
    }

    /// Retrieves a block by its block number.
    pub async fn get_block_by_number(&self, number: u64) -> Result<Option<Block>, BlockError> {
        let data = self.archive
            .get(Identifier::Index(number))
            .await
            .map_err(BlockError::Archive)?;
        
        if let Some(bytes) = data {
            let block = bincode::deserialize(&bytes).map_err(BlockError::Serialization)?;
            Ok(Some(block))
        } else {
            Ok(None)
        }
    }

    /// Retrieves a block by its hash.
    pub async fn get_block_by_hash(&self, hash: &[u8; 32]) -> Result<Option<Block>, BlockError> {
        let data = self.archive
            .get(Identifier::Key(hash))
            .await
            .map_err(BlockError::Archive)?;

        if let Some(bytes) = data {
            let block = bincode::deserialize(&bytes).map_err(BlockError::Serialization)?;
            Ok(Some(block))
        } else {
            Ok(None)
        }
    }

    /// Checks if a block with the given number exists.
    pub async fn has_block(&self, number: u64) -> Result<bool, BlockError> {
        self.archive
            .has(Identifier::Index(number))
            .await
            .map_err(BlockError::Archive)
    }

    /// Finds gaps in the block sequence, returning the range of the next gap if any exists.
    pub async fn next_gap(&self, number: u64) -> (Option<u64>, Option<u64>) {
        self.archive.next_gap(number)
    }

    /// Removes blocks older than the given number to conserve storage space.
    pub async fn prune(&mut self, min_block: u64) -> Result<(), BlockError> {
        self.archive
            .prune(min_block)
            .await
            .map_err(BlockError::Archive)
    }

    /// Properly closes the storage, ensuring all data is persisted.
    pub async fn close(self) -> Result<(), BlockError> {
        self.archive
            .close()
            .await
            .map_err(BlockError::Archive)
    }
}

/// Errors that can occur during block operations.
#[derive(Debug, Error)]
pub enum BlockError {
    #[error("Archive error: {0}")]
    Archive(#[from] commonware_storage::archive::Error),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] bincode::Error),
    
    #[error("Invalid block hash")]
    InvalidHash,
    
    #[error("Invalid parent block hash")]
    InvalidParentHash,
    
    #[error("Invalid block number")]
    InvalidBlockNumber,
    
    #[error("Invalid block timestamp")]
    InvalidTimestamp,
    
    #[error("Missing parent block")]
    MissingParent,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn get_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }

    #[tokio::test]
    async fn test_block_storage() {
        let registry = Arc::new(Mutex::new(Registry::default()));
        let mut storage = BlockStorage::new(runtime.clone(), registry)
            .await
            .unwrap();

        // Create and store a test block
        let block = Block::new(1, [1; 32], get_timestamp());
        storage.put_block(block.clone()).await.unwrap();

        // Retrieve by number
        let retrieved = storage.get_block_by_number(1).await.unwrap().unwrap();
        assert_eq!(retrieved.hash, block.hash);

        // Retrieve by hash
        let retrieved = storage.get_block_by_hash(&block.hash).await.unwrap().unwrap();
        assert_eq!(retrieved.number, block.number);
    }

    #[test]
    fn test_block_validation() {
        let timestamp = get_timestamp();
        
        // Create parent block
        let parent = Block::new(0, [0; 32], timestamp);
        
        // Create valid child block
        let valid_child = Block::new(1, parent.hash, timestamp + 1);
        assert!(valid_child.validate(Some(&parent)).is_ok());

        // Test invalid block number
        let invalid_number = Block::new(2, parent.hash, timestamp + 1);
        assert!(matches!(
            invalid_number.validate(Some(&parent)),
            Err(BlockError::InvalidBlockNumber)
        ));

        // Test invalid parent hash
        let invalid_parent = Block::new(1, [2; 32], timestamp + 1);
        assert!(matches!(
            invalid_parent.validate(Some(&parent)),
            Err(BlockError::InvalidParentHash)
        ));
    }
}