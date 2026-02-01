//! Online learning module for task scheduling optimization.
//!
//! This module implements incremental learning algorithms that adapt to user behavior:
//!
//! - **FTRL (Follow The Regularized Leader)**: Online learning for success prediction
//! - **Multi-Armed Bandit**: Time slot optimization with Thompson Sampling
//! - **Feature Engineering**: Extract ML features from tasks and context
//! - **Reward System**: Compute learning signals from user feedback

mod bandit;
mod features;
mod ftrl;
mod manager;
mod reward;

pub use bandit::{ArmStats, TimeSlotArm, TimeSlotBandit};
pub use features::{FeatureExtractor, FeatureVector};
pub use ftrl::FtrlModel;
pub use manager::{
    GlobalLearningModel, LearningManager, LearningManagerState, TaskTypeLearningModel,
};
pub use reward::{compute_reward, RewardSignal};
