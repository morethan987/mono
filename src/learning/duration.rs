//! Bayesian duration prediction for tasks
//!
//! Tracks task execution times and provides mean and variance estimates
//! using a Normal-Inverse-Gamma prior or equivalent.

use serde::{Deserialize, Serialize};

/// Stats for a specific task type used for Bayesian duration prediction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DurationStats {
    /// Sum of actual execution times in minutes
    pub sum_x: f64,
    /// Sum of squared actual execution times
    pub sum_x_sq: f64,
    /// Number of observations
    pub count: u32,
}

impl DurationStats {
    pub fn new() -> Self {
        Self {
            sum_x: 0.0,
            sum_x_sq: 0.0,
            count: 0,
        }
    }

    pub fn update(&mut self, actual_minutes: u32) {
        let x = actual_minutes as f64;
        self.sum_x += x;
        self.sum_x_sq += x * x;
        self.count += 1;
    }

    /// Apply time-based decay to duration stats
    pub fn apply_decay(&mut self, factor: f64) {
        self.sum_x *= factor;
        self.sum_x_sq *= factor;
        self.count = (self.count as f64 * factor).round() as u32;
    }

    /// Calculate sample mean and variance
    /// Returns (mean, variance)
    pub fn estimate(&self) -> (f64, f64) {
        if self.count == 0 {
            return (30.0, 100.0); // Default 30 mins, high variance
        }

        let n = self.count as f64;
        let mean = self.sum_x / n;

        // Variance formula: (sum_x_sq / n) - (mean^2)
        let variance = (self.sum_x_sq / n) - (mean * mean);
        let variance = variance.max(1.0); // Ensure non-zero variance

        (mean, variance)
    }
}

pub struct BayesianDurationPredictor;

impl BayesianDurationPredictor {
    /// Predict task duration mean and variance
    pub fn predict(stats: &DurationStats) -> (f64, f64) {
        stats.estimate()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_duration_stats_update() {
        let mut stats = DurationStats::new();
        stats.update(30);
        stats.update(40);
        stats.update(50);

        let (mean, variance) = stats.estimate();
        assert_eq!(mean, 40.0);
        // Variance: (30^2 + 40^2 + 50^2)/3 - 40^2 = (900+1600+2500)/3 - 1600 = 5000/3 - 1600 = 1666.67 - 1600 = 66.67
        assert!((variance - 66.67).abs() < 0.01);
    }

    #[test]
    fn test_cold_start() {
        let stats = DurationStats::new();
        let (mean, variance) = stats.estimate();
        assert_eq!(mean, 30.0);
        assert_eq!(variance, 100.0);
    }
}
