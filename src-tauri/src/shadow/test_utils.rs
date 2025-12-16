//! Testing utilities for shadow world
//!
//! Provides helper functions and test environments for writing tests with shadow mode.

use crate::shadow::{ShadowConfig, ShadowState};
use crate::services::{AwsOperations, GitOperations};
use crate::shadow::{MockAwsService, MockGitService};
use std::sync::Arc;

/// Create test configuration with shadow mode enabled
pub fn test_config() -> ShadowConfig {
    ShadowConfig {
        enabled: true,
        failure_rate: 0.0,
        simulate_delays: false, // Faster tests
    }
}

/// Create test configuration with failure injection
pub fn test_config_with_failures(rate: f64) -> ShadowConfig {
    ShadowConfig {
        enabled: true,
        failure_rate: rate,
        simulate_delays: false,
    }
}

/// Create fresh shadow state for testing
pub fn test_state() -> Arc<ShadowState> {
    Arc::new(ShadowState::new())
}

/// Complete test environment with all mock services
pub struct TestEnvironment {
    pub config: ShadowConfig,
    pub state: Arc<ShadowState>,
    pub aws_service: Arc<dyn AwsOperations>,
    pub git_service: Arc<dyn GitOperations>,
}

impl TestEnvironment {
    /// Create a new test environment
    ///
    /// # Example
    /// ```
    /// use deployotron::shadow::test_utils::TestEnvironment;
    ///
    /// #[tokio::test]
    /// async fn test_deployment() {
    ///     let env = TestEnvironment::new().await;
    ///     // Use env.aws_service and env.git_service for testing
    /// }
    /// ```
    pub async fn new() -> Self {
        let config = test_config();
        let state = test_state();
        
        let aws_service: Arc<dyn AwsOperations> = Arc::new(
            MockAwsService::new(Some("us-east-1".into()), config.clone(), state.clone())
        );
        let git_service: Arc<dyn GitOperations> = Arc::new(
            MockGitService::new(config.clone(), state.clone())
        );
        
        Self {
            config,
            state,
            aws_service,
            git_service,
        }
    }
    
    /// Create test environment with failure injection
    pub async fn with_failures(failure_rate: f64) -> Self {
        let config = test_config_with_failures(failure_rate);
        let state = test_state();
        
        let aws_service: Arc<dyn AwsOperations> = Arc::new(
            MockAwsService::new(Some("us-east-1".into()), config.clone(), state.clone())
        );
        let git_service: Arc<dyn GitOperations> = Arc::new(
            MockGitService::new(config.clone(), state.clone())
        );
        
        Self {
            config,
            state,
            aws_service,
            git_service,
        }
    }
    
    /// Reset all state (useful between tests)
    pub fn reset(&self) {
        self.state.reset();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_config_creation() {
        let config = test_config();
        assert!(config.enabled);
        assert_eq!(config.failure_rate, 0.0);
        assert!(!config.simulate_delays);
    }
    
    #[test]
    fn test_config_with_failures() {
        let config = test_config_with_failures(0.5);
        assert!(config.enabled);
        assert_eq!(config.failure_rate, 0.5);
    }
    
    #[tokio::test]
    async fn test_environment_creation() {
        let env = TestEnvironment::new().await;
        assert!(env.config.enabled);
        assert!(!env.config.simulate_delays);
    }
    
    #[tokio::test]
    async fn test_environment_reset() {
        let env = TestEnvironment::new().await;
        
        // Add some state
        env.state.add_ecr_repository("test".to_string(), "uri".to_string());
        assert!(env.state.get_ecr_repository("test").is_some());
        
        // Reset
        env.reset();
        assert!(env.state.get_ecr_repository("test").is_none());
    }
}
