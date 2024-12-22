use serde::{Serialize, Deserialize};
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;
use std::collections::HashMap;

/// Represents different types of token allocations in the system
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AllocationCategory {
    /// Network activity rewards (65% of total)
    NetworkActivity,
    /// Ecosystem development fund (10% of total)
    EcosystemDevelopment,
    /// Developer fund (7% of total)
    Developer,
    /// Infrastructure fund (5% of total)
    Infrastructure,
    /// Research and development fund (4% of total)
    ResearchAndDevelopment,
    /// Strategic partnership fund (4% of total)
    StrategicPartnership,
    /// Faucet distribution (5% of total)
    Faucet,
}

/// Represents the vesting schedule for a particular allocation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VestingSchedule {
    /// Start time of the vesting period
    pub start_time: u64,
    /// Duration of the vesting period in seconds
    pub duration: u64,
    /// Duration of the cliff period in seconds (if any)
    pub cliff_duration: Option<u64>,
    /// Whether tokens are released linearly or at specific milestones
    pub release_type: ReleaseType,
    /// Total amount to be vested
    pub total_amount: u64,
    /// Amount already released
    pub released_amount: u64,
}

/// Defines how tokens are released during vesting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReleaseType {
    /// Tokens are released continuously over time
    Linear,
    /// Tokens are released at specific milestones
    Milestone(Vec<(u64, u64)>), // (timestamp, amount) pairs
}

impl VestingSchedule {
    /// Creates a new vesting schedule with linear release
    pub fn new_linear(
        start_time: u64,
        duration: u64,
        cliff_duration: Option<u64>,
        total_amount: u64,
    ) -> Self {
        Self {
            start_time,
            duration,
            cliff_duration,
            release_type: ReleaseType::Linear,
            total_amount,
            released_amount: 0,
        }
    }

    /// Creates a new vesting schedule with milestone-based release
    pub fn new_milestone(
        start_time: u64,
        duration: u64,
        milestones: Vec<(u64, u64)>,
        total_amount: u64,
    ) -> Self {
        Self {
            start_time,
            duration,
            cliff_duration: None,
            release_type: ReleaseType::Milestone(milestones),
            total_amount,
            released_amount: 0,
        }
    }

    /// Calculates the amount of tokens that can be released at a given time
    pub fn releasable_amount(&self, current_time: u64) -> u64 {
        if current_time < self.start_time {
            return 0;
        }

        // Check cliff period
        if let Some(cliff_duration) = self.cliff_duration {
            if current_time < self.start_time + cliff_duration {
                return 0;
            }
        }

        match &self.release_type {
            ReleaseType::Linear => {
                let elapsed = current_time.saturating_sub(self.start_time);
                let total_releasable = if elapsed >= self.duration {
                    self.total_amount
                } else {
                    (self.total_amount * elapsed) / self.duration
                };
                total_releasable.saturating_sub(self.released_amount)
            }
            ReleaseType::Milestone(milestones) => {
                let total_releasable = milestones
                    .iter()
                    .filter(|(timestamp, _)| *timestamp <= current_time)
                    .map(|(_, amount)| amount)
                    .sum::<u64>();
                total_releasable.saturating_sub(self.released_amount)
            }
        }
    }
}

/// Manages block rewards and token emission schedules
#[derive(Debug)]
pub struct RewardSchedule {
    /// Starting block number for this schedule
    genesis_block: u64,
    /// Starting timestamp for this schedule
    genesis_time: u64,
    /// Maps block ranges to their reward amounts
    block_rewards: HashMap<(u64, u64), u64>,
    /// Allocation percentages for different categories
    allocation_percentages: HashMap<AllocationCategory, u8>,
    /// Vesting schedules for different allocations
    vesting_schedules: HashMap<AllocationCategory, VestingSchedule>,
}

impl Default for RewardSchedule {
    fn default() -> Self {
        Self::new()
    }
}

impl RewardSchedule {
    /// Creates a new reward schedule with the default Rømer Chain parameters
    pub fn new() -> Self {
        let genesis_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let mut block_rewards = HashMap::new();
        // Year 1: 16 RØMER per block
        block_rewards.insert((0, 31_536_000), 16);
        // Year 2: 8 RØMER per block
        block_rewards.insert((31_536_000, 63_072_000), 8);
        // Years 3-4: 4 RØMER per block
        block_rewards.insert((63_072_000, 126_144_000), 4);

        let mut allocation_percentages = HashMap::new();
        allocation_percentages.insert(AllocationCategory::NetworkActivity, 65);
        allocation_percentages.insert(AllocationCategory::EcosystemDevelopment, 10);
        allocation_percentages.insert(AllocationCategory::Developer, 7);
        allocation_percentages.insert(AllocationCategory::Infrastructure, 5);
        allocation_percentages.insert(AllocationCategory::ResearchAndDevelopment, 4);
        allocation_percentages.insert(AllocationCategory::StrategicPartnership, 4);
        allocation_percentages.insert(AllocationCategory::Faucet, 5);

        let mut vesting_schedules = HashMap::new();
        
        // Developer fund: 3-year linear vesting, no cliff
        vesting_schedules.insert(
            AllocationCategory::Developer,
            VestingSchedule::new_linear(
                genesis_time,
                94_608_000, // 3 years
                None,
                70_560_000_000, // 70.56M tokens
            ),
        );

        // Infrastructure fund: 4-year vesting with 3-month cliff
        vesting_schedules.insert(
            AllocationCategory::Infrastructure,
            VestingSchedule::new_linear(
                genesis_time,
                126_144_000, // 4 years
                Some(7_776_000), // 3 months
                50_400_000_000, // 50.4M tokens
            ),
        );

        // Research & Development fund: 3-year vesting with 6-month cliff
        vesting_schedules.insert(
            AllocationCategory::ResearchAndDevelopment,
            VestingSchedule::new_linear(
                genesis_time,
                94_608_000, // 3 years
                Some(15_552_000), // 6 months
                40_320_000_000, // 40.32M tokens
            ),
        );

        // Strategic Partnership fund: 2-year vesting with 3-month cliff
        vesting_schedules.insert(
            AllocationCategory::StrategicPartnership,
            VestingSchedule::new_linear(
                genesis_time,
                63_072_000, // 2 years
                Some(7_776_000), // 3 months
                40_320_000_000, // 40.32M tokens
            ),
        );

        Self {
            genesis_block: 0,
            genesis_time,
            block_rewards,
            allocation_percentages,
            vesting_schedules,
        }
    }

    /// Calculates the block reward for a given block number
    pub fn calculate_block_reward(&self, block_number: u64) -> u64 {
        let block_time = block_number; // 1 second block time
        for ((start, end), reward) in &self.block_rewards {
            if block_time >= *start && block_time < *end {
                return *reward;
            }
        }
        0 // Default to 0 after emission schedule ends
    }

    /// Calculates the allocation amount for a specific category from a block reward
    pub fn calculate_allocation(&self, category: &AllocationCategory, block_reward: u64) -> u64 {
        if let Some(percentage) = self.allocation_percentages.get(category) {
            (block_reward * *percentage as u64) / 100
        } else {
            0
        }
    }

    /// Returns the vesting schedule for a given allocation category
    pub fn get_vesting_schedule(&self, category: &AllocationCategory) -> Option<&VestingSchedule> {
        self.vesting_schedules.get(category)
    }

    /// Calculates total tokens emitted up to a given block
    pub fn calculate_total_emission(&self, block_number: u64) -> u64 {
        let mut total = 0;
        for block in 0..=block_number {
            total += self.calculate_block_reward(block);
        }
        total
    }
}

/// Errors that can occur during reward operations
#[derive(Debug, Error)]
pub enum RewardError {
    #[error("Invalid block number: {0}")]
    InvalidBlockNumber(u64),
    
    #[error("Invalid allocation category")]
    InvalidAllocationCategory,
    
    #[error("Invalid vesting schedule")]
    InvalidVestingSchedule,
    
    #[error("Calculation overflow")]
    Overflow,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_block_rewards() {
        let schedule = RewardSchedule::new();
        
        // Test Year 1 rewards
        assert_eq!(schedule.calculate_block_reward(0), 16);
        assert_eq!(schedule.calculate_block_reward(31_535_999), 16);
        
        // Test Year 2 rewards
        assert_eq!(schedule.calculate_block_reward(31_536_000), 8);
        assert_eq!(schedule.calculate_block_reward(63_071_999), 8);
        
        // Test Year 3-4 rewards
        assert_eq!(schedule.calculate_block_reward(63_072_000), 4);
        assert_eq!(schedule.calculate_block_reward(126_143_999), 4);
        
        // Test post-emission
        assert_eq!(schedule.calculate_block_reward(126_144_000), 0);
    }

    #[test]
    fn test_vesting_schedules() {
        let schedule = RewardSchedule::new();
        let dev_schedule = schedule.get_vesting_schedule(&AllocationCategory::Developer).unwrap();
        
        // Test before start
        assert_eq!(dev_schedule.releasable_amount(dev_schedule.start_time - 1), 0);
        
        // Test linear vesting
        let half_time = dev_schedule.start_time + (dev_schedule.duration / 2);
        let half_amount = dev_schedule.releasable_amount(half_time);
        assert!(half_amount > 0 && half_amount < dev_schedule.total_amount);
        
        // Test full vesting
        let end_time = dev_schedule.start_time + dev_schedule.duration;
        assert_eq!(dev_schedule.releasable_amount(end_time), dev_schedule.total_amount);
    }
}