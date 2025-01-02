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
use commonware_cryptography::Scheme;
use commonware_runtime::deterministic::Executor;
use commonware_runtime::Runner;
use node::hardware::VirtualizationType;
use tracing::{error, info};

use crate::cmd::cli::NodeCliArgs;
use crate::identity::keys::NodeKeyManager;
use crate::node::hardware::HardwareVerifier;
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

    let verifier = HardwareVerifier::new();

    // Run the hardware verification test
    match verifier.verify() {
        Ok((virtualization_type, result)) => {
            // Print operating system information
            println!("Operating System: {:?}", HardwareVerifier::detect_os());

            // Print virtualization details
            match virtualization_type {
                VirtualizationType::Physical => {
                    println!("Hardware Type: Physical Machine");
                }
                VirtualizationType::Virtual(virt_type) => {
                    println!("Hardware Type: Virtual Environment ({})", virt_type);
                }
            }

            // Print performance details
            println!("Performance Details:");
            println!("  Operations per second: {}", result.ops_per_second);
            println!("  Performance score: {:.2}", result.performance_score);
            println!("  Test duration: {:?}", result.test_duration);
        }
        Err(err) => {
            eprintln!("Hardware Verification Failed: {:?}", err);
            std::process::exit(1);
        }
    }

    // Initialize the key manager
    let key_manager = match NodeKeyManager::new() {
        Ok(manager) => manager,
        Err(e) => {
            error!("Failed to initialize key manager: {}", e);
            std::process::exit(1);
        }
    };

    // Check for existing key, generate if not found
    let signer = match key_manager.check_existing_key() {
        Ok(Some(existing_key)) => {
            info!("Loaded existing validator key");
            existing_key
        }
        Ok(None) => {
            // No existing key, generate a new one
            match key_manager.generate_key() {
                Ok(new_key) => {
                    info!("Generated new validator key");
                    new_key
                }
                Err(e) => {
                    error!("Failed to generate validator key: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Err(e) => {
            error!("Error checking existing key: {}", e);
            std::process::exit(1);
        }
    };

    info!("Validator key ready");
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
        node.run(args.address, args.get_bootstrap_addr()).await;
    });
}
