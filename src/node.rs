use commonware_runtime::deterministic::Context as RuntimeContext;
use commonware_cryptography::Ed25519;
use commonware_runtime::Spawner;
use commonware_runtime::Clock;
use commonware_runtime::deterministic::Blob;
use commonware_runtime::deterministic::Context as Storage;
use commonware_consensus::simplex::{Config as ConsensusConfig, Engine};
use commonware_consensus::Automaton;
use commonware_cryptography::Scheme;
use commonware_cryptography::Hasher;
use std::net::SocketAddr;
use std::time::{Duration, SystemTime};
use std::sync::Arc;
use prometheus_client::registry::Registry;
use governor::Quota;
use std::sync::Mutex;
use commonware_storage::journal::{Journal, Config as JournalConfig};
use std::num::NonZeroU32;
use commonware_p2p::authenticated::{Config as P2PConfig, Network as P2PNetwork};
use commonware_p2p::Receiver;
use tracing::{info, error};

use crate::automaton::BlockchainAutomaton;
use crate::utils::Sha256Hasher;
use crate::genesis_config::GenesisConfig;

/// The main Node structure that coordinates all components
pub struct Node {
    runtime: RuntimeContext,
    automaton: BlockchainAutomaton,
    config: GenesisConfig,
}

impl Node {
    /// Creates a new Node instance
    pub fn new(runtime: RuntimeContext, signer: Ed25519, config: GenesisConfig) -> Self {
        let automaton = BlockchainAutomaton::new(runtime.clone(), signer);
        Self { 
            runtime, 
            automaton,
            config,
        }
    }

    /// Initializes the genesis state for the blockchain
    async fn initialize_genesis_state(&mut self, journal: &mut Journal<Blob, Storage>) -> Result<(), Box<dyn std::error::Error>> {
        info!("Initializing genesis state for chain {}", self.config.network.chain_id);

        // Check if we're past genesis time
        let current_time = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)?
            .as_secs();
        
        if current_time < self.config.network.genesis_time {
            let wait_time = self.config.network.genesis_time - current_time;
            info!("Waiting {} seconds for genesis time...", wait_time);
            self.runtime.sleep(Duration::from_secs(wait_time)).await;
        }

        // Create and store the genesis block
        let genesis_block = self.automaton.genesis().await;
        
        // Store genesis block in the journal
        journal.append(0, genesis_block).await?;
        journal.sync(0).await?;

        info!("Genesis block created and stored");
        Ok(())
    }

    /// Main entry point for running the node
    pub async fn run(mut self, address: SocketAddr, is_genesis: bool, bootstrap: Option<SocketAddr>) {
        info!("Starting node at {}", address);

        // Initialize storage with journal configuration
        let journal_config = JournalConfig {
            registry: Arc::new(Mutex::new(Registry::default())),
            partition: format!("blockchain_data_{}", self.config.network.chain_id),
        };

        let mut journal = Journal::init(
            self.runtime.clone(),
            journal_config,
        ).await.expect("Failed to create journal");

        // Configure P2P network with authentication
        let p2p_config = P2PConfig::recommended(
            self.automaton.signer.clone(),
            self.config.network.chain_id.as_bytes(),
            Arc::new(Mutex::new(Registry::default())),
            address,
            bootstrap.map(|addr| (self.automaton.signer.public_key(), addr))
                .into_iter()
                .collect(),
            self.config.networking.max_message_size,
        );

        let (mut network, mut oracle) = P2PNetwork::new(self.runtime.clone(), p2p_config);

        // Register the node's public key for network participation
        oracle.register(0, vec![self.automaton.signer.public_key()]);

        // Register all channels before moving network into async block
        
        // Main channel for block propagation
        let (sender, mut receiver) = network.register(
            0, // Main channel ID
            Quota::per_second(NonZeroU32::new(100).unwrap()),
            1024, // Message buffer size
            Some(3) // Compression level
        );

        // Voter channel for consensus voting
        let (voter_sender, mut voter_receiver) = network.register(
            1, // Voter channel ID
            Quota::per_second(NonZeroU32::new(100).unwrap()),
            1024,
            Some(3)
        );
        
        // Resolver channel for consensus resolution
        let (resolver_sender, mut resolver_receiver) = network.register(
            2, // Resolver channel ID
            Quota::per_second(NonZeroU32::new(100).unwrap()),
            1024,
            Some(3)
        );

        self.automaton.set_sender(sender.clone());

        // Initialize genesis state if this is a genesis node
        if is_genesis {
            if let Err(e) = self.initialize_genesis_state(&mut journal).await {
                error!("Failed to initialize genesis state: {}", e);
                return;
            }
        }

        // Configure consensus engine
        let consensus_config = ConsensusConfig {
            crypto: Ed25519::from_seed(42),
            hasher: Sha256Hasher::new(),
            automaton: self.automaton.clone(),
            relay: self.automaton.clone(),
            committer: self.automaton.clone(),
            supervisor: self.automaton.clone(),
            registry: Arc::new(Mutex::new(Registry::default())),
            mailbox_size: 1024,
            namespace: self.config.network.chain_id.as_bytes().to_vec(),
            replay_concurrency: 4,
            leader_timeout: Duration::from_millis(self.config.consensus.block_time_ms),
            notarization_timeout: Duration::from_millis(self.config.consensus.block_time_ms * 2),
            nullify_retry: Duration::from_millis(self.config.consensus.block_time_ms / 2),
            activity_timeout: 100,
            fetch_timeout: Duration::from_secs(5),
            max_fetch_count: 1000,
            max_fetch_size: self.config.networking.max_message_size,
            fetch_rate_per_peer: Quota::per_second(NonZeroU32::new(10).unwrap()),
            fetch_concurrent: 4,
        };

        // Initialize consensus engine
        let engine = Engine::new(
            self.runtime.clone(),
            journal,
            consensus_config,
        );

        // Spawn network handler with message processing
        let network_handle = self.runtime.spawn("p2p", {
            let runtime = self.runtime.clone();
            async move {
                let message_handle = runtime.spawn("message_processor", async move {
                    while let Ok((sender_id, message)) = receiver.recv().await {
                        info!("Processing message from {}: {:?}", hex::encode(&sender_id), message);
                        // Message will be handled by automaton's verify method
                    }
                });

                // Run the network and wait for completion
                let network_result = network.run().await;
                
                // Ensure message processor completes
                message_handle.await.expect("Message processor failed");
                
                network_result
            }
        });

        info!("Starting consensus engine");
        
        // Run consensus engine with voter and resolver channels
        engine.run(
            (voter_sender, voter_receiver),
            (resolver_sender, resolver_receiver)
        ).await;

        // Wait for network completion
        network_handle.await.expect("Network handler failed");
    }
}