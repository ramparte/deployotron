//! Shadow world testing system
//!
//! Provides mock implementations of external services (AWS, Git, Docker) for testing
//! without requiring real infrastructure. Enable with DEPLOYOTRON_SHADOW_MODE environment variable.

pub mod config;
pub mod state;
pub mod aws_mock;
pub mod git_mock;

#[cfg(test)]
pub mod test_utils;

pub use config::ShadowConfig;
pub use state::{ShadowState, ServiceStatus};
pub use aws_mock::MockAwsService;
pub use git_mock::MockGitService;
