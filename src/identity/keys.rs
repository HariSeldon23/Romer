use commonware_cryptography::{Ed25519, PrivateKey, PublicKey, Scheme};
use rand::rngs::OsRng;
use std::fs;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum KeyManagementError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Cryptography error: {0}")]
    Crypto(String),
}

pub struct NodeKeyManager {
    key_path: PathBuf,
}

impl NodeKeyManager {
    pub fn new() -> Result<Self, KeyManagementError> {
        // Determine the key storage directory
        let key_dir = if let Ok(dir) = std::env::var("ROMER_HOME") {
            PathBuf::from(dir)
        } else {
            // Fallback to Windows user profile
            let user_profile = std::env::var("USERPROFILE")
                .map_err(|_| KeyManagementError::Io(std::io::Error::new(
                    std::io::ErrorKind::NotFound, 
                    "Could not find user profile directory"
                )))?;
            PathBuf::from(user_profile).join(".romer")
        };

        // Ensure the directory exists
        fs::create_dir_all(&key_dir)?;

        // Set the full path for the key file
        let key_path = key_dir.join("node.key");

        Ok(Self { key_path })
    }

    pub fn generate_key(&self) -> Result<Ed25519, KeyManagementError> {
        // Generate a new key
        let signer = Ed25519::new(&mut OsRng);
        
        // Save the key
        self.save_key(&signer)?;
        
        Ok(signer)
    }

    fn save_key(&self, signer: &Ed25519) -> Result<(), KeyManagementError> {
        // Get the private key bytes
        let private_key_bytes = signer.private_key();
        
        // Write to file
        fs::write(&self.key_path, private_key_bytes)
            .map_err(|e| KeyManagementError::Io(e))
    }

    pub fn key_path(&self) -> &PathBuf {
        &self.key_path
    }
}