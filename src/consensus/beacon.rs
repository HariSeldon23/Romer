use commonware_consensus::Supervisor;
use commonware_cryptography::Ed25519;
use bytes::Bytes;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use thiserror::Error;

/// Handles leader election based on region rotation.
/// This is a simple implementation that moves through regions in a round-robin fashion,
/// skipping regions that have no active validators.
#[derive(Clone)]
pub struct BeaconConsensus {
    /// Maps regions to their active validators
    validators_by_region: Arc<Mutex<HashMap<String, Vec<Ed25519>>>>,
    /// List of regions in order of rotation
    regions: Vec<String>,
    /// Current region index for round-robin selection
    current_region_idx: Arc<Mutex<usize>>,
}

impl BeaconConsensus {
    /// Creates a new BeaconConsensus instance with the given list of regions.
    /// The order of regions in the list determines their rotation order.
    pub fn new(regions: Vec<String>) -> Self {
        Self {
            validators_by_region: Arc::new(Mutex::new(HashMap::new())),
            regions,
            current_region_idx: Arc::new(Mutex::new(0)),
        }
    }

    /// Registers a validator for a specific region.
    /// This makes the validator eligible for leader selection in that region.
    pub fn register_validator(&self, region: String, validator: Ed25519) -> Result<(), BeaconError> {
        // Verify the region is valid
        if !self.regions.contains(&region) {
            return Err(BeaconError::InvalidRegion(region));
        }

        let mut validators = self.validators_by_region.lock().map_err(|_| BeaconError::LockError)?;
        validators.entry(region).or_insert_with(Vec::new).push(validator);
        Ok(())
    }

    /// Removes a validator from a region.
    /// The validator will no longer be considered for leader selection.
    pub fn remove_validator(&self, region: &str, validator_key: &[u8]) -> Result<(), BeaconError> {
        let mut validators = self.validators_by_region.lock().map_err(|_| BeaconError::LockError)?;
        
        if let Some(region_validators) = validators.get_mut(region) {
            region_validators.retain(|v| v.public_key() != validator_key);
        }
        
        Ok(())
    }

    /// Gets the next region in round-robin order that has active validators.
    /// Returns None if no regions have validators.
    fn next_active_region(&self) -> Result<Option<String>, BeaconError> {
        let validators = self.validators_by_region.lock().map_err(|_| BeaconError::LockError)?;
        let mut idx = self.current_region_idx.lock().map_err(|_| BeaconError::LockError)?;
        
        // Check each region in order until we find one with validators
        for _ in 0..self.regions.len() {
            let region = &self.regions[*idx];
            *idx = (*idx + 1) % self.regions.len();
            
            if let Some(region_validators) = validators.get(region) {
                if !region_validators.is_empty() {
                    return Ok(Some(region.clone()));
                }
            }
        }
        
        Ok(None)
    }

    /// Gets the validators for a specific region
    pub fn get_region_validators(&self, region: &str) -> Result<Vec<Ed25519>, BeaconError> {
        let validators = self.validators_by_region.lock().map_err(|_| BeaconError::LockError)?;
        Ok(validators.get(region).cloned().unwrap_or_default())
    }

    /// Gets all currently registered validators across all regions
    pub fn get_all_validators(&self) -> Result<Vec<Ed25519>, BeaconError> {
        let validators = self.validators_by_region.lock().map_err(|_| BeaconError::LockError)?;
        Ok(validators.values().flat_map(|v| v.iter().cloned()).collect())
    }
}

impl Supervisor for BeaconConsensus {
    type Index = u64;  // View number
    type Seed = ();    // We don't need additional seed data

    fn leader(&self, view: u64, _seed: ()) -> Option<Bytes> {
        // Get the next active region
        let region = self.next_active_region().ok()??;
        
        // Get validators for this region
        let validators = match self.get_region_validators(&region) {
            Ok(v) => v,
            Err(_) => return None,
        };
        
        if validators.is_empty() {
            return None;
        }

        // Select validator within region based on view number
        let validator_idx = (view as usize) % validators.len();
        let leader = &validators[validator_idx];
        
        Some(Bytes::copy_from_slice(&leader.public_key()))
    }

