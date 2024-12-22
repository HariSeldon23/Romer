use commonware_consensus::{
    simplex::{Engine, Config as SimplexConfig},
    Automaton, Committer,
};
use commonware_cryptography::Ed25519;
use commonware_p2p::authenticated::Sender;
use commonware_runtime::tokio::Runtime;
use commonware_utils::hash;
use std::time::Duration;
use std::sync::{Arc, Mutex};
use prometheus_client::registry::Registry;
use governor::Quota;
use std::num::NonZeroU32;
use thiserror::Error;

// Export our submodules
pub mod beacon;
pub mod proposer;
pub mod relay;

// Re-export key types that users of this module will need
pub use beacon::BeaconConsensus;
pub use proposer::Proposer;
pub use relay::{ConsensusRelay, ConsensusMessage, RelayError};

use crate::storage::BlockStorage;

/// Configuration for the consensus system
#[derive(Clone)]
pub struct ConsensusConfig {
    /// Cryptographic signer for this node
    pub signer: Ed25519,
    /// Storage interface for blocks
    pub storage: BlockStorage,
    /// Region names in priority order
    pub regions: Vec<String>,
    /// Metrics registry
    pub registry: Arc<Mutex<Registry>>,
    /// Maximum time to wait for leader responses
    pub leader_timeout: Duration,
    /// Maximum time to wait for block notarization
    pub notarization_timeout: Duration,
    /// Size of the message mailbox
    pub mailbox_size: usize,
    /// Number of concurrent replay operations
    pub replay_concurrency: usize,
}

impl ConsensusConfig {
    /// Creates a new configuration with default timeouts
    pub fn new(
        signer: Ed25519,
        storage: BlockStorage,
        regions: Vec<String>,
        registry: Arc<Mutex<Registry>>,
    ) -> Self {
        Self {
            signer,
            storage,
            regions,
            registry,
            leader_timeout: Duration::from_secs(1),
            notarization_timeout: Duration::from_secs(2),
            mailbox_size: 1024,
            replay_concurrency: 4,
        }
    }

    /// Customizes the leader timeout
    pub fn with_leader_timeout(mut self, timeout: Duration) -> Self {
        self.leader_timeout = timeout;
        self
    }

    /// Customizes the notarization timeout
    pub fn with_notarization_timeout(mut self, timeout: Duration) -> Self {
        self.notarization_timeout = timeout;
        self
    }
}

/// Core consensus state
#[derive(Debug, Clone)]
pub struct ConsensusState {
    /// Current view number
    pub view: u64,
    /// Hash of the latest finalized block
    pub latest_hash: [u8; 32],
    /// Number of the latest finalized block
    pub latest_number: u64,
}

/// Errors that can occur during consensus operations
#[derive(Debug, Error)]
pub enum ConsensusError {
    #[error("Network error: {0}")]
    Network(String),

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Relay error: {0}")]
    Relay(#[from] RelayError),

    #[error("Timeout waiting for consensus")]
    Timeout,

    #[error("Invalid block proposal")]
    InvalidProposal,

    #[error("Failed to achieve quorum")]
    NoQuorum,

    #[error("Internal error: {0}")]
    Internal(String),
}

/// Initialize the consensus system with the given configuration
pub async fn init_consensus<E>(
    runtime: E,
    network: Sender,
    config: ConsensusConfig,
) -> Result<(Engine<E, Ed25519, Proposer, ConsensusRelay, BeaconConsensus>, ConsensusRelay), ConsensusError>
where
    E: Runtime + Clone + 'static,
{
    // Initialize our relay first since other components need it
    let relay = ConsensusRelay::new(network, config.storage.clone());

    // Initialize beacon for leader election
    let beacon = BeaconConsensus::new(config.regions);

    // Register ourselves as a validator
    relay.announce_validator(
        config.signer.public_key().to_vec(),
        config.regions[0].clone(), // Our primary region
    ).await.map_err(ConsensusError::Relay)?;

    // Initialize proposer
    let proposer = Proposer::new(config.storage.clone());

    // Configure the consensus engine
    let engine_config = SimplexConfig {
        crypto: config.signer,
        automaton: proposer,
        relay: relay.clone(),
        supervisor: beacon,
        registry: config.registry,
        mailbox_size: config.mailbox_size,
        namespace: b"romer-consensus-v1".to_vec(),
        replay_concurrency: config.replay_concurrency,
        leader_timeout: config.leader_timeout,
        notarization_timeout: config.notarization_timeout,
        nullify_retry: Duration::from_secs(1),
        activity_timeout: 100,
        fetch_timeout: Duration::from_secs(5),
        max_fetch_count: 1000,
        max_fetch_size: 1024 * 1024,
        fetch_rate_per_peer: Quota::per_second(NonZeroU32::new(10).unwrap()),
        fetch_concurrent: 4,
    };

    // Create and return both the engine and the relay
    // We return the relay so the node can process incoming messages
    let engine = Engine::new(runtime, engine_config);
    Ok((engine, relay))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_consensus_initialization() {
        let signer = Ed25519::generate();
        let network = Sender::default();
        let registry = Arc::new(Mutex::new(Registry::default()));
        let storage = BlockStorage::new(runtime.clone(), registry.clone()).await.unwrap();
        let regions = vec!["Frankfurt".to_string(), "London".to_string()];

        let config = ConsensusConfig::new(signer, storage, regions, registry)
            .with_leader_timeout(Duration::from_secs(2))
            .with_notarization_timeout(Duration::from_secs(3));

        let (engine, relay) = init_consensus(runtime.clone(), network, config).await.unwrap();

        // The engine and relay should both be properly initialized
        assert!(engine.is_initialized());
        
        // We should be registered in our primary region
        // This would verify through the relay's internal state
    }
}