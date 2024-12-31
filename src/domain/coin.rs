// src/domain/coin.rs

use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenConfig {
    pub name: String,              // "RØMER"
    pub symbol: String,            // "ROMER"
    pub decimals: u8,              // 8 decimals
    pub initial_supply: u64,       // Total Genesis supply
    pub distribution: TokenDistribution,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenDistribution {
    pub treasury: u64,             // 50% to treasury
    pub mainnet_contributors: u64, // 20% to contributors
    pub burn_reserve: u64,         // 30% to burn reserve
}