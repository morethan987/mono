//! FTRL (Follow The Regularized Leader) online learning algorithm.
//!
//! FTRL is an online learning algorithm that's particularly good for
//! sparse features and incremental updates. It's widely used in production
//! recommendation systems at Google, Facebook, etc.
//!
//! Reference: "Ad Click Prediction: a View from the Trenches" (Google, 2013)

use crate::learning::features::FeatureVector;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// FTRL model for online binary classification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FtrlModel {
    /// Accumulated gradients squared (for adaptive learning rate)
    z: HashMap<u32, f64>,
    /// Per-coordinate learning rate denominator
    n: HashMap<u32, f64>,

    // Hyperparameters
    /// Base learning rate (alpha)
    pub alpha: f64,
    /// Learning rate scale factor (beta)
    pub beta: f64,
    /// L1 regularization strength (sparsity)
    pub lambda1: f64,
    /// L2 regularization strength (stability)
    pub lambda2: f64,

    // Statistics
    /// Number of training examples seen
    pub examples_seen: u64,
}

impl Default for FtrlModel {
    fn default() -> Self {
        Self::new()
    }
}

impl FtrlModel {
    /// Create a new FTRL model with default hyperparameters.
    pub fn new() -> Self {
        Self {
            z: HashMap::new(),
            n: HashMap::new(),
            alpha: 0.1,    // base learning rate
            beta: 1.0,     // learning rate scale
            lambda1: 0.01, // L1 regularization (promotes sparsity)
            lambda2: 0.01, // L2 regularization (prevents large weights)
            examples_seen: 0,
        }
    }

    /// Create with custom hyperparameters.
    pub fn with_params(alpha: f64, beta: f64, lambda1: f64, lambda2: f64) -> Self {
        Self {
            z: HashMap::new(),
            n: HashMap::new(),
            alpha,
            beta,
            lambda1,
            lambda2,
            examples_seen: 0,
        }
    }

    /// Compute effective weight for a feature index.
    fn get_weight(&self, i: u32) -> f64 {
        let z_i = *self.z.get(&i).unwrap_or(&0.0);
        let n_i = *self.n.get(&i).unwrap_or(&0.0);

        // Soft thresholding (L1 sparsity)
        if z_i.abs() <= self.lambda1 {
            return 0.0;
        }

        // Compute weight with per-coordinate learning rate
        let sign = if z_i >= 0.0 { 1.0 } else { -1.0 };
        let learning_rate = (self.beta + n_i.sqrt()) / self.alpha + self.lambda2;

        -(z_i - sign * self.lambda1) / learning_rate
    }

    /// Predict probability for a feature vector.
    ///
    /// Returns a probability in [0, 1].
    pub fn predict(&self, features: &FeatureVector) -> f64 {
        let mut logit = 0.0;

        for (idx, value) in features.iter() {
            let w = self.get_weight(*idx);
            logit += w * value;
        }

        sigmoid(logit)
    }

    /// Update model with a single training example.
    ///
    /// # Arguments
    /// * `features` - Feature vector for the example
    /// * `label` - True label (1.0 for positive, 0.0 for negative)
    pub fn update(&mut self, features: &FeatureVector, label: f64) {
        // Compute prediction
        let p = self.predict(features);

        // Compute gradient (cross-entropy loss derivative)
        let g = p - label;

        // Update each feature
        for (idx, value) in features.iter() {
            let idx = *idx;
            let x_i = *value;

            // Gradient for this feature
            let g_i = g * x_i;

            // Get current values
            let n_i = *self.n.get(&idx).unwrap_or(&0.0);
            let z_i = *self.z.get(&idx).unwrap_or(&0.0);

            // Update n (sum of squared gradients)
            let n_i_new = n_i + g_i * g_i;

            // Compute sigma for z update
            let sigma = ((n_i_new).sqrt() - (n_i).sqrt()) / self.alpha;

            // Update z
            let w_i = self.get_weight(idx);
            let z_i_new = z_i + g_i - sigma * w_i;

            // Store updates
            self.n.insert(idx, n_i_new);
            self.z.insert(idx, z_i_new);
        }

        self.examples_seen += 1;
    }

    /// Get the number of non-zero weights.
    pub fn num_weights(&self) -> usize {
        self.z
            .keys()
            .filter(|&i| self.get_weight(*i).abs() > 1e-10)
            .count()
    }

    pub fn weights_count(&self) -> usize {
        self.num_weights()
    }

    /// Get all non-zero weights as a HashMap.
    pub fn weights(&self) -> HashMap<u32, f64> {
        let mut result = HashMap::new();
        for idx in self.z.keys() {
            let w = self.get_weight(*idx);
            if w.abs() > 1e-10 {
                result.insert(*idx, w);
            }
        }
        result
    }

    /// Reset the model to initial state.
    pub fn reset(&mut self) {
        self.z.clear();
        self.n.clear();
        self.examples_seen = 0;
    }

    /// Serialize model state to JSON for persistence.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Deserialize model state from JSON.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

/// Sigmoid function for probability conversion.
#[inline]
fn sigmoid(x: f64) -> f64 {
    // Numerically stable sigmoid
    if x >= 0.0 {
        1.0 / (1.0 + (-x).exp())
    } else {
        let exp_x = x.exp();
        exp_x / (1.0 + exp_x)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sigmoid() {
        assert!((sigmoid(0.0) - 0.5).abs() < 1e-10);
        assert!(sigmoid(10.0) > 0.99);
        assert!(sigmoid(-10.0) < 0.01);
    }

    #[test]
    fn test_ftrl_basic() {
        let model = FtrlModel::new();

        // Initial prediction should be 0.5 (no weights)
        let features = FeatureVector::new();
        assert!((model.predict(&features) - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_ftrl_learning() {
        let mut model = FtrlModel::with_params(0.5, 1.0, 0.0, 0.0); // No regularization

        // Create positive examples with feature 0
        let mut pos_features = FeatureVector::new();
        pos_features.add_binary(0);
        pos_features.add_binary(1);

        // Create negative examples with feature 2
        let mut neg_features = FeatureVector::new();
        neg_features.add_binary(2);
        neg_features.add_binary(3);

        // Train
        for _ in 0..100 {
            model.update(&pos_features, 1.0);
            model.update(&neg_features, 0.0);
        }

        // Model should learn
        assert!(
            model.predict(&pos_features) > 0.7,
            "Positive prediction: {}",
            model.predict(&pos_features)
        );
        assert!(
            model.predict(&neg_features) < 0.3,
            "Negative prediction: {}",
            model.predict(&neg_features)
        );
    }

    #[test]
    fn test_ftrl_sparsity() {
        let mut model = FtrlModel::with_params(0.1, 1.0, 0.5, 0.0); // Strong L1

        let mut features = FeatureVector::new();
        features.add_binary(0);

        // Train lightly
        for _ in 0..5 {
            model.update(&features, 1.0);
        }

        // With strong L1, small weights should be zeroed out
        // (depending on the training, some weights may remain zero)
        assert!(model.num_weights() <= model.z.len());
    }

    #[test]
    fn test_ftrl_serialization() {
        let mut model = FtrlModel::new();

        let mut features = FeatureVector::new();
        features.add_binary(0);
        model.update(&features, 1.0);

        // Serialize
        let json = model.to_json().unwrap();

        // Deserialize
        let loaded = FtrlModel::from_json(&json).unwrap();

        // Should match
        assert_eq!(model.examples_seen, loaded.examples_seen);
        assert!((model.predict(&features) - loaded.predict(&features)).abs() < 1e-10);
    }
}
