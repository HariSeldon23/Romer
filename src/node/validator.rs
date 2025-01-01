use bytes::{Buf, BufMut, Bytes, BytesMut};
use commonware_consensus::simplex::{Config as ConsensusConfig, Engine};
use commonware_consensus::Automaton;
use commonware_cryptography::Ed25519;
use commonware_cryptography::Hasher;
use commonware_cryptography::Scheme;
use commonware_p2p::authenticated::{Config as P2PConfig, Network as P2PNetwork};
use commonware_p2p::Receiver;
use commonware_runtime::deterministic::Blob;
use commonware_runtime::deterministic::Context as RuntimeContext;
use commonware_runtime::deterministic::Context as Storage;
use commonware_runtime::Clock;
use commonware_runtime::Spawner;
use commonware_storage::journal::{Config as JournalConfig, Journal};
use governor::Quota;
use prometheus_client::registry::Registry;
use std::net::SocketAddr;
use std::num::NonZeroU32;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::{Duration, SystemTime};
use tracing::{error, info};

use crate::block::{Block, BlockHeader};
use crate::config::genesis::GenesisConfig;
use crate::config::storage::StorageConfig;
use crate::config::validator::ValidatorConfig;
use crate::consensus::automaton::BlockchainAutomaton;
use crate::regions::region::RegionConfig;
use crate::utils::utils::Sha256Hasher;

/// The main Node structure that coordinates all components
pub struct Node {
    runtime: RuntimeContext,
    automaton: BlockchainAutomaton,
    genesis_config: GenesisConfig,
    validator_config: ValidatorConfig,
    storage_config: StorageConfig,
}

impl Node {
    /// Creates a new Node instance
    pub fn new(runtime: RuntimeContext, signer: Ed25519) -> Self {
        // Load network-wide genesis configuration
        let genesis_config = match GenesisConfig::load_default() {
            Ok(config) => {
                info!("Genesis configuration loaded successfully");
                info!("Chain ID: {}", config.network.chain_id);
                config
            }
            Err(e) => {
                error!("Failed to load genesis configuration: {}", e);
                std::process::exit(1);
            }
        };

        // Create automaton with genesis config
        let automaton = BlockchainAutomaton::new(runtime.clone(), signer, genesis_config.clone());

        // Load Storage configuration
        let storage_config = match StorageConfig::load_default() {
            Ok(config) => {
                info!("Storage configuration loaded successfully");

                config
            }
            Err(e) => {
                error!("Failed to load storage configuration: {}", e);
                std::process::exit(1);
            }
        };

        let validator_config = match ValidatorConfig::load_validator_config() {
            Ok(config) => {
                let region_config = RegionConfig::load()
                    .expect("Region config should be valid as it was checked during validation");

                let city_key = config.city.to_lowercase().replace(" ", "-");
                let region_details = region_config
                    .regions
                    .city
                    .get(&city_key)
                    .expect("Region should exist as it was validated");

                info!("Validator configuration loaded successfully");
                info!("Region Details:");
                // Use a more console-friendly format with region code in brackets
                info!(
                    "  [{}] {} ({}, {})",
                    region_details.region_code, // We'll add this field
                    region_details.city,
                    region_details.jurisdiction_state,
                    region_details.jurisdiction_country
                );
                info!("  Internet Exchange: {}", region_details.internet_exchange);

                config
            }
            Err(e) => {
                error!("Failed to load validator configuration: {}", e);
                std::process::exit(1);
            }
        };

        Self {
            runtime,
            automaton,
            genesis_config,
            validator_config,
            storage_config,
        }
    }

    /// Main entry point for running the node
    pub async fn run(
        mut self,
        address: SocketAddr,
        is_genesis: bool,
        bootstrap: Option<SocketAddr>,
    ) {
        info!("Starting node at {}", address);

        // Initialize storage with journal configuration
        let journal_config = JournalConfig {
            registry: Arc::new(Mutex::new(Registry::default())),
            // Simply use the genesis partition name from storage config
            partition: self.storage_config.journal.partitions.genesis.clone(),
        };

        info!(
            "Creating journal with partition: {}",
            journal_config.partition
        );

        let mut journal = Journal::init(self.runtime.clone(), journal_config)
            .await
            .expect("Failed to create journal");

        info!("Commonware Journal Storage initialized...");

        // Configure P2P network with authentication
        let p2p_config = P2PConfig::recommended(
            self.automaton.signer.clone(),
            self.genesis_config.network.chain_id.as_bytes(),
            Arc::new(Mutex::new(Registry::default())),
            address,
            bootstrap
                .map(|addr| (self.automaton.signer.public_key(), addr))
                .into_iter()
                .collect(),
            self.genesis_config.networking.max_message_size,
        );

        let (mut network, mut oracle) = P2PNetwork::new(self.runtime.clone(), p2p_config);

        // Register the node's public key for network participation
        oracle.register(0, vec![self.automaton.signer.public_key()]);

        // Register all channels before moving network into async block

        // Main channel for block propagation
        let (sender, mut receiver) = network.register(
            0, // Main channel ID
            Quota::per_second(NonZeroU32::new(100).unwrap()),
            1024,    // Message buffer size
            Some(3), // Compression level
        );

        // Voter channel for consensus voting
        let (voter_sender, mut voter_receiver) = network.register(
            1, // Voter channel ID
            Quota::per_second(NonZeroU32::new(100).unwrap()),
            1024,
            Some(3),
        );

        // Resolver channel for consensus resolution
        let (resolver_sender, mut resolver_receiver) = network.register(
            2, // Resolver channel ID
            Quota::per_second(NonZeroU32::new(100).unwrap()),
            1024,
            Some(3),
        );

        self.automaton.set_sender(sender.clone());

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
            namespace: self.genesis_config.network.chain_id.as_bytes().to_vec(),
            replay_concurrency: 4,
            leader_timeout: Duration::from_millis(self.genesis_config.consensus.block_time_ms),
            notarization_timeout: Duration::from_millis(
                self.genesis_config.consensus.block_time_ms * 2,
            ),
            nullify_retry: Duration::from_millis(self.genesis_config.consensus.block_time_ms / 2),
            activity_timeout: 100,
            fetch_timeout: Duration::from_secs(5),
            max_fetch_count: 1000,
            max_fetch_size: self.genesis_config.networking.max_message_size,
            fetch_rate_per_peer: Quota::per_second(NonZeroU32::new(10).unwrap()),
            fetch_concurrent: 4,
        };

        // Initialize consensus engine
        let engine = Engine::new(self.runtime.clone(), journal, consensus_config);

        // Spawn network handler with message processing
        let network_handle = self.runtime.spawn("p2p", {
            let runtime = self.runtime.clone();
            async move {
                let message_handle = runtime.spawn("message_processor", async move {
                    while let Ok((sender_id, message)) = receiver.recv().await {
                        info!(
                            "Processing message from {}: {:?}",
                            hex::encode(&sender_id),
                            message
                        );
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
        engine
            .run(
                (voter_sender, voter_receiver),
                (resolver_sender, resolver_receiver),
            )
            .await;

        // Wait for network completion
        network_handle.await.expect("Network handler failed");
    }
}
