use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;

/// Represents the category of a network region
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RegionCategory {
    /// Major internet exchange points
    MajorInternetExchange,
    /// Submarine cable landing stations
    SubmarineCableLanding,
    /// Strategic terrestrial crossing points
    StrategicTerrestrialCrossing,
    /// Regional internet exchange points
    RegionalInternetExchange,
    /// Emerging strategic points
    EmergingStrategicPoint,
}

/// Represents a jurisdiction where a region operates
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Jurisdiction {
    /// Primary country or region
    pub primary: String,
    /// Optional subdivision (state/province)
    pub subdivision: Option<String>,
}

impl Jurisdiction {
    /// Creates a new jurisdiction with only a primary region
    pub fn new(primary: impl Into<String>) -> Self {
        Self {
            primary: primary.into(),
            subdivision: None,
        }
    }

    /// Creates a new jurisdiction with both primary and subdivision
    pub fn with_subdivision(primary: impl Into<String>, subdivision: impl Into<String>) -> Self {
        Self {
            primary: primary.into(),
            subdivision: Some(subdivision.into()),
        }
    }
}

/// Represents a network region with its associated metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Region {
    /// Name of the city where the region is located
    pub city: String,
    /// Category of the network region
    pub category: RegionCategory,
    /// Legal jurisdiction of the region
    pub jurisdiction: Jurisdiction,
    /// Geographic coordinates (latitude, longitude)
    pub coordinates: Option<(f64, f64)>,
    /// Additional metadata about the region
    pub metadata: HashMap<String, String>,
}

impl Region {
    /// Creates a new Region instance
    pub fn new(
        city: impl Into<String>,
        category: RegionCategory,
        jurisdiction: Jurisdiction,
    ) -> Self {
        Self {
            city: city.into(),
            category,
            jurisdiction,
            coordinates: None,
            metadata: HashMap::new(),
        }
    }

    /// Adds geographic coordinates to the region
    pub fn with_coordinates(mut self, latitude: f64, longitude: f64) -> Self {
        self.coordinates = Some((latitude, longitude));
        self
    }

