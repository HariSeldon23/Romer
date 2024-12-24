use clap::Parser;
use commonware_cryptography::{Ed25519, Scheme};
use commonware_runtime::{Runner};
use commonware_runtime::deterministic::{Executor};
use std::net::SocketAddr;

// AUTOMATON
mod automaton;  
mod node;
mod utils;
mod genesis_config;
use crate::node::Node;
use crate::genesis_config::GenesisConfig;

// Command line arguments for node configuration
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct NodeCliArgs {
    /// Node's network address
    #[arg(short, long, default_value = "127.0.0.1:8000")]
    address: SocketAddr,

    /// Genesis node flag
    #[arg(short, long)]
    genesis: bool,

    /// Bootstrap node address (required for non-genesis nodes)
    #[arg(short, long)]
    bootstrap: Option<String>,
}

fn main() {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Parse command line arguments
    let args = NodeCliArgs::parse();

    // Load the genesis configuration
    let genesis_config = GenesisConfig::load_default()
        .expect("Failed to load genesis configuration");

    // Parse network addresses
    let local_addr: SocketAddr = args.address;
    let bootstrap_addr = args.bootstrap
        .map(|addr| addr.parse().expect("Invalid bootstrap address"));

    // Initialize the Commonware Runtime
    let (executor, runtime, _) = Executor::default();

    // Create node identity
    let signer = Ed25519::from_seed(42);

    // Create and run the node with configuration
    let node = Node::new(runtime.clone(), signer, genesis_config);
    
    Runner::start(executor, async move {
        node.run(
            local_addr,
            args.genesis,
            bootstrap_addr,
        ).await;
    });
}