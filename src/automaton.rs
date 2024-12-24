use commonware_consensus::{Automaton, simplex::Context};
use commonware_runtime::deterministic::Context as RuntimeContext;
use commonware_cryptography::{Ed25519, PublicKey, Scheme};
use commonware_p2p::{Recipients, Sender};  // Removed unused Receiver import
use commonware_consensus::{Relay, Committer, Supervisor};
use commonware_runtime::Clock;
use bytes::Bytes;
use std::time::SystemTime;
use tracing::{info, warn};
use futures::channel::oneshot;
use futures::Future;

#[derive(Clone)]
pub struct BlockchainAutomaton {
    runtime: RuntimeContext,
    p2p_sender: Option<commonware_p2p::authenticated::Sender>,
    pub signer: Ed25519,
}

impl BlockchainAutomaton {
    pub fn new(runtime: RuntimeContext, signer: Ed25519) -> Self {
        Self {
            runtime,
            p2p_sender: None,
            signer,
        }
    }

    pub fn set_sender(&mut self, sender: commonware_p2p::authenticated::Sender) {
        self.p2p_sender = Some(sender);
    }
}

impl Automaton for BlockchainAutomaton {
    type Context = Context;

    async fn genesis(&mut self) -> Bytes {
        let timestamp: u64 = self.runtime.current()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        Bytes::from(format!("Genesis Block: {}", timestamp))
    }

    // Changed to return the Future directly instead of nesting it
    async fn propose(&mut self, context: Self::Context) -> oneshot::Receiver<Bytes> {
        let timestamp: u64 = self.runtime.current()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let block = Bytes::from(format!("Block at view {}: {}", context.view, timestamp));
        
        if let Some(sender) = &mut self.p2p_sender {
            if let Err(e) = sender.send(Recipients::All, block.clone(), true).await {
                warn!("Failed to broadcast block: {}", e);
            }
        }

        // Create and return the receiver directly
        let (tx, rx) = oneshot::channel();
        let _ = tx.send(block);
        rx
    }

    // Changed to return the Future directly instead of nesting it
    async fn verify(&mut self, context: Self::Context, payload: Bytes) -> oneshot::Receiver<bool> {
        let is_valid = if payload.is_empty() {
            warn!("Received empty payload at view {}", context.view);
            false
        } else {
            match String::from_utf8(payload.to_vec()) {
                Ok(block_content) => {
                    let is_valid = block_content.contains(&format!("view {}", context.view));
                    if is_valid {
                        if let Some(sender) = &mut self.p2p_sender {
                            let validation_message = Bytes::from(format!(
                                "Block validated for view {}: {}",
                                context.view, block_content
                            ));
                            if let Err(e) = sender.send(Recipients::All, validation_message, true).await {
                                warn!("Failed to broadcast validation: {}", e);
                            }
                        }
                    }
                    is_valid
                },
                Err(_) => {
                    warn!("Invalid UTF-8 payload at view {}", context.view);
                    false
                }
            }
        };

        // Create and return the receiver directly
        let (tx, rx) = oneshot::channel();
        let _ = tx.send(is_valid);
        rx
    }
}

impl Relay for BlockchainAutomaton {
    async fn broadcast(&mut self, payload: Bytes) {
        if let Some(sender) = &mut self.p2p_sender {
            let mut sender = sender.clone();
            if let Err(e) = sender.send(Recipients::All, payload, true).await {
                warn!("Failed to broadcast: {}", e);
            }
        }
    }
}

impl Committer for BlockchainAutomaton {
    async fn prepared(&mut self, _proof: Bytes, payload: Bytes) {
        info!("Block prepared: {:?}", String::from_utf8_lossy(&payload));
    }

    async fn finalized(&mut self, _proof: Bytes, payload: Bytes) {
        info!("Block finalized: {:?}", String::from_utf8_lossy(&payload));
    }
}

impl Supervisor for BlockchainAutomaton {
    type Index = u64;
    type Seed = ();

    fn leader(&self, _index: Self::Index, _seed: Self::Seed) -> Option<PublicKey> {
        Some(self.signer.public_key())
    }

    fn participants(&self, _index: Self::Index) -> Option<&Vec<PublicKey>> {
        None
    }

    fn is_participant(&self, _index: Self::Index, _candidate: &PublicKey) -> Option<u32> {
        Some(0)
    }

    async fn report(&self, _activity: u8, _proof: Bytes) {}
}