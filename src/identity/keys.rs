use commonware_cryptography::{Ed25519, PrivateKey, Scheme};
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
            let user_profile = std::env::var("USERPROFILE").map_err(|_| {
                KeyManagementError::Io(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "Could not find user profile directory",
                ))
            })?;
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
        fs::write(&self.key_path, private_key_bytes).map_err(|e| KeyManagementError::Io(e))
    }

    pub fn check_existing_key(&self) -> Result<Option<Ed25519>, KeyManagementError> {
        // Check if key file exists
        if !self.key_path.exists() {
            return Ok(None);
        }

        // Read the entire file contents
        let key_bytes = std::fs::read(&self.key_path).map_err(|e| KeyManagementError::Io(e))?;

        // Validate key bytes
        if key_bytes.is_empty() {
            return Err(KeyManagementError::Crypto("Empty key file".to_string()));
        }

        // Create the private key directly from the owned Vec<u8>
        let private_key = PrivateKey::try_from(key_bytes)
            .map_err(|e| KeyManagementError::Crypto(format!("Invalid key format: {}", e)))?;

        // Reconstruct the signer
        <Ed25519 as Scheme>::from(private_key)
            .ok_or_else(|| KeyManagementError::Crypto("Failed to reconstruct key".to_string()))
            .map(Some)
    }

    pub fn key_path(&self) -> &PathBuf {
        &self.key_path
    }
}
