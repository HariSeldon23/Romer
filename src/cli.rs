use clap::{Parser, command};
use std::net::SocketAddr;

/// Command line interface configuration for the Rømer Chain node
#[derive(Parser, Debug)]
#[command(
    name = "Rømer Chain",
    author = "Rømer Chain Development Team",
    version = "0.1.0",
    about = "A blockchain designed to thrive in bear markets with physical infrastructure requirements",
    long_about = "Rømer Chain implements a novel Proof of Physics consensus mechanism \
                  combined with Austrian economics principles to create a blockchain that \
                  maintains strength during market downturns. The system requires physical \
                  infrastructure distribution and provides stable computation costs."
)]
pub struct NodeCliArgs {
    /// Network address for this node in the format IP:PORT
    /// Example: 127.0.0.1:8000
    #[arg(
        short, 
        long,
        default_value = "127.0.0.1:8000",
        help = "The network address this node will listen on"
    )]
    pub address: SocketAddr,

    /// Designates this node as a genesis node that will initialize the blockchain
    #[arg(
        short,
        long,
        help = "Start this node as a genesis node that will create the initial blockchain state",
        long_help = "When set, this node will create the genesis block and initialize the blockchain. \
                     Only one genesis node should exist per network. All other nodes should connect \
                     to an existing network through a bootstrap node."
    )]
    pub genesis: bool,

    /// Address of an existing node to bootstrap from
    #[arg(
        short,
        long,
        help = "Address of an existing node to connect to",
        long_help = "The network address of an existing node that will help bootstrap this node \
                     into the network. Required for all non-genesis nodes. \
                     Format: IP:PORT (e.g. 127.0.0.1:8000)",
        required_unless_present = "genesis",
        conflicts_with = "genesis"
    )]
    pub bootstrap: Option<String>,

    /// Log level for node operation
    #[arg(
        short,
        long,
        default_value = "info",
        help = "Set the logging level",
        value_parser = ["error", "warn", "info", "debug", "trace"],
        long_help = "Controls how verbose the node's logging will be:\n\
                     - error: Only critical errors\n\
                     - warn:  Warnings and errors\n\
                     - info:  General information plus warnings and errors\n\
                     - debug: Detailed information for debugging\n\
                     - trace: Very verbose debugging information"
    )]
    pub log_level: String,
}

impl NodeCliArgs {
    /// Parses the log level string into a tracing::Level
    pub fn get_log_level(&self) -> tracing::Level {
        match self.log_level.as_str() {
            "error" => tracing::Level::ERROR,
            "warn" => tracing::Level::WARN,
            "info" => tracing::Level::INFO,
            "debug" => tracing::Level::DEBUG,
            "trace" => tracing::Level::TRACE,
            _ => tracing::Level::INFO,
        }
    }

    /// Returns the bootstrap address as a SocketAddr if provided
    pub fn get_bootstrap_addr(&self) -> Option<SocketAddr> {
        self.bootstrap
            .as_ref()
            .map(|addr| addr.parse().expect("Invalid bootstrap address"))
    }
}