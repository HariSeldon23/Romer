// address.rs
use commonware_cryptography::{PublicKey};
use serde::{Serialize, Deserialize};
use sha2::{Sha256, Digest};
use thiserror::Error;
use crate::network::NetworkParameters;

#[derive(Error, Debug)]
pub enum AddressError {
    #[error("Invalid address format")]
    InvalidFormat,
    #[error("Invalid checksum")]
    InvalidChecksum,
    #[error("Wrong network ID: expected {expected:02x}, got {actual:02x}")]
    WrongNetwork { expected: u8, actual: u8 },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Address {
    // The 20-byte address data derived from public key
    data: [u8; 20],
}

impl Address {
    // Create a new address from a public key
    pub fn from_public_key(public_key: &PublicKey) -> Self {
        // We use SHA-256 of the public key and take the last 20 bytes
        let mut hasher = Sha256::new();
        hasher.update(public_key);
        let hash = hasher.finalize();
        
        let mut data = [0u8; 20];
        data.copy_from_slice(&hash[12..32]);
        
        Self { data }
    }

    // Convert address to Base58Check string for the given network
    pub fn to_string(&self, network: &NetworkParameters) -> String {
        // Create payload: network_id + address_data
        let mut payload = vec![network.network_id()];
        payload.extend_from_slice(&self.data);
        
        // Calculate double SHA256 checksum
        let mut hasher = Sha256::new();
        hasher.update(&payload);
        let hash1 = hasher.finalize();
        
        let mut hasher = Sha256::new();
        hasher.update(hash1);
        let hash2 = hasher.finalize();
        
        // Add 4-byte checksum to payload
        payload.extend_from_slice(&hash2[0..4]);
        
        // Encode full payload in Base58
        bs58::encode(payload).into_string()
    }

    // Parse address from string for the given network
    pub fn from_string(s: &str, network: &NetworkParameters) -> Result<Self, AddressError> {
        // Decode from Base58
        let decoded = bs58::decode(s)
            .into_vec()
            .map_err(|_| AddressError::InvalidFormat)?;
        
        // Check minimum length (1 network byte + 20 address bytes + 4 checksum bytes)
        if decoded.len() != 25 {
            return Err(AddressError::InvalidFormat);
        }
        
        // Verify network ID
        let received_network_id = decoded[0];
        if received_network_id != network.network_id() {
            return Err(AddressError::WrongNetwork {
                expected: network.network_id(),
                actual: received_network_id,
            });
        }
        
        // Verify checksum
        let payload = &decoded[0..21];
        let received_checksum = &decoded[21..25];
        
        let mut hasher = Sha256::new();
        hasher.update(payload);
        let hash1 = hasher.finalize();
        
        let mut hasher = Sha256::new();
        hasher.update(hash1);
        let hash2 = hasher.finalize();
        
        if received_checksum != &hash2[0..4] {
            return Err(AddressError::InvalidChecksum);
        }
        
        // Extract address data
        let mut data = [0u8; 20];
        data.copy_from_slice(&decoded[1..21]);
        
        Ok(Self { data })
    }

    // Get raw address bytes
    pub fn as_bytes(&self) -> &[u8; 20] {
        &self.data
    }
}

// Implement Display using mainnet by default
impl std::fmt::Display for Address {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string(&NetworkParameters::mainnet()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use commonware_cryptography::{Ed25519, Scheme};

    #[test]
    fn test_address_roundtrip() {
        let network = NetworkParameters::mainnet();
        let signer = Ed25519::from_seed(42);
        let addr = Address::from_public_key(&signer.public_key());
        let encoded = addr.to_string(&network);
        let decoded = Address::from_string(&encoded, &network).unwrap();
        assert_eq!(addr, decoded);
    }

    #[test]
    fn test_wrong_network() {
        let mainnet = NetworkParameters::mainnet();
        let testnet = NetworkParameters::testnet();
        
        let signer = Ed25519::from_seed(42);
        let addr = Address::from_public_key(&signer.public_key());
        let encoded = addr.to_string(&mainnet);
        
        // Try to decode mainnet address as testnet
        assert!(matches!(
            Address::from_string(&encoded, &testnet),
            Err(AddressError::WrongNetwork { .. })
        ));
    }
}