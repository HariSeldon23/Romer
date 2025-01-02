use std::net::IpAddr;
use std::sync::Arc;
use std::time::Duration;

use commonware_cryptography::Ed25519;
use commonware_runtime::deterministic::Context as RuntimeContext;
use commonware_runtime::Clock;
use commonware_runtime::Spawner;
use commonware_p2p::authenticated::{Config as P2PConfig, Network as P2PNetwork};
use commonware_storage::journal::{Config as JournalConfig, Journal};
use commonware_runtime::deterministic::Blob;
use commonware_runtime::deterministic::Context as Storage;

use governor::Quota;
use prometheus_client::registry::Registry;
use std::num::NonZeroU32;
use std::sync::Mutex;
use std::net::SocketAddr;
use tracing::{error, info, warn};

use crate::config::genesis::GenesisConfig;
use crate::config::storage::StorageConfig;
use crate::config::validator::ValidatorConfig;
use crate::consensus::automaton::BlockchainAutomaton;
use crate::regions::region::RegionConfig;
use crate::location::LocationVerificationService;

/// The main Node structure that coordinates all components
pub struct Node {
    runtime: RuntimeContext,
    genesis_config: GenesisConfig,
    storage_config: StorageConfig,
    validator_config: ValidatorConfig,
    signer: Ed25519,
}

impl Node {
    /// Prepares the node's operational context by loading and validating configurations
    async fn configure_node_context(
        runtime: RuntimeContext, 
        signer: Ed25519,
        validator_ip: IpAddr
    ) -> Result<Self, Box<dyn std::error::Error>> {
        // Location Verification Step
        let location_service = LocationVerificationService::new();
        let verification_result = location_service.verify_location(validator_ip).await;

        // Validate location before proceeding
        if !verification_result.is_verified {
            return Err("Location verification failed".into());
        }

        // Load region configuration to validate detected region
        let region_config = RegionConfig::load()
            .map_err(|e| format!("Failed to load region configuration: {}", e))?;

        // Get the estimated region from location verification
        let estimated_region = verification_result.estimated_region
            .ok_or("No region could be estimated")?;

        // Find the corresponding city in the region configuration
        let (city_key, region_details) = region_config.regions.city
            .iter()
            .find(|(_, details)| details.region.to_lowercase() == estimated_region.to_lowercase())
            .ok_or("No matching city found for estimated region")?;

        // Log detailed region information
        info!("Verified Validator Location:");
        info!(
            "  [{}] {} ({}, {})",
            region_details.region_code,
            region_details.city,
            region_details.jurisdiction_state,
            region_details.jurisdiction_country
        );
        info!("  Internet Exchange: {}", region_details.internet_exchange);
        info!("  Network Performance: {:?}", verification_result.network_performance);

        // Load other configurations
        let genesis_config = GenesisConfig::load_default()
            .map_err(|e| format!("Failed to load genesis configuration: {}", e))?;
        
        let storage_config = StorageConfig::load_default()
            .map_err(|e| format!("Failed to load storage configuration: {}", e))?;

        // Create a ValidatorConfig based on the verified location
        let validator_config = ValidatorConfig {
            city: region_details.city.clone(),
        };

        Ok(Self {
            runtime,
            genesis_config,
            storage_config,
            validator_config,
            signer,
        })
    }

    /// Async constructor using the new configuration method
    pub async fn try_new(
        runtime: RuntimeContext, 
        signer: Ed25519,
        validator_ip: IpAddr
    ) -> Result<Self, Box<dyn std::error::Error>> {
        Self::configure_node_context(runtime, signer, validator_ip).await
    }

    /// Synchronous constructor (use with caution)
    pub fn new(
        runtime: RuntimeContext, 
        signer: Ed25519,
        validator_ip: IpAddr
    ) -> Self {
        tokio::runtime::Runtime::new()
            .expect("Failed to create Tokio runtime")
            .block_on(async move {
                Self::configure_node_context(runtime, signer, validator_ip)
                    .await
                    .expect("Failed to configure node context")
            })
    }

    /// Initializes the genesis state for the blockchain
    async fn initialize_genesis_state(
        &mut self, 
        journal: &mut Journal<Blob, Storage>
    ) -> Result<(), Box<dyn std::error::Error>> {
        info!(
            "Initializing genesis state for chain {}",
            self.genesis_config.network.chain_id
        );

        // Check genesis time logic (similar to previous implementation)
        let current_time = self.runtime.current()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)?
            .as_secs();

        if current_time < self.genesis_config.network.genesis_time {
            let wait_time = self.genesis_config.network.genesis_time - current_time;
            info!("Waiting {} seconds for genesis time...", wait_time);
            self.runtime.sleep(Duration::from_secs(wait_time)).await;
        }

        // Initialize the automaton for genesis block creation
        let automaton = BlockchainAutomaton::new(
            self.runtime.clone(), 
            self.signer.clone(), 
            self.genesis_config.clone(),
            self.storage_config.clone()
        );

        // Create and store genesis block
        let genesis_block = automaton.genesis().await;
        journal.append(0, genesis_block).await?;
        journal.sync(0).await?;

        info!("Genesis block created and stored successfully");
        Ok(())
    }

    /// Main entry point for running the node
    pub async fn run(
        &mut self,
        address: SocketAddr,
        bootstrap: Option<SocketAddr>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        info!("Starting node at {}", address);

        // Journal configuration using storage config
        let journal_config = JournalConfig {
            registry: Arc::new(Mutex::new(Registry::default())),
            partition: self.storage_config.journal.partitions.genesis.clone(),
        };

        let mut journal = Journal::init(self.runtime.clone(), journal_config)
            .await
            .map_err(|e| format!("Failed to initialize journal: {}", e))?;

        // Initialize genesis state if needed
        self.initialize_genesis_state(&mut journal).await?;

        // Configure P2P network 
        let p2p_config = P2PConfig::recommended(
            self.signer.clone(),
            self.genesis_config.network.chain_id.as_bytes(),
            Arc::new(Mutex::new(Registry::default())),
            address,
            bootstrap
                .map(|addr| (self.signer.public_key(), addr))
                .into_iter()
                .collect(),
            self.genesis_config.networking.max_message_size,
        );

        let (network, _oracle) = P2PNetwork::new(self.runtime.clone(), p2p_config);

        // Additional setup steps would follow here
        // Such as registering network channels, starting consensus engine, etc.

        Ok(())
    }
}