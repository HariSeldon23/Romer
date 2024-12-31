// src/domain/block.rs
use serde::{Serialize, Deserialize};
use std::time::SystemTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockHeader {
    pub view: u32, // You might go through multiple views before successfully finalizing a block at a given height
    pub height: u64,
    pub timestamp: SystemTime,
    pub previous_hash: [u8; 32],
    pub transactions_root: [u8; 32],
    pub state_root: [u8; 32],
    pub validator_public_key: PublicKey,
    pub utilization: f64,          // Current utilization vs base threshold
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub header: BlockHeader,
    pub transactions: Vec<Transaction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub transaction_type: TransactionType,
    pub from: String,              // Base58 encoded address
    pub nonce: u64,                // Transaction sequence number
    pub gas_amount: u64,           // Computed gas requirement
    pub signature: Signature,      // Transaction signature
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransactionType {
    TokenTransfer {
        to: String,                // Base58 encoded recipient
        amount: u64,               // Amount in smallest unit (8 decimals)
    }
}