use commonware_cryptography::Ed25519;
use commonware_runtime::deterministic::Context as RuntimeContext;
use std::net::SocketAddr;
use tracing::{error, info};

use crate::config::genesis::GenesisConfig;
use crate::config::storage::StorageConfig;
use crate::config::validator::ValidatorConfig;
use crate::consensus::automaton::BlockchainAutomaton;
use crate::regions::region::RegionConfig;

/// The main Node structure that coordinates all components
pub struct Node {
    runtime: RuntimeContext,
    genesis_config: GenesisConfig,
    validator_config: ValidatorConfig,
    storage_config: StorageConfig,
    signer: Ed25519,
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
                info!(
                    "  [{}] {} ({}, {})",
                    region_details.region_code,
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
            genesis_config,
            validator_config,
            storage_config,
            signer,
        }
    }

    /// Main entry point for running the node
    pub async fn run(
        &self,
        address: SocketAddr,
        bootstrap: Option<SocketAddr>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        info!("Starting node at {}", address);

        // Initialize the automaton within the run method
        let automaton = BlockchainAutomaton::new(
            self.runtime.clone(), 
            self.signer.clone(), 
            self.genesis_config.clone()
        );

        // Additional node startup logic can be added here
        // For example, network initialization, consensus engine setup, etc.

        Ok(())
    }
}