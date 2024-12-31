// src/domain/economics.rs

use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AustrianEconomicsConfig {
    pub base_threshold: f64,                // 50% utilization target
    pub operation_costs: OperationCosts,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationCosts {
    pub token_transfer: u64,       // Base cost in ROMER for transfers
}

// Example instantiation of genesis config
impl Default for AustrianEconomicsConfig {
    fn default() -> Self {
        Self {
            base_threshold: 0.5,    // 50% target utilization
            operation_costs: OperationCosts {
                token_transfer: 1000,  // 0.00001 ROMER (with 8 decimals)
            }
        }
    }
}