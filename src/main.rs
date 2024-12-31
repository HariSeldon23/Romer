mod automaton;  
mod validator;
mod utils;
mod genesis_config;
mod cli;  

use clap::Parser;
use commonware_cryptography::{Ed25519, Scheme};
use commonware_runtime::Runner;
use commonware_runtime::deterministic::Executor;
use tracing::{info, error};

use crate::validator::Node;
use crate::genesis_config::GenesisConfig;
use crate::cli::NodeCliArgs;

fn main() {
    // Parse command line arguments
    let args: NodeCliArgs = NodeCliArgs::parse();

    // Initialize logging with configured level
    tracing_subscriber::fmt()
        .with_max_level(args.get_log_level())
        .with_target(true)
        .init();
    
    info!("Starting Rømer Chain Node");
    info!("Using local address: {}", args.address);

    // Load the genesis configuration
    let genesis_config = match GenesisConfig::load_default() {
        Ok(config) => {
            info!("Genesis config loaded successfully");
            config
        },
        Err(e) => {
            error!("Failed to load genesis config: {}", e);
            std::process::exit(1);
        }
    };

    // Initialize the Commonware Runtime
    let (executor, runtime, _) = Executor::default();
    info!("Commonware Runtime initialized");

    // Create node identity
    let signer = Ed25519::from_seed(42);
    info!("Node identity created");

    // Create and run the node with configuration
    let node = Node::new(runtime.clone(), signer, genesis_config);

    info!("New Node spun up");
    
    Runner::start(executor, async move {
        node.run(
            args.address,
            args.genesis,
            args.get_bootstrap_addr(),
        ).await;
    });
}