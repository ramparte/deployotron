//! Service layer for Deployotron
//!
//! This module provides the service layer components:
//! - GitService: Repository cloning, framework detection, commit information
//! - AwsService: AWS ECS/ECR deployments, CloudWatch logs, health monitoring
//! - TerraformService: Infrastructure-as-Code generation for ECS deployments
//! - ClaudeService: AI-powered deployment assistance and troubleshooting

pub mod git_service;
pub mod aws_service;
pub mod terraform_service;
pub mod claude_service;

// Trait definitions for testability
pub mod aws_trait;
pub mod git_trait;
pub mod factory;

pub use git_service::{GitService, GitServiceError};
pub use git_trait::{GitOperations, CommitInfo};
pub use aws_service::{AwsService, AwsServiceError, EcsDeploymentConfig, ServiceHealth};
pub use aws_trait::AwsOperations;
pub use terraform_service::{TerraformService, TerraformServiceError, TerraformConfig};
pub use claude_service::{ClaudeService, ClaudeServiceError, DeploymentContext, ClaudeResponse};
pub use factory::{create_aws_operations, create_git_operations};
