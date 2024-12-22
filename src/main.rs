use serde::{Serialize, Deserialize};
use commonware_runtime::{tokio::{Executor, Config}, Runner, Spawner};
use commonware_cryptography::{Ed25519, Scheme};
use commonware_p2p::{authenticated, Sender, Receiver, Recipients};
use commonware_p2p::authenticated::Network as AuthNetwork;
use clap::{Command, Arg};
use prometheus_client::registry::Registry;
use std::sync::{Arc, Mutex};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::str::FromStr;
use governor::Quota;
use std::num::NonZeroU32;
use bytes::Bytes;
use tracing;

// Unique namespace for our network protocol
const ROMER_NAMESPACE: &[u8] = b"romer-chain-v0.1";
const MAX_MESSAGE_SIZE: usize = 1024 * 1024; // 1 MB

/// Configuration for a validator node
struct NodeConfig {
    key_seed: u64,
    port: u16,
    region: String,
    bootstrap_nodes: Vec<(Vec<u8>, SocketAddr)>  // (public_key, address)
}

/// Network announcement message format
#[derive(Serialize, Deserialize)]
struct NetworkAnnouncement {
    public_key: Vec<u8>,
    region: String,
    timestamp: u64,
}

/// PeerDiscovery handles the peer discovery protocol
struct PeerDiscovery {
    identity: Ed25519,
    region: String,
}

impl PeerDiscovery {
    fn new(identity: Ed25519, region: String) -> Self {
        Self { identity, region }
    }

    async fn run<S, R>(self, mut sender: S, mut receiver: R)
    where
        S: Sender,
        R: Receiver,
    {
        // Log that our discovery service is starting
        tracing::info!("Starting peer discovery service");
        // Create our network announcement
        let announcement = NetworkAnnouncement {
            public_key: self.identity.public_key().to_vec(),
            region: self.region.clone(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };

        // Broadcast to all peers
        match sender
            .send(
                Recipients::All,
                bincode::serialize(&announcement).unwrap().into(),
                false,
            )
            .await
        {
            Ok(_) => {
                tracing::info!(
                    region = self.region,
                    "Broadcasted our presence to the network"
                );
            }
            Err(e) => {
                tracing::error!("Failed to send announcement: {:?}", e);
            }
        }

        // Handle incoming announcements
        while let Ok((peer_key, msg)) = receiver.recv().await {
            match bincode::deserialize::<NetworkAnnouncement>(&msg) {
                Ok(announcement) => {
                    tracing::info!(
                        peer_id = hex::encode(&peer_key),
                        peer_region = announcement.region,
                        timestamp = announcement.timestamp,
                        "New peer joined the network"
                    );
                }
                Err(e) => {
                    tracing::warn!(
                        peer = hex::encode(&peer_key),
                        "Failed to deserialize announcement: {:?}",
                        e
                    );
                }
            }
        }
    }
}

fn main() {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    // Initialize runtime with default configuration
    let runtime_config = Config::default();
    let (executor, runtime) = Executor::init(runtime_config);

    // Parse command line arguments
    let matches = Command::new("romer")
        .about("Rømer Chain validator node")
        .subcommand(
            Command::new("node")
                .about("Run a validator node")
                .arg(
                    Arg::new("key")
                        .long("key")
                        .required(true)
                        .help("Validator's Ed25519 private key seed"),
                )
                .arg(
                    Arg::new("port")
                        .long("port")
                        .default_value("30303")
                        .help("Port to listen on"),
                )
                .arg(
                    Arg::new("bootstrappers")
                        .long("bootstrappers")
                        .required(false)
                        .value_delimiter(',')
                        .help("Comma-separated list of bootstrap nodes (key@ip:port)"),
                )
                .arg(
                    Arg::new("region")
                        .long("region")
                        .required(true)
                        .help("Validator's region"),
                ),
        )
        .get_matches();

    // Extract configuration into owned structure
    if let Some(node_matches) = matches.subcommand_matches("node") {
        let config = NodeConfig {
            key_seed: node_matches
                .get_one::<String>("key")
                .expect("Please provide a key seed")
                .parse::<u64>()
                .expect("Key seed must be a number"),
            port: node_matches
                .get_one::<String>("port")
                .unwrap_or(&"30303".to_string())
                .parse::<u16>()
                .expect("Invalid port"),
            region: node_matches
                .get_one::<String>("region")
                .expect("Please provide a region")
                .to_string(),
            bootstrap_nodes: if let Some(bootstrappers) = node_matches.get_many::<String>("bootstrappers") {
                bootstrappers
                    .map(|node| {
                        let parts: Vec<&str> = node.split('@').collect();
                        if parts.len() != 2 {
                            panic!("Bootstrap node format should be key@ip:port");
                        }
                        let peer_key = parts[0].parse::<u64>()
                            .expect("Bootstrap key must be a number");
                        let peer_verifier = Ed25519::from_seed(peer_key).public_key();
                        let addr = SocketAddr::from_str(parts[1])
                            .expect("Invalid bootstrap node address");
                        (peer_verifier.to_vec(), addr)
                    })
                    .collect()
            } else {
                Vec::new()
            },
        };

        // Start the executor with owned configuration
        executor.start(async move {
            // Initialize validator identity
            let signer = Ed25519::from_seed(config.key_seed);
            tracing::info!(
                key = hex::encode(&signer.public_key()),
                "Loaded validator identity"
            );

            // Create bootstrap nodes list
            let bootstrap_nodes: Vec<_> = config.bootstrap_nodes.iter()
                .map(|(key, addr)| (Bytes::copy_from_slice(key), *addr))
                .collect();

            // Configure P2P network
            let p2p_config = authenticated::Config::aggressive(
                signer.clone(),
                ROMER_NAMESPACE,
                Arc::new(Mutex::new(Registry::default())),
                SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), config.port),
                bootstrap_nodes,
                MAX_MESSAGE_SIZE,
            );

            // Initialize network
            let (mut network, _oracle) = AuthNetwork::new(runtime.clone(), p2p_config);

            // Set up discovery channel
            const DISCOVERY_CHANNEL: u32 = 1;
            let (discovery_sender, discovery_receiver) = network.register(
                DISCOVERY_CHANNEL,
                Quota::per_second(NonZeroU32::new(10).unwrap()),
                256, // message backlog
                Some(3), // compression level
            );

            // Create and spawn peer discovery handler
            let discovery = PeerDiscovery::new(
                signer,
                config.region,
            );
            runtime.spawn(
                "discovery",
                discovery.run(discovery_sender, discovery_receiver),
            );

            // Set up network monitoring
            const NETWORK_EVENTS_CHANNEL: u32 = 2;
            let (events_sender, mut events_receiver) = network.register(
                NETWORK_EVENTS_CHANNEL,
                Quota::per_second(NonZeroU32::new(100).unwrap()),
                1024, // larger backlog for events
                None, // no compression for events
            );
            
            // Spawn network event monitor
            runtime.spawn(
                "network-monitor",
                async move {
                    while let Ok((peer_key, _)) = events_receiver.recv().await {
                        tracing::info!(
                            peer = hex::encode(&peer_key),
                            "Network connection established"
                        );
                    }
                },
            );

            // Run the network
            network.run().await;
        });
    }
}