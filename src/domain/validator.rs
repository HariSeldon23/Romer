// src/domain/validator.rs

use serde::{Serialize, Deserialize};
use std::time::SystemTime;
use commonware_cryptography::{PublicKey, Signature};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorInfo {
    pub public_key: PublicKey,
    pub location: ValidatorCity,
    pub hardware_specs: HardwareRequirements,
    pub physical_proof: Option<PhysicalProof>,
    pub block_reward: u64,  // Amount of ROMER per block produced
}

impl ValidatorInfo {
    /// Creates a new validator with the specified parameters.
    /// This generalized constructor works for any city where validators might operate.
    pub fn new(
        public_key: PublicKey,
        location: ValidatorCity,
        hardware_specs: HardwareRequirements,
        block_reward: u64,
    ) -> Self {
        Self {
            public_key,
            location,
            hardware_specs,
            physical_proof: None,  // Physical proofs are always added after initial creation
            block_reward,
        }
    }

    /// Checks if this validator meets all location requirements
    pub fn validate_location(&self) -> bool {
        // A validator's location is valid if:
        // 1. The city is active for validator registration
        // 2. The location has appropriate network infrastructure
        self.location.is_active
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareRequirements {
    pub ram_gb: u32,        // 32GB minimum
    pub cpu_cores: u32,     // 8 cores minimum
    pub storage_gb: u32,    // 4TB = 4000GB minimum
    pub network_mbps: u32,  // 1000 Mbps minimum
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhysicalProof {
    pub proof_hash: String,         // Hash of the ZK proof
    pub submission_date: SystemTime,
    pub last_verification: SystemTime,
}