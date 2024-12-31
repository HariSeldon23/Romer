// network.rs
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkParameters {
    // Network identifier for address prefixing and cross-network protection
    network_id: u8,
    // Protocol version for future upgrades
    protocol_version: u8,
}

impl NetworkParameters {
    // Create new parameters for mainnet
    pub fn mainnet() -> Self {
        Self {
            network_id: 0x3C, // Decimal 60 for Rømer mainnet
            protocol_version: 0x01,
        }
    }

    // Create new parameters for testnet
    pub fn testnet() -> Self {
        Self {
            network_id: 0x3D, // Decimal 61 for Rømer testnet
            protocol_version: 0x01,
        }
    }

    pub fn network_id(&self) -> u8 {
        self.network_id
    }

    pub fn protocol_version(&self) -> u8 {
        self.protocol_version
    }
}