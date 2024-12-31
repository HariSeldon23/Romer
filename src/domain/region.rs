use serde::{Serialize, Deserialize};
use strum_macros::{EnumString, Display};

/// Represents the category of internet infrastructure in a city.
/// This helps understand the network connectivity capabilities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, EnumString, Display)]
#[serde(rename_all = "snake_case")]
pub enum NetworkCategory {
    // We'll expand this enum as we add more cities
    RegionalInternetExchange,
}

/// Represents the legal jurisdiction where a validator operates.
/// This is important for understanding regulatory implications.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Jurisdiction {
    pub country: String,
    pub region: String,  // State, province, etc.
}

/// Represents a city where validators can operate.
/// For now, we only support Brisbane but this structure allows
/// easy addition of more cities in the future.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ValidatorCity {
    pub name: String,
    pub category: NetworkCategory,
    pub jurisdiction: Jurisdiction,
    pub is_active: bool,
}

impl ValidatorCity {
    /// Returns the single active validator city (Brisbane)
    pub fn brisbane() -> Self {
        Self {
            name: "Brisbane".to_string(),
            category: NetworkCategory::RegionalInternetExchange,
            jurisdiction: Jurisdiction {
                country: "Australia".to_string(),
                region: "Queensland".to_string(),
            },
            is_active: true,
        }
    }

    /// Returns expected internal latency within this city in milliseconds
    pub fn internal_latency(&self) -> u32 {
        match self.name.as_str() {
            "Brisbane" => 15,
            _ => 50,  // Conservative default for future cities
        }
    }
}

impl Default for ValidatorCity {
    fn default() -> Self {
        Self::brisbane()  // Brisbane is our default city during initial development
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_brisbane_configuration() {
        let brisbane = ValidatorCity::brisbane();
        assert_eq!(brisbane.name, "Brisbane");
        assert_eq!(brisbane.jurisdiction.country, "Australia");
        assert_eq!(brisbane.jurisdiction.region, "Queensland");
        assert!(brisbane.is_active);
    }

    #[test]
    fn test_serialization() {
        let brisbane = ValidatorCity::brisbane();
        let serialized = serde_json::to_string(&brisbane).unwrap();
        let deserialized: ValidatorCity = serde_json::from_str(&serialized).unwrap();
        assert_eq!(brisbane, deserialized);
    }
}