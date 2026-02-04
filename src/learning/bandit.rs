use rand_distr::{Beta, Distribution};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TimeSlotArm {
    Morning,
    Afternoon,
    Evening,
    Night,
}

impl fmt::Display for TimeSlotArm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TimeSlotArm::Morning => write!(f, "上午 (6-12点)"),
            TimeSlotArm::Afternoon => write!(f, "下午 (12-18点)"),
            TimeSlotArm::Evening => write!(f, "傍晚 (18-22点)"),
            TimeSlotArm::Night => write!(f, "夜间 (22-6点)"),
        }
    }
}

impl TimeSlotArm {
    pub const ALL: [TimeSlotArm; 4] = [
        TimeSlotArm::Morning,
        TimeSlotArm::Afternoon,
        TimeSlotArm::Evening,
        TimeSlotArm::Night,
    ];

    pub fn from_hour(hour: u32) -> Self {
        match hour {
            6..=11 => TimeSlotArm::Morning,
            12..=17 => TimeSlotArm::Afternoon,
            18..=21 => TimeSlotArm::Evening,
            _ => TimeSlotArm::Night,
        }
    }

    pub fn as_index(&self) -> usize {
        match self {
            TimeSlotArm::Morning => 0,
            TimeSlotArm::Afternoon => 1,
            TimeSlotArm::Evening => 2,
            TimeSlotArm::Night => 3,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArmStats {
    pub successes: f64,
    pub failures: f64,
}

impl Default for ArmStats {
    fn default() -> Self {
        Self {
            successes: 1.0,
            failures: 1.0,
        }
    }
}

impl ArmStats {
    pub fn success_rate(&self) -> f64 {
        self.successes / (self.successes + self.failures)
    }

    pub fn total_trials(&self) -> f64 {
        self.successes + self.failures - 2.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSlotBandit {
    arms: [ArmStats; 4],
    pub total_selections: u64,
}

impl Default for TimeSlotBandit {
    fn default() -> Self {
        Self::new()
    }
}

impl TimeSlotBandit {
    pub fn new() -> Self {
        Self {
            arms: [
                ArmStats::default(),
                ArmStats::default(),
                ArmStats::default(),
                ArmStats::default(),
            ],
            total_selections: 0,
        }
    }

    pub fn select_arm(&self) -> TimeSlotArm {
        let mut rng = rand::rng();
        let mut best_arm = TimeSlotArm::Morning;
        let mut best_sample = f64::NEG_INFINITY;

        for arm in TimeSlotArm::ALL {
            let stats = &self.arms[arm.as_index()];
            let beta =
                Beta::new(stats.successes, stats.failures).unwrap_or(Beta::new(1.0, 1.0).unwrap());
            let sample = beta.sample(&mut rng);

            if sample > best_sample {
                best_sample = sample;
                best_arm = arm;
            }
        }

        best_arm
    }

    pub fn select_arm_greedy(&self) -> TimeSlotArm {
        let mut best_arm = TimeSlotArm::Morning;
        let mut best_rate = f64::NEG_INFINITY;

        for arm in TimeSlotArm::ALL {
            let rate = self.arms[arm.as_index()].success_rate();
            if rate > best_rate {
                best_rate = rate;
                best_arm = arm;
            }
        }

        best_arm
    }

    pub fn update(&mut self, arm: TimeSlotArm, success: bool) {
        let stats = &mut self.arms[arm.as_index()];
        if success {
            stats.successes += 1.0;
        } else {
            stats.failures += 1.0;
        }
        self.total_selections += 1;
    }

    /// Apply time-based decay to bandit stats
    pub fn apply_decay(&mut self, factor: f64) {
        for arm in &mut self.arms {
            // Keep at least 1.0 to avoid Beta distribution issues
            arm.successes = (arm.successes * factor).max(1.0);
            arm.failures = (arm.failures * factor).max(1.0);
        }
    }

    pub fn get_stats(&self, arm: TimeSlotArm) -> &ArmStats {
        &self.arms[arm.as_index()]
    }

    pub fn success_rate(&self, arm: TimeSlotArm) -> f64 {
        self.arms[arm.as_index()].success_rate()
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arm_from_hour() {
        assert_eq!(TimeSlotArm::from_hour(8), TimeSlotArm::Morning);
        assert_eq!(TimeSlotArm::from_hour(14), TimeSlotArm::Afternoon);
        assert_eq!(TimeSlotArm::from_hour(19), TimeSlotArm::Evening);
        assert_eq!(TimeSlotArm::from_hour(23), TimeSlotArm::Night);
        assert_eq!(TimeSlotArm::from_hour(3), TimeSlotArm::Night);
    }

    #[test]
    fn test_bandit_learning() {
        let mut bandit = TimeSlotBandit::new();

        for _ in 0..50 {
            bandit.update(TimeSlotArm::Morning, true);
            bandit.update(TimeSlotArm::Afternoon, false);
        }

        assert!(bandit.success_rate(TimeSlotArm::Morning) > 0.8);
        assert!(bandit.success_rate(TimeSlotArm::Afternoon) < 0.2);

        let best = bandit.select_arm_greedy();
        assert_eq!(best, TimeSlotArm::Morning);
    }

    #[test]
    fn test_thompson_sampling_exploration() {
        let bandit = TimeSlotBandit::new();

        let mut selections = [0u32; 4];
        for _ in 0..100 {
            let arm = bandit.select_arm();
            selections[arm.as_index()] += 1;
        }

        for count in selections {
            assert!(count > 0, "All arms should be explored with uniform priors");
        }
    }

    #[test]
    fn test_serialization() {
        let mut bandit = TimeSlotBandit::new();
        bandit.update(TimeSlotArm::Morning, true);
        bandit.update(TimeSlotArm::Morning, true);

        let json = bandit.to_json().unwrap();
        let loaded = TimeSlotBandit::from_json(&json).unwrap();

        assert_eq!(
            bandit.success_rate(TimeSlotArm::Morning),
            loaded.success_rate(TimeSlotArm::Morning)
        );
    }
}
