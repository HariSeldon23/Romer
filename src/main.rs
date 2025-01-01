// main.rs

mod cmd;
mod node;
mod consensus;
mod config;
mod utils;
mod block;

use clap::Parser;
use commonware_cryptography::{Ed25519, Scheme};
use commonware_runtime::Runner;
use commonware_runtime::deterministic::Executor;
use tracing::{info, error};

use crate::node::validator::Node;
use crate::cmd::cli::NodeCliArgs;

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

    // Initialize the Commonware Runtime
    let (executor, runtime, _) = Executor::default();
    info!("Default Commonware Runtime initialized");

    // Create node identity
    use rand::rngs::OsRng;
    let signer = Ed25519::new(&mut OsRng);
    info!("Validator key pair created");
    info!("Public key: {}", hex::encode(signer.public_key()));

    // Create and run the node with both configurations
    info!("Starting Node initialization...");
    let node = Node::new(
        runtime.clone(), 
        signer,
    );
    
    info!("Node initialized");
    
    Runner::start(executor, async move {
        node.run(
            args.address,
            args.genesis,
            args.get_bootstrap_addr(),
        ).await;
    });
}