    /// Adds a metadata key-value pair to the region
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

/// Registry managing all network regions
#[derive(Clone)]
pub struct RegionRegistry {
    regions: Arc<Vec<Region>>,
}

impl Default for RegionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl RegionRegistry {
    /// Creates a new RegionRegistry with predefined regions
    pub fn new() -> Self {
        let regions = vec![
            // Major Internet Exchanges
            Region::new(
                "Frankfurt",
                RegionCategory::MajorInternetExchange,
                Jurisdiction::with_subdivision("European Union", "Germany"),
            )
            .with_metadata("exchange", "DE-CIX"),
            Region::new(
                "Amsterdam",
                RegionCategory::MajorInternetExchange,
                Jurisdiction::with_subdivision("European Union", "Netherlands"),
            )
            .with_metadata("exchange", "AMS-IX"),
            Region::new(
                "London",
                RegionCategory::MajorInternetExchange,
                Jurisdiction::new("United Kingdom"),
            )
            .with_metadata("exchange", "LINX"),
            // Submarine Cable Landings
            Region::new(
                "Marseille",
                RegionCategory::SubmarineCableLanding,
                Jurisdiction::with_subdivision("European Union", "France"),
            ),
            Region::new(
                "Los Angeles",
                RegionCategory::SubmarineCableLanding,
                Jurisdiction::with_subdivision("United States", "California"),
            ),
            // Strategic Terrestrial Crossings
            Region::new(
                "Stockholm",
                RegionCategory::StrategicTerrestrialCrossing,
                Jurisdiction::with_subdivision("European Union", "Sweden"),
            ),
            Region::new(
                "Beijing",
                RegionCategory::StrategicTerrestrialCrossing,
                Jurisdiction::new("China"),
            ),
            // Regional Internet Exchanges
            Region::new(
                "Paris",
                RegionCategory::RegionalInternetExchange,
                Jurisdiction::with_subdivision("European Union", "France"),
            )
            .with_metadata("exchange", "France-IX"),
            Region::new(
                "Madrid",
                RegionCategory::RegionalInternetExchange,
                Jurisdiction::with_subdivision("European Union", "Spain"),
            )
            .with_metadata("exchange", "ESPANIX"),
            // Emerging Strategic Points
            Region::new(
                "Dublin",
                RegionCategory::EmergingStrategicPoint,
                Jurisdiction::with_subdivision("European Union", "Ireland"),
            ),
            Region::new(
                "Montreal",
                RegionCategory::EmergingStrategicPoint,
                Jurisdiction::with_subdivision("Canada", "Quebec"),
            ),
            // Major Internet Exchanges
            Region::new(
                "Ashburn VA",
                RegionCategory::MajorInternetExchange,
                Jurisdiction::with_subdivision("United States", "Federal"),
            )
            .with_metadata("exchange", "Equinix"),
            Region::new(
                "New York/NJ",
                RegionCategory::MajorInternetExchange,
                Jurisdiction::with_subdivision("United States", "Federal"),
            )
            .with_metadata("exchange", "NYIIX"),
            Region::new(
                "Tokyo",
                RegionCategory::MajorInternetExchange,
                Jurisdiction::new("Japan"),
            )
            .with_metadata("exchange", "JPNAP"),
            Region::new(
                "Singapore",
                RegionCategory::MajorInternetExchange,
                Jurisdiction::new("Singapore"),
            )
            .with_metadata("exchange", "SGIX"),
            Region::new(
                "Hong Kong",
                RegionCategory::MajorInternetExchange,
                Jurisdiction::new("Hong Kong SAR"),
            )
            .with_metadata("exchange", "HKIX"),
            Region::new(
                "Sydney",
                RegionCategory::MajorInternetExchange,
                Jurisdiction::new("Australia"),
            )
            .with_metadata("exchange", "IX Australia"),
            Region::new(
                "São Paulo",
                RegionCategory::MajorInternetExchange,
                Jurisdiction::new("Brazil"),
            )
            .with_metadata("exchange", "IX.br"),
            // Submarine Cable Landings
            Region::new(
                "Seattle",
                RegionCategory::SubmarineCableLanding,
                Jurisdiction::with_subdivision("United States", "Washington"),
            ),
            Region::new(
                "Miami",
                RegionCategory::SubmarineCableLanding,
                Jurisdiction::with_subdivision("United States", "Florida"),
            ),
            Region::new(
                "Toronto",
                RegionCategory::SubmarineCableLanding,
                Jurisdiction::with_subdivision("Canada", "Ontario"),
            ),
            Region::new(
                "Dubai",
                RegionCategory::SubmarineCableLanding,
                Jurisdiction::new("UAE"),
            ),
            Region::new(
                "Mumbai",
                RegionCategory::SubmarineCableLanding,
                Jurisdiction::new("India"),
            ),
            Region::new(
                "Chennai",
                RegionCategory::SubmarineCableLanding,
                Jurisdiction::new("India"),
            ),
            Region::new(
                "Fortaleza",
                RegionCategory::SubmarineCableLanding,
                Jurisdiction::new("Brazil"),
            ),
            Region::new(
                "Manila",
                RegionCategory::SubmarineCableLanding,
                Jurisdiction::new("Philippines"),
            ),
            // Strategic Terrestrial Crossings
            Region::new(
                "Warsaw",
                RegionCategory::StrategicTerrestrialCrossing,
                Jurisdiction::with_subdivision("European Union", "Poland"),
            ),
            Region::new(
                "Istanbul",
                RegionCategory::StrategicTerrestrialCrossing,
                Jurisdiction::new("Turkey"),
            ),
            Region::new(
                "Cairo",
                RegionCategory::StrategicTerrestrialCrossing,
                Jurisdiction::new("Egypt"),
            ),
            Region::new(
                "Moscow",
                RegionCategory::StrategicTerrestrialCrossing,
                Jurisdiction::new("Russia"),
            ),
            Region::new(
                "Seoul",
                RegionCategory::StrategicTerrestrialCrossing,
                Jurisdiction::new("South Korea"),
            ),
            Region::new(
                "Taipei",
                RegionCategory::StrategicTerrestrialCrossing,
                Jurisdiction::new("Taiwan"),
            ),
            Region::new(
                "Jakarta",
                RegionCategory::StrategicTerrestrialCrossing,
                Jurisdiction::new("Indonesia"),
            ),
            Region::new(
                "Auckland",
                RegionCategory::StrategicTerrestrialCrossing,
                Jurisdiction::new("New Zealand"),
            ),
            // Regional Internet Exchanges
            Region::new(
                "Milan",
                RegionCategory::RegionalInternetExchange,
                Jurisdiction::with_subdivision("European Union", "Italy"),
            )
            .with_metadata("exchange", "MIX"),
            Region::new(
                "Vienna",
                RegionCategory::RegionalInternetExchange,
                Jurisdiction::with_subdivision("European Union", "Austria"),
            )
            .with_metadata("exchange", "VIX"),
            Region::new(
                "Prague",
                RegionCategory::RegionalInternetExchange,
                Jurisdiction::with_subdivision("European Union", "Czech Republic"),
            )
            .with_metadata("exchange", "NIX.CZ"),
            Region::new(
                "Copenhagen",
                RegionCategory::RegionalInternetExchange,
                Jurisdiction::with_subdivision("European Union", "Denmark"),
            )
            .with_metadata("exchange", "Netnod"),
            Region::new(
                "Helsinki",
                RegionCategory::RegionalInternetExchange,
                Jurisdiction::with_subdivision("European Union", "Finland"),
            )
            .with_metadata("exchange", "FICIX"),
            Region::new(
                "Tel Aviv",
                RegionCategory::RegionalInternetExchange,
                Jurisdiction::new("Israel"),
            )
            .with_metadata("exchange", "IIX"),
            Region::new(
                "Johannesburg",
                RegionCategory::RegionalInternetExchange,
                Jurisdiction::new("South Africa"),
            )
            .with_metadata("exchange", "NAPAfrica"),
            Region::new(
                "Lagos",
                RegionCategory::RegionalInternetExchange,
                Jurisdiction::new("Nigeria"),
            )
            .with_metadata("exchange", "IXPN"),
            // Emerging Strategic Points
            Region::new(
                "Nairobi",
                RegionCategory::EmergingStrategicPoint,
                Jurisdiction::new("Kenya"),
            ),
            Region::new(
                "Cape Town",
                RegionCategory::EmergingStrategicPoint,
                Jurisdiction::new("South Africa"),
            ),
            Region::new(
                "Panama City",
                RegionCategory::EmergingStrategicPoint,
                Jurisdiction::new("Panama"),
            ),
            Region::new(
                "Santiago",
                RegionCategory::EmergingStrategicPoint,
                Jurisdiction::new("Chile"),
            ),
            Region::new(
                "Vancouver",
                RegionCategory::EmergingStrategicPoint,
                Jurisdiction::with_subdivision("Canada", "British Columbia"),
            ),
            Region::new(
                "Perth",
                RegionCategory::EmergingStrategicPoint,
                Jurisdiction::new("Australia"),
            ),
            Region::new(
                "Kuala Lumpur",
                RegionCategory::EmergingStrategicPoint,
                Jurisdiction::new("Malaysia"),
            ),
            Region::new(
                "Muscat",
                RegionCategory::EmergingStrategicPoint,
                Jurisdiction::new("Oman"),
            ),
        ];

        Self {
            regions: Arc::new(regions),
        }
    }

