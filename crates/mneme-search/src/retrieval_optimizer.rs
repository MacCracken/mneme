//! Retrieval optimizer — Thompson Sampling over RRF blend weights.
//!
//! Learns optimal weights for combining full-text, semantic, and recency
//! signals by tracking which search results users actually engage with.

use serde::{Deserialize, Serialize};

/// Blend weights applied during hybrid search.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct BlendWeights {
    /// Weight for full-text (BM25) RRF scores.
    pub fulltext: f64,
    /// Weight for semantic (vector) RRF scores.
    pub semantic: f64,
    /// Additive boost per recency tier (0 = no boost).
    pub recency: f64,
}

impl Default for BlendWeights {
    fn default() -> Self {
        Self {
            fulltext: 1.0,
            semantic: 1.0,
            recency: 0.0,
        }
    }
}

/// A single arm in the Thompson Sampling bandit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Arm {
    pub name: String,
    pub weights: BlendWeights,
    /// Beta distribution parameter: successes + 1.
    pub alpha: f64,
    /// Beta distribution parameter: failures + 1.
    pub beta: f64,
}

impl Arm {
    pub fn new(name: String, weights: BlendWeights) -> Self {
        Self {
            name,
            weights,
            alpha: 1.0, // uniform prior
            beta: 1.0,
        }
    }

    /// Expected reward (mean of beta distribution).
    pub fn expected_reward(&self) -> f64 {
        self.alpha / (self.alpha + self.beta)
    }

    /// Record a success (user clicked a result).
    pub fn record_success(&mut self) {
        self.alpha += 1.0;
    }

    /// Record a failure (user didn't click any result).
    pub fn record_failure(&mut self) {
        self.beta += 1.0;
    }

    /// Decay parameters toward prior (prevents stale learning).
    pub fn decay(&mut self, factor: f64) {
        self.alpha = 1.0 + (self.alpha - 1.0) * factor;
        self.beta = 1.0 + (self.beta - 1.0) * factor;
    }

    /// Sample from the beta distribution using a simple approximation.
    ///
    /// For a proper implementation, use a real random beta sampler.
    /// This uses the mean ± jitter for simplicity.
    pub fn sample(&self, rng_seed: u64) -> f64 {
        let mean = self.expected_reward();
        let variance = (self.alpha * self.beta)
            / ((self.alpha + self.beta).powi(2) * (self.alpha + self.beta + 1.0));
        let std_dev = variance.sqrt();

        // Simple deterministic jitter based on seed
        let jitter = ((rng_seed % 1000) as f64 / 500.0 - 1.0) * std_dev;
        (mean + jitter).clamp(0.0, 1.0)
    }
}

/// The retrieval optimizer managing multiple arms.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalOptimizer {
    arms: Vec<Arm>,
    /// Total number of searches performed.
    pub total_searches: u64,
    /// Total number of successful feedback signals.
    pub total_successes: u64,
}

impl Default for RetrievalOptimizer {
    fn default() -> Self {
        Self::new()
    }
}

impl RetrievalOptimizer {
    /// Create a new optimizer with default arms.
    pub fn new() -> Self {
        Self {
            arms: vec![
                Arm::new(
                    "balanced".into(),
                    BlendWeights {
                        fulltext: 1.0,
                        semantic: 1.0,
                        recency: 0.0,
                    },
                ),
                Arm::new(
                    "fulltext_heavy".into(),
                    BlendWeights {
                        fulltext: 1.5,
                        semantic: 0.5,
                        recency: 0.0,
                    },
                ),
                Arm::new(
                    "semantic_heavy".into(),
                    BlendWeights {
                        fulltext: 0.5,
                        semantic: 1.5,
                        recency: 0.0,
                    },
                ),
                Arm::new(
                    "recency_boost".into(),
                    BlendWeights {
                        fulltext: 1.0,
                        semantic: 1.0,
                        recency: 0.005,
                    },
                ),
            ],
            total_searches: 0,
            total_successes: 0,
        }
    }

    /// Select the best arm using Thompson Sampling.
    ///
    /// Returns the arm index and its blend weights.
    pub fn select_arm(&self) -> (usize, BlendWeights) {
        // Use total_searches as a seed for deterministic-ish sampling
        let seed = self.total_searches;

        let mut best_idx = 0;
        let mut best_sample = f64::NEG_INFINITY;

        for (i, arm) in self.arms.iter().enumerate() {
            // Each arm gets a different seed offset
            let s = arm.sample(seed.wrapping_add(i as u64 * 7919));
            if s > best_sample {
                best_sample = s;
                best_idx = i;
            }
        }

        (best_idx, self.arms[best_idx].weights)
    }

    /// Record that a search was performed with the given arm.
    pub fn record_search(&mut self, arm_idx: usize) {
        self.total_searches += 1;
        // Failure by default — upgraded to success if feedback arrives
        if arm_idx < self.arms.len() {
            self.arms[arm_idx].record_failure();
        }
    }

    /// Record positive feedback for a search (user clicked a result).
    ///
    /// Reverses the default failure and records a success instead.
    pub fn record_feedback(&mut self, arm_idx: usize) {
        if arm_idx < self.arms.len() {
            self.total_successes += 1;
            // Undo the failure from record_search, add a success
            self.arms[arm_idx].beta -= 1.0;
            if self.arms[arm_idx].beta < 1.0 {
                self.arms[arm_idx].beta = 1.0;
            }
            self.arms[arm_idx].record_success();
        }
    }

    /// Decay all arms toward their priors.
    ///
    /// Call periodically (e.g., weekly) to prevent stale learning.
    pub fn decay_all(&mut self, factor: f64) {
        for arm in &mut self.arms {
            arm.decay(factor);
        }
    }

    /// Get stats for all arms.
    pub fn arm_stats(&self) -> Vec<ArmStats> {
        self.arms
            .iter()
            .map(|a| ArmStats {
                name: a.name.clone(),
                alpha: a.alpha,
                beta: a.beta,
                expected_reward: a.expected_reward(),
                weights: a.weights,
            })
            .collect()
    }

    /// Get the number of arms.
    pub fn num_arms(&self) -> usize {
        self.arms.len()
    }
}

/// Statistics for a single arm.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArmStats {
    pub name: String,
    pub alpha: f64,
    pub beta: f64,
    pub expected_reward: f64,
    pub weights: BlendWeights,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_optimizer_has_four_arms() {
        let opt = RetrievalOptimizer::new();
        assert_eq!(opt.num_arms(), 4);
        assert_eq!(opt.total_searches, 0);
        assert_eq!(opt.total_successes, 0);
    }

    #[test]
    fn select_arm_returns_valid_index() {
        let opt = RetrievalOptimizer::new();
        let (idx, weights) = opt.select_arm();
        assert!(idx < opt.num_arms());
        assert!(weights.fulltext > 0.0);
    }

    #[test]
    fn record_search_increments_count() {
        let mut opt = RetrievalOptimizer::new();
        opt.record_search(0);
        assert_eq!(opt.total_searches, 1);
    }

    #[test]
    fn record_feedback_increments_success() {
        let mut opt = RetrievalOptimizer::new();
        opt.record_search(0);
        opt.record_feedback(0);
        assert_eq!(opt.total_successes, 1);
        // Alpha should be > 1 (prior + success)
        assert!(opt.arm_stats()[0].alpha > 1.0);
    }

    #[test]
    fn feedback_without_search_is_safe() {
        let mut opt = RetrievalOptimizer::new();
        opt.record_feedback(0); // no prior search
        assert_eq!(opt.total_successes, 1);
    }

    #[test]
    fn feedback_invalid_arm_is_noop() {
        let mut opt = RetrievalOptimizer::new();
        opt.record_feedback(99); // out of bounds
        assert_eq!(opt.total_successes, 0);
    }

    #[test]
    fn decay_moves_toward_prior() {
        let mut opt = RetrievalOptimizer::new();
        // Build up some history
        for _ in 0..10 {
            opt.record_search(0);
            opt.record_feedback(0);
        }
        let before = opt.arm_stats()[0].alpha;
        assert!(before > 5.0);

        opt.decay_all(0.5);
        let after = opt.arm_stats()[0].alpha;
        assert!(after < before);
        assert!(after > 1.0); // doesn't go below prior
    }

    #[test]
    fn arm_expected_reward_is_bounded() {
        let opt = RetrievalOptimizer::new();
        for stat in opt.arm_stats() {
            assert!(stat.expected_reward >= 0.0);
            assert!(stat.expected_reward <= 1.0);
        }
    }

    #[test]
    fn successful_arm_gets_higher_reward() {
        let mut opt = RetrievalOptimizer::new();
        // Arm 0 gets lots of success
        for _ in 0..20 {
            opt.record_search(0);
            opt.record_feedback(0);
        }
        // Arm 1 gets no success
        for _ in 0..20 {
            opt.record_search(1);
        }

        let stats = opt.arm_stats();
        assert!(stats[0].expected_reward > stats[1].expected_reward);
    }

    #[test]
    fn blend_weights_default() {
        let w = BlendWeights::default();
        assert_eq!(w.fulltext, 1.0);
        assert_eq!(w.semantic, 1.0);
        assert_eq!(w.recency, 0.0);
    }

    #[test]
    fn arm_sample_is_bounded() {
        let arm = Arm::new("test".into(), BlendWeights::default());
        for seed in 0..100 {
            let s = arm.sample(seed);
            assert!(s >= 0.0 && s <= 1.0);
        }
    }

    #[test]
    fn optimizer_serde_roundtrip() {
        let mut opt = RetrievalOptimizer::new();
        opt.record_search(0);
        opt.record_feedback(0);

        let json = serde_json::to_string(&opt).unwrap();
        let restored: RetrievalOptimizer = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.total_searches, 1);
        assert_eq!(restored.total_successes, 1);
        assert_eq!(restored.num_arms(), 4);
    }
}
