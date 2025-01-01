// main.rs

mod block;
mod cmd;
mod config;
mod consensus;
mod identity;
mod node;
mod regions;
mod utils;

use clap::Parser;
use commonware_cryptography::{Ed25519, Scheme};
use commonware_runtime::deterministic::Executor;
use commonware_runtime::Runner;
use rand::rngs::OsRng;
use tracing::{error, info};

use crate::cmd::cli::NodeCliArgs;
use crate::identity::keys::NodeKeyManager;
use crate::node::validator::Node;

fn main() {
    // Parse command line arguments
    let args: NodeCliArgs = NodeCliArgs::parse();

    // Initialize logging with configured level
    tracing_subscriber::fmt()
        .with_max_level(args.get_log_level())
        .with_target(true)
        .init();

    let romer_ascii = r#"
    ██████╗  ██████╗ ███╗   ███╗███████╗██████╗ 
    ██╔══██╗██╔═══██╗████╗ ████║██╔════╝██╔══██╗
    ██████╔╝██║   ██║██╔████╔██║█████╗  ██████╔╝
    ██╔══██╗██║   ██║██║╚██╔╝██║██╔══╝  ██╔══██╗
    ██║  ██║╚██████╔╝██║ ╚═╝ ██║███████╗██║  ██║
    ╚═╝  ╚═╝ ╚═════╝ ╚═╝     ╚═╝╚══════╝╚═╝  ╚═╝
    "#;

    // Print the ASCII art to the console
    println!("{}", romer_ascii);

    info!("Starting Rømer Chain Node");
    info!("Using local address: {}", args.address);

    // Initialize the key manager
    let key_manager = match NodeKeyManager::new() {
        Ok(manager) => manager,
        Err(e) => {
            error!("Failed to initialize key manager: {}", e);
            std::process::exit(1);
        }
    };

    // Generate or load the validator key
    let signer = match key_manager.generate_key() {
        Ok(key) => key,
        Err(e) => {
            error!("Failed to generate validator key: {}", e);
            std::process::exit(1);
        }
    };

    info!("Validator key pair created");
    info!("Public key: {}", hex::encode(signer.public_key()));
    info!("Key stored at: {:?}", key_manager.key_path());

    // Initialize the Commonware Runtime
    let (executor, runtime, _) = Executor::default();
    info!("Default Commonware Runtime initialized");

    // Create and run the node with both configurations
    info!("Starting Node initialization...");
    let node = Node::new(runtime.clone(), signer);

    info!("Node initialized");

    Runner::start(executor, async move {
        node.run(args.address, args.genesis, args.get_bootstrap_addr())
            .await;
    });
}