    /// Returns all regions in a specific category
    pub fn regions_by_category(&self, category: &RegionCategory) -> Vec<&Region> {
        self.regions
            .iter()
            .filter(|r| r.category == *category)
            .collect()
    }

    /// Returns all regions in a specific jurisdiction
    pub fn regions_by_jurisdiction(&self, jurisdiction: &Jurisdiction) -> Vec<&Region> {
        self.regions
            .iter()
            .filter(|r| r.jurisdiction == *jurisdiction)
            .collect()
    }

    /// Returns a region by its city name
    pub fn region_by_city(&self, city: &str) -> Option<&Region> {
        self.regions
            .iter()
            .find(|r| r.city.eq_ignore_ascii_case(city))
    }

    /// Returns all regions
    pub fn all_regions(&self) -> &[Region] {
        &self.regions
    }
}

/// Errors that can occur during region operations
#[derive(Debug, Error)]
pub enum RegionError {
    #[error("Region not found: {0}")]
    RegionNotFound(String),

    #[error("Invalid jurisdiction: {0}")]
    InvalidJurisdiction(String),

    #[error("Invalid coordinates: {lat}, {lon}")]
    InvalidCoordinates { lat: f64, lon: f64 },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_region_creation() {
        let region = Region::new(
            "Frankfurt",
            RegionCategory::MajorInternetExchange,
            Jurisdiction::with_subdivision("European Union", "Germany"),
        );

        assert_eq!(region.city, "Frankfurt");
        assert_eq!(region.category, RegionCategory::MajorInternetExchange);
        assert_eq!(region.jurisdiction.primary, "European Union");
        assert_eq!(region.jurisdiction.subdivision, Some("Germany".to_string()));
    }

    #[test]
    fn test_region_registry() {
        let registry = RegionRegistry::new();

        // Test finding region by city
        let frankfurt = registry.region_by_city("Frankfurt");
        assert!(frankfurt.is_some());

        // Test filtering by category
        let major_exchanges = registry.regions_by_category(&RegionCategory::MajorInternetExchange);
        assert!(!major_exchanges.is_empty());

        // Test filtering by jurisdiction
        let eu_regions = registry
            .regions_by_jurisdiction(&Jurisdiction::with_subdivision("European Union", "Germany"));
        assert!(!eu_regions.is_empty());
    }
}
