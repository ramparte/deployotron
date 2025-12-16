//! Service factory functions
//!
//! Creates appropriate service implementations based on shadow mode configuration.
//! Enables easy switching between real and mock implementations.

use crate::services::{AwsOperations, GitOperations};
use crate::services::{AwsService, GitService};
use crate::shadow::{ShadowConfig, ShadowState, MockAwsService, MockGitService};
use std::sync::Arc;

/// Create AWS operations implementation (real or mock based on config)
///
/// # Arguments
/// * `region` - AWS region (e.g., "us-east-1")
/// * `config` - Shadow configuration determining real vs mock
/// * `state` - Shared shadow state (used only if mock)
///
/// # Returns
/// Arc-wrapped trait object for AWS operations
///
/// # Example
/// ```
/// use deployotron::services::factory::create_aws_operations;
/// use deployotron::shadow::{ShadowConfig, ShadowState};
/// use std::sync::Arc;
///
/// #[tokio::main]
/// async fn main() {
///     let config = ShadowConfig::from_env();
///     let state = Arc::new(ShadowState::new());
///     
///     let aws = create_aws_operations(None, &config, state).await.unwrap();
///     // Use aws for operations
/// }
/// ```
pub async fn create_aws_operations(
    region: Option<String>,
    config: &ShadowConfig,
    state: Arc<ShadowState>
) -> Result<Arc<dyn AwsOperations>, Box<dyn std::error::Error>> {
    if config.is_enabled() {
        Ok(Arc::new(MockAwsService::new(region, config.clone(), state)))
    } else {
        Ok(Arc::new(AwsService::new(region).await?))
    }
}

/// Create Git operations implementation (real or mock based on config)
///
/// # Arguments
/// * `config` - Shadow configuration determining real vs mock
/// * `state` - Shared shadow state (used only if mock)
///
/// # Returns
/// Arc-wrapped trait object for Git operations
///
/// # Example
/// ```
/// use deployotron::services::factory::create_git_operations;
/// use deployotron::shadow::{ShadowConfig, ShadowState};
/// use std::sync::Arc;
///
/// fn main() {
///     let config = ShadowConfig::from_env();
///     let state = Arc::new(ShadowState::new());
///     
///     let git = create_git_operations(&config, state);
///     // Use git for operations
/// }
/// ```
pub fn create_git_operations(
    config: &ShadowConfig,
    state: Arc<ShadowState>
) -> Arc<dyn GitOperations> {
    if config.is_enabled() {
        Arc::new(MockGitService::new(config.clone(), state))
    } else {
        Arc::new(GitService::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_create_mock_aws_operations() {
        let config = ShadowConfig {
            enabled: true,
            failure_rate: 0.0,
            simulate_delays: false,
        };
        let state = Arc::new(ShadowState::new());
        
        let aws = create_aws_operations(Some("us-east-1".into()), &config, state).await;
        assert!(aws.is_ok());
    }
    
    #[tokio::test]
    async fn test_create_real_aws_operations_disabled() {
        let config = ShadowConfig {
            enabled: false,
            failure_rate: 0.0,
            simulate_delays: false,
        };
        let state = Arc::new(ShadowState::new());
        
        // This will try to create real AWS service, which may fail without credentials
        // That's expected behavior
        let _aws = create_aws_operations(Some("us-east-1".into()), &config, state).await;
    }
    
    #[test]
    fn test_create_mock_git_operations() {
        let config = ShadowConfig {
            enabled: true,
            failure_rate: 0.0,
            simulate_delays: false,
        };
        let state = Arc::new(ShadowState::new());
        
        let git = create_git_operations(&config, state);
        // Successfully created
        assert!(Arc::strong_count(&git) == 1);
    }
    
    #[test]
    fn test_create_real_git_operations() {
        let config = ShadowConfig {
            enabled: false,
            failure_rate: 0.0,
            simulate_delays: false,
        };
        let state = Arc::new(ShadowState::new());
        
        let git = create_git_operations(&config, state);
        assert!(Arc::strong_count(&git) == 1);
    }
}
