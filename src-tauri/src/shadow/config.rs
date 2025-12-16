//! Shadow mode configuration
//!
//! Controls shadow world behavior through environment variables:
//! - DEPLOYOTRON_SHADOW_MODE: Enable shadow mode (any value)
//! - DEPLOYOTRON_SHADOW_FAILURE_RATE: Failure injection rate (0.0-1.0, default: 0.0)

use std::env;

/// Configuration for shadow world testing
#[derive(Debug, Clone)]
pub struct ShadowConfig {
    /// Whether shadow mode is enabled
    pub enabled: bool,
    /// Probability of simulating failures (0.0 = never, 1.0 = always)
    pub failure_rate: f64,
    /// Whether to simulate realistic delays
    pub simulate_delays: bool,
}

impl ShadowConfig {
    /// Load configuration from environment variables
    ///
    /// # Environment Variables
    /// - `DEPLOYOTRON_SHADOW_MODE`: If present, enables shadow mode
    /// - `DEPLOYOTRON_SHADOW_FAILURE_RATE`: Float between 0.0 and 1.0 (default: 0.0)
    ///
    /// # Example
    /// ```bash
    /// export DEPLOYOTRON_SHADOW_MODE=1
    /// export DEPLOYOTRON_SHADOW_FAILURE_RATE=0.1
    /// ```
    pub fn from_env() -> Self {
        let enabled = env::var("DEPLOYOTRON_SHADOW_MODE").is_ok();
        
        let failure_rate = env::var("DEPLOYOTRON_SHADOW_FAILURE_RATE")
            .ok()
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0)
            .clamp(0.0, 1.0);
        
        Self {
            enabled,
            failure_rate,
            simulate_delays: true,
        }
    }
    
    /// Check if shadow mode is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
    
    /// Determine if current operation should fail (based on failure_rate)
    ///
    /// Uses random number generation to decide based on configured failure rate.
    pub fn should_fail(&self) -> bool {
        if self.failure_rate <= 0.0 {
            return false;
        }
        if self.failure_rate >= 1.0 {
            return true;
        }
        
        use rand::Rng;
        let mut rng = rand::thread_rng();
        rng.gen::<f64>() < self.failure_rate
    }
}

impl Default for ShadowConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            failure_rate: 0.0,
            simulate_delays: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_default_config() {
        let config = ShadowConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.failure_rate, 0.0);
        assert!(config.simulate_delays);
    }
    
    #[test]
    fn test_should_fail_never() {
        let config = ShadowConfig {
            enabled: true,
            failure_rate: 0.0,
            simulate_delays: false,
        };
        assert!(!config.should_fail());
    }
    
    #[test]
    fn test_should_fail_always() {
        let config = ShadowConfig {
            enabled: true,
            failure_rate: 1.0,
            simulate_delays: false,
        };
        assert!(config.should_fail());
    }
}