    fn participants(&self, _view: u64) -> Option<Vec<Bytes>> {
        // Get all registered validators
        let all_validators = match self.get_all_validators() {
            Ok(validators) => validators,
            Err(_) => return None,
        };

        if all_validators.is_empty() {
            None
        } else {
            Some(all_validators
                .iter()
                .map(|v| Bytes::copy_from_slice(&v.public_key()))
                .collect())
        }
    }

    fn is_participant(&self, _view: u64, candidate: &Bytes) -> Option<u32> {
        // Get all validators and find the candidate's position
        let all_validators = match self.get_all_validators() {
            Ok(validators) => validators,
            Err(_) => return None,
        };

        for (position, validator) in all_validators.iter().enumerate() {
            if Bytes::copy_from_slice(&validator.public_key()) == *candidate {
                return Some(position as u32);
            }
        }
        
        None
    }
}

/// Errors that can occur during beacon operations
#[derive(Debug, Error)]
pub enum BeaconError {
    #[error("Invalid region specified: {0}")]
    InvalidRegion(String),

    #[error("Failed to acquire lock")]
    LockError,

    #[error("No active validators in region")]
    NoValidators,

    #[error("Invalid validator")]
    InvalidValidator,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_test_beacon() -> BeaconConsensus {
        BeaconConsensus::new(vec![
            "Frankfurt".to_string(),
            "London".to_string(),
            "Amsterdam".to_string(),
        ])
    }

    #[test]
    fn test_validator_registration() {
        let beacon = setup_test_beacon();
        let validator = Ed25519::generate();

        // Test valid registration
        assert!(beacon.register_validator("Frankfurt".to_string(), validator.clone()).is_ok());

        // Test invalid region
        assert!(beacon.register_validator("Invalid".to_string(), validator).is_err());
    }

    #[test]
    fn test_region_rotation() {
        let beacon = setup_test_beacon();
        
        // Register validators in different regions
        let validator1 = Ed25519::generate();
        let validator2 = Ed25519::generate();
        let validator3 = Ed25519::generate();

        beacon.register_validator("Frankfurt".to_string(), validator1).unwrap();
        beacon.register_validator("London".to_string(), validator2).unwrap();
        beacon.register_validator("Amsterdam".to_string(), validator3).unwrap();

        // Check leader rotation
        let leader1 = beacon.leader(0, ());
        let leader2 = beacon.leader(1, ());
        let leader3 = beacon.leader(2, ());

        assert!(leader1.is_some());
        assert!(leader2.is_some());
        assert!(leader3.is_some());
        
        // Leaders should be different as we rotate through regions
        assert_ne!(leader1, leader2);
        assert_ne!(leader2, leader3);
    }

    #[test]
    fn test_empty_region_skipping() {
        let beacon = setup_test_beacon();
        
        // Only register validators in Frankfurt and Amsterdam
        let validator1 = Ed25519::generate();
        let validator2 = Ed25519::generate();
        
        beacon.register_validator("Frankfurt".to_string(), validator1).unwrap();
        beacon.register_validator("Amsterdam".to_string(), validator2).unwrap();

        // Check that we skip the empty London region
        let leader1 = beacon.leader(0, ());
        let leader2 = beacon.leader(1, ());
        
        assert!(leader1.is_some());
        assert!(leader2.is_some());
        assert_ne!(leader1, leader2);
    }

    #[test]
    fn test_validator_removal() {
        let beacon = setup_test_beacon();
        let validator = Ed25519::generate();
        let region = "Frankfurt".to_string();

        // Register and then remove a validator
        beacon.register_validator(region.clone(), validator.clone()).unwrap();
        assert!(beacon.remove_validator(&region, &validator.public_key()).is_ok());

        // Region should now be empty
        assert!(beacon.get_region_validators(&region).unwrap().is_empty());
    }
}