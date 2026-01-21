//! Variable difficulty adjustment engine for mining pool.
//!
//! Automatically adjusts miner difficulty based on share submission rate.
//!
//! **Phase 8 Feature**: This module is only available with the `pool` feature.
//!
//! # Example
//! ```rust,ignore
//! # use bitquan_node::VarDiff;
//! let mut vardiff = VarDiff::new(30.0, 0.05);
//! // Adjust difficulty based on miner shares...
//! ```

/// Variable difficulty controller.
#[derive(Clone, Debug)]
pub struct VarDiff {
    /// Target share submission time in seconds.
    pub target_time: f64,
    /// Adjustment rate (0.0 to 1.0, typically 0.05).
    pub adjust_rate: f64,
}

impl VarDiff {
    /// Create a new vardiff controller.
    ///
    /// # Arguments
    /// * `target_time` - Desired interval between shares (seconds)
    /// * `adjust_rate` - How aggressively to adjust (0.0 = no change, 1.0 = immediate)
    pub fn new(target_time: f64, adjust_rate: f64) -> Self {
        Self {
            target_time,
            adjust_rate,
        }
    }

    /// Adjust difficulty based on actual share submission time.
    ///
    /// Uses a simple proportional controller to adjust difficulty.
    ///
    /// # Arguments
    /// * `actual_time` - Time since last share submission (seconds)
    /// * `current_diff` - Current difficulty
    ///
    /// # Returns
    /// New difficulty, clamped to reasonable bounds [0.01, 10000.0]
    pub fn adjust(&self, actual_time: f64, current_diff: f64) -> f64 {
        // Calculate ratio: actual / target
        // If ratio > 1.0, miner is too slow, decrease difficulty
        // If ratio < 1.0, miner is too fast, increase difficulty
        let ratio = actual_time / self.target_time;

        // Apply adjustment: new_diff = current_diff * (1 + rate * (1 - ratio))
        // This makes diff inversely proportional to share rate
        let delta = 1.0 - ratio;
        let new_diff = current_diff * (1.0 + self.adjust_rate * delta);

        // Clamp to reasonable bounds
        new_diff.clamp(0.01, 10000.0)
    }

    /// Check if adjustment should be triggered based on share count.
    ///
    /// Typically adjust after every N shares (e.g., 8-16).
    pub fn should_adjust(&self, shares_since_adjust: u64) -> bool {
        shares_since_adjust >= 8
    }
}

impl Default for VarDiff {
    fn default() -> Self {
        Self::new(15.0, 0.05)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vardiff_increase() {
        let vd = VarDiff::new(15.0, 0.05);

        // Miner submitting too fast (5s instead of 15s)
        // Difficulty should increase
        let new_diff = vd.adjust(5.0, 1.0);
        assert!(new_diff > 1.0, "Difficulty should increase for fast miner");
    }

    #[test]
    fn test_vardiff_decrease() {
        let vd = VarDiff::new(15.0, 0.05);

        // Miner submitting too slow (30s instead of 15s)
        // Difficulty should decrease
        let new_diff = vd.adjust(30.0, 1.0);
        assert!(new_diff < 1.0, "Difficulty should decrease for slow miner");
    }

    #[test]
    fn test_vardiff_bounds() {
        let vd = VarDiff::new(15.0, 0.5); // High adjustment rate

        // Extreme fast case
        let new_diff = vd.adjust(0.1, 1.0);
        assert!((0.01..=10000.0).contains(&new_diff));

        // Extreme slow case
        let new_diff = vd.adjust(1000.0, 1.0);
        assert!((0.01..=10000.0).contains(&new_diff));
    }

    #[test]
    fn test_should_adjust() {
        let vd = VarDiff::default();

        assert!(!vd.should_adjust(0));
        assert!(!vd.should_adjust(7));
        assert!(vd.should_adjust(8));
        assert!(vd.should_adjust(16));
    }
}
