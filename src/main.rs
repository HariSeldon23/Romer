use clap::Parser;
use commonware_cryptography::Ed25519;
use commonware_p2p::{
    authenticated::{self, Network as AuthNetwork},
    Sender, Recipients, Receiver,
};
use commonware_runtime::tokio::{Executor, Config as RuntimeConfig};
use prometheus_client::registry::Registry;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use governor::Quota;
use std::num::NonZeroU32;
use tracing;

mod storage;
mod consensus;
mod metrics;

use crate::storage::BlockStorage;
use crate::consensus::{ConsensusConfig, init_consensus, ConsensusRelay};
use crate::metrics::NetworkMetrics;

// Protocol constants
const ROMER_NAMESPACE: &[u8] = b"romer-chain-v0.1";
const MAX_MESSAGE_SIZE: usize = 1024 * 1024; // 1 MB

/// Command line arguments for the node
#[derive(Parser, Debug)]
#[clap(author, version, about)]
struct Args {
    /// Private key seed for the validator
    #[clap(long)]
    key_seed: u64,

    /// Port to listen on
    #[clap(long, default_value = "30303")]
    port: u16,

    /// Validator's region
    #[clap(long)]
    region: String,

    /// Bootstrap nodes in the format key@ip:port
    #[clap(long, value_delimiter = ',')]
    bootstrappers: Vec<String>,

    /// Data directory for blockchain storage
    #[clap(long, default_value = "./data")]
    data_dir: String,
}

/// Configuration for setting up a validator node
#[derive(Clone)]
struct NodeConfig {
    key_seed: u64,
    port: u16,
    region: String,
    bootstrap_nodes: Vec<(Vec<u8>, SocketAddr)>, // (public_key, address)
    data_dir: String,
}

impl TryFrom<Args> for NodeConfig {
    type Error = Box<dyn std::error::Error>;

    fn try_from(args: Args) -> Result<Self, Self::Error> {
        let bootstrap_nodes = args.bootstrappers
            .iter()
            .map(|node| {
                let parts: Vec<&str> = node.split('@').collect();
                if parts.len() != 2 {
                    return Err("Bootstrap node format should be key@ip:port".into());
                }
                let peer_key = parts[0]
                    .parse::<u64>()
                    .map_err(|_| "Bootstrap key must be a number")?;
                let peer_verifier = Ed25519::from_seed(peer_key).public_key().to_vec();
                let addr = SocketAddr::from_str(parts[1])
                    .map_err(|_| "Invalid bootstrap node address")?;
                Ok((peer_verifier, addr))
            })
            .collect::<Result<Vec<_>, Box<dyn std::error::Error>>>()?;

        Ok(NodeConfig {
            key_seed: args.key_seed,
            port: args.port,
            region: args.region,
            bootstrap_nodes,
            data_dir: args.data_dir,
        })
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging with timestamps
    tracing_subscriber::fmt()
        .with_target(false)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .with_level(true)
        .init();

    let args = Args::parse();
    let config = NodeConfig::try_from(args)?;

    // Initialize runtime configuration
    let runtime_config = RuntimeConfig::default();
    let (executor, runtime) = Executor::init(runtime_config);

    // Start the executor with full node configuration
    executor.start(async move {
        // Initialize validator identity
        let signer = Ed25519::from_seed(config.key_seed);
        tracing::info!(
            public_key = hex::encode(signer.public_key()),
            region = config.region,
            "Initializing validator"
        );

        // Initialize metrics registry
        let registry = Arc::new(Mutex::new(Registry::default()));
        let network_metrics = Arc::new(NetworkMetrics::new(&mut registry.lock().unwrap()));

        // Initialize storage
        let storage = BlockStorage::new(
            runtime.clone(),
            registry.clone(),
        )
        .await
        .expect("Failed to initialize storage");

        // Configure P2P network
        let p2p_config = authenticated::Config::aggressive(
            signer.clone(),
            ROMER_NAMESPACE,
            registry.clone(),
            SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), config.port),
            config.bootstrap_nodes
                .iter()
                .map(|(key, addr)| (Bytes::copy_from_slice(key), *addr))
                .collect(),
            MAX_MESSAGE_SIZE,
        );

        // Initialize network
        let (mut network, _oracle) = AuthNetwork::new(runtime.clone(), p2p_config);

        // Set up consensus channels
        const CONSENSUS_CHANNEL: u32 = 1;
        let (consensus_sender, consensus_receiver) = network.register(
            CONSENSUS_CHANNEL,
            Quota::per_second(NonZeroU32::new(100).unwrap()),
            1024,
            Some(5),
        );

        // Initialize consensus with our region
        let regions = vec![
            "Frankfurt".to_string(),
            "London".to_string(),
            "Amsterdam".to_string(),
            "New York".to_string(),
            "Tokyo".to_string(),
        ];

        let consensus_config = ConsensusConfig::new(
            signer.clone(),
            storage.clone(),
            regions,
            registry.clone(),
        )
        .with_leader_timeout(Duration::from_secs(5))
        .with_notarization_timeout(Duration::from_secs(10));

        // Initialize consensus system
        let (consensus, relay) = init_consensus(
            runtime.clone(),
            consensus_sender,
            consensus_config,
        )
        .await
        .expect("Failed to initialize consensus");

        // Clone relay for message handler
        let message_relay = relay.clone();

        // Spawn consensus message handler
        runtime.spawn("consensus-handler", async move {
            while let Ok(msg) = consensus_receiver.recv().await {
                let peer_key = msg.sender().to_vec();
                let msg_data = msg.into_data();

                match bincode::deserialize(&msg_data) {
                    Ok(consensus_msg) => {
                        if let Err(e) = message_relay.handle_message(consensus_msg, peer_key).await {
                            tracing::warn!(
                                error = ?e,
                                "Failed to handle consensus message"
                            );
                        }
                    }
                    Err(e) => {
                        tracing::warn!(
                            error = ?e,
                            "Failed to deserialize consensus message"
                        );
                    }
                }
            }
        });

        // Spawn metrics monitoring
        let health_metrics = network_metrics.clone();
        runtime.spawn("metrics-health", async move {
            health_metrics.run_health_check().await;
        });

        // Register ourselves in our primary region
        relay.announce_validator(
            signer.public_key().to_vec(),
            config.region,
        ).await.expect("Failed to register validator");

        // Spawn consensus engine
        runtime.spawn("consensus", consensus.run());

        // Run the network
        network.run().await;
    });

    Ok(())
}