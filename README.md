# Rømer Chain

## Prerequisites

You must have Rust installed on your system. If you haven't installed it yet, run:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

## Building

Clone and build the project:

```bash
cargo build --release
```

## Running a Node

### Starting the First Node

To start the first node in the network:

```bash
cargo run --release -- node --key 1234 --region sydney --port 30303
```

**Please note** Your private key is not stored anywhere and is regenerated everytime you use the key to seed.

### Joining an Existing Network

To connect to an existing network, you'll need to know the key and address of at least one running node. Then start your node with the bootstrappers flag:

```bash
cargo run --release -- node --key 5678 --region amsterdam --port 30304 --bootstrappers 1234@127.0.0.1:30303
```

### Command Options

The node command accepts these arguments:

```bash
--key          Required. Your validator's private key seed (any number)
--region       Required. Your validator's region
--port         Optional. Network port (default: 30303)
--bootstrappers Optional. Comma-separated list of bootstrap nodes (format: key@ip:port)
```

### Available Regions

Use any of these regions when starting your node:

Frankfurt, Amsterdam, London, Ashburn VA, New York/NJ, Tokyo, Singapore, Hong Kong, Sydney, São Paulo, Marseille, Los Angeles, Seattle, Miami, Toronto, Dubai, Mumbai, Chennai, Fortaleza, Manila, Stockholm, Warsaw, Istanbul, Cairo, Moscow, Beijing, Seoul, Taipei, Jakarta, Auckland, Paris, Madrid, Milan, Vienna, Prague, Copenhagen, Helsinki, Tel Aviv, Johannesburg, Lagos, Nairobi, Cape Town, Panama City, Santiago, Vancouver, Perth, Kuala Lumpur, Muscat, Dublin, Montreal